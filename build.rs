fn main() {
    // Embed the Windows application manifest. It does two things:
    //   1. Requests elevation (requireAdministrator) — PresentMon needs an
    //      elevated ETW session to read present telemetry.
    //   2. Declares per-monitor (v2) DPI awareness so the overlay positions
    //      and renders crisply on mixed-DPI multi-monitor setups.
    #[cfg(windows)]
    {
        embed_resource::compile("assets/framewise.rc", embed_resource::NONE);
    }
}
