//! Native Win32 overlay window.
//!
//! It's a layered, click-through, top-most tool window (no taskbar/Alt-Tab
//! entry, never steals focus). Drawing is double-buffered GDI; window opacity is
//! applied with `SetLayeredWindowAttributes`. A timer drives the refresh: each
//! tick we detect the target monitor, recompute the stats text, size/position
//! the window on that monitor, and repaint.

pub mod monitor;
pub mod render;

use std::time::Duration;

use anyhow::{anyhow, Context};
use windows::core::w;
use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontIndirectW,
    CreateRoundRectRgn, CreateSolidBrush, DeleteDC, DeleteObject, EndPaint, FillRect, GetDC,
    GetTextExtentPoint32W, InvalidateRect, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
    SetWindowRgn, TextOutW, HFONT, HGDIOBJ, LOGFONTW, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW,
    PostQuitMessage, RegisterClassW, SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, GWLP_USERDATA, HWND_TOPMOST, LWA_ALPHA, SWP_NOACTIVATE, SW_HIDE,
    SW_SHOWNOACTIVATE, WM_DESTROY, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::config::{Config, Position};
use crate::stats::Telemetry;

const TIMER_ID: usize = 1;
/// Window over which FPS is averaged for the on-screen number.
const FPS_RECENT: Duration = Duration::from_millis(1000);
/// If the game hasn't presented within this long, FPS is shown as unknown.
const FPS_STALE: Duration = Duration::from_millis(1500);
/// Background colour of the overlay panel.
const BG: (u8, u8, u8) = (18, 18, 24);

/// Heap state owned by the window (pointer stored in GWLP_USERDATA).
struct OverlayState {
    config: Config,
    tele: Telemetry,
    font: HFONT,
    dpi: u32,
    visible: bool,
    lines: Vec<render::Line>,
    pad: i32,
    line_height: i32,
    /// X offset where value text starts (labels start at `pad`).
    value_x: i32,
}

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut OverlayState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayState;
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

unsafe fn create_font(font_size: i32, dpi: u32) -> HFONT {
    // Negative height = em (character) height in device pixels.
    let height = -(font_size * dpi as i32 / 72);
    let mut lf = LOGFONTW {
        lfHeight: height,
        lfWeight: 600, // semi-bold (FW_NORMAL = 400, FW_BOLD = 700)
        ..Default::default()
    };
    for (i, c) in "Consolas".encode_utf16().enumerate() {
        if i < lf.lfFaceName.len() - 1 {
            lf.lfFaceName[i] = c;
        }
    }
    CreateFontIndirectW(&lf)
}

/// Public handle used by the rest of the app to control the overlay.
pub struct Overlay {
    hwnd: HWND,
}

impl Overlay {
    pub fn create(config: Config, tele: Telemetry) -> anyhow::Result<Overlay> {
        unsafe {
            let hinstance = GetModuleHandleW(None).context("GetModuleHandleW")?;

            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: hinstance.into(),
                lpszClassName: w!("FrameWiseOverlayClass"),
                ..Default::default()
            };
            // RegisterClassW returns 0 on failure; ignore "already registered".
            let _atom = RegisterClassW(&wc);

            let ex_style = WS_EX_LAYERED
                | WS_EX_TRANSPARENT
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE;

            let hwnd = CreateWindowExW(
                ex_style,
                w!("FrameWiseOverlayClass"),
                w!("FrameWise Overlay"),
                WS_POPUP,
                0,
                0,
                10,
                10,
                None,
                None,
                hinstance,
                None,
            )
            .context("CreateWindowExW")?;
            if hwnd.0.is_null() {
                return Err(anyhow!("CreateWindowExW returned null"));
            }

            let dpi = GetDpiForWindow(hwnd).max(96);
            let font = create_font(config.overlay.font_size, dpi);
            let visible = config.overlay.visible_on_start;
            let refresh_ms = 1000 / config.overlay.refresh_hz.max(1);
            let opacity = config.overlay.opacity;

            let state = Box::new(OverlayState {
                config,
                tele,
                font,
                dpi,
                visible,
                lines: Vec::new(),
                pad: 8,
                line_height: 0,
                value_x: 8,
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            // Apply opacity and start the refresh timer.
            let alpha = (opacity.clamp(0.10, 1.0) * 255.0) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
            SetTimer(hwnd, TIMER_ID, refresh_ms, None);

            if visible {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }

            Ok(Overlay { hwnd })
        }
    }

    pub fn is_visible(&self) -> bool {
        unsafe { get_state(self.hwnd).map(|s| s.visible).unwrap_or(false) }
    }

    pub fn set_visible(&self, visible: bool) {
        unsafe {
            if let Some(state) = get_state(self.hwnd) {
                state.visible = visible;
            }
            let _ = ShowWindow(self.hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }

    pub fn toggle(&self) {
        self.set_visible(!self.is_visible());
    }

    pub fn destroy(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe fn on_timer(hwnd: HWND, state: &mut OverlayState) {
    if !state.visible {
        return;
    }

    // Keep font in sync with the window's current DPI (multi-monitor moves).
    let dpi = GetDpiForWindow(hwnd).max(96);
    if dpi != state.dpi {
        let old = state.font;
        state.font = create_font(state.config.overlay.font_size, dpi);
        let _ = DeleteObject(HGDIOBJ(old.0));
        state.dpi = dpi;
    }
    let scale = dpi as f32 / 96.0;
    state.pad = (8.0 * scale) as i32;
    let margin = (state.config.overlay.margin as f32 * scale) as i32;

    // Where is the game, and on which monitor?
    let target = monitor::detect();
    state.tele.set_target_pid(target.pid);

    // Combine slow system stats with freshly-computed FPS.
    let mut snap = state.tele.snapshot();
    if let Some((fps, ft, low)) = state.tele.fps_summary(FPS_RECENT, FPS_STALE) {
        snap.fps = Some(fps);
        snap.frame_time_ms = Some(ft);
        snap.low_1_percent = Some(low);
    }

    let lines = render::build_lines(
        &state.config.overlay.stats,
        &snap,
        state.tele.fps_available(),
    );
    let (w, h, row_h, value_x) = measure(hwnd, state.font, &lines, state.pad);
    state.lines = lines;
    state.line_height = row_h;
    state.value_x = value_x;

    let (x, y) = position_for(
        state.config.overlay.position,
        target.monitor_rect,
        w,
        h,
        margin,
    );
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);

    // Rounded corners: clip the window to a round rectangle (the system takes
    // ownership of the region, so we don't free it).
    let radius = ((10.0 * scale) as i32).max(2);
    let rgn = CreateRoundRectRgn(0, 0, w + 1, h + 1, radius, radius);
    let _ = SetWindowRgn(hwnd, rgn, BOOL(1));

    let _ = InvalidateRect(hwnd, None, BOOL(1));
}

/// Measure the rows and compute panel size. Returns `(width, height, row_height,
/// value_x)` where `value_x` is the x offset at which value text is drawn (the
/// label column ends just before it).
unsafe fn measure(
    hwnd: HWND,
    font: HFONT,
    lines: &[render::Line],
    pad: i32,
) -> (i32, i32, i32, i32) {
    let screen = GetDC(hwnd);
    let memdc = CreateCompatibleDC(screen);
    let old = SelectObject(memdc, HGDIOBJ(font.0));

    let text_width = |s: &str| -> (i32, i32) {
        let wide: Vec<u16> = s.encode_utf16().collect();
        let mut sz = SIZE::default();
        if GetTextExtentPoint32W(memdc, &wide, &mut sz).as_bool() {
            (sz.cx, sz.cy)
        } else {
            (0, 0)
        }
    };

    let mut label_w = 0i32;
    let mut value_w = 0i32;
    let mut line_h = 0i32;
    for line in lines {
        let (lw, lh) = text_width(&line.label);
        let (vw, vh) = text_width(&line.value);
        label_w = label_w.max(lw);
        value_w = value_w.max(vw);
        line_h = line_h.max(lh.max(vh));
    }

    SelectObject(memdc, old);
    let _ = DeleteDC(memdc);
    ReleaseDC(hwnd, screen);

    let gap = (pad as f32 * 1.6) as i32; // space between label and value columns
    let value_x = pad + label_w + gap;
    let row_h = line_h + (line_h as f32 * 0.22) as i32;
    let w = (value_x + value_w + pad).max(1);
    let h = (row_h * lines.len() as i32 + pad * 2).max(1);
    (w, h, row_h, value_x)
}

fn position_for(pos: Position, mon: RECT, w: i32, h: i32, margin: i32) -> (i32, i32) {
    match pos {
        Position::TopLeft => (mon.left + margin, mon.top + margin),
        Position::TopRight => (mon.right - w - margin, mon.top + margin),
        Position::BottomLeft => (mon.left + margin, mon.bottom - h - margin),
        Position::BottomRight => (mon.right - w - margin, mon.bottom - h - margin),
    }
}

unsafe fn on_paint(hwnd: HWND, state: &OverlayState) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);

    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;

    // Off-screen buffer to avoid flicker.
    let memdc = CreateCompatibleDC(hdc);
    let hbm = CreateCompatibleBitmap(hdc, w, h);
    let old_bm = SelectObject(memdc, HGDIOBJ(hbm.0));

    let bg = CreateSolidBrush(rgb(BG.0, BG.1, BG.2));
    FillRect(memdc, &rc, bg);
    SetBkMode(memdc, TRANSPARENT);
    let old_font = SelectObject(memdc, HGDIOBJ(state.font.0));

    let (lr, lg, lb) = render::LABEL_COLOR;
    let mut y = state.pad;
    for line in &state.lines {
        // Muted label.
        SetTextColor(memdc, rgb(lr, lg, lb));
        let label: Vec<u16> = line.label.encode_utf16().collect();
        let _ = TextOutW(memdc, state.pad, y, &label);

        // Accent-coloured value.
        let (r, g, b) = render::color_for(line.kind);
        SetTextColor(memdc, rgb(r, g, b));
        let value: Vec<u16> = line.value.encode_utf16().collect();
        let _ = TextOutW(memdc, state.value_x, y, &value);

        y += state.line_height;
    }

    let _ = BitBlt(hdc, 0, 0, w, h, memdc, 0, 0, SRCCOPY);

    SelectObject(memdc, old_font);
    SelectObject(memdc, old_bm);
    let _ = DeleteObject(HGDIOBJ(hbm.0));
    let _ = DeleteObject(HGDIOBJ(bg.0));
    let _ = DeleteDC(memdc);
    let _ = EndPaint(hwnd, &ps);
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            if let Some(state) = get_state(hwnd) {
                on_timer(hwnd, state);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            if let Some(state) = get_state(hwnd) {
                on_paint(hwnd, state);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // Reclaim and drop the heap state, delete the font, then quit.
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayState;
            if !ptr.is_null() {
                let state = Box::from_raw(ptr);
                let _ = DeleteObject(HGDIOBJ(state.font.0));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
