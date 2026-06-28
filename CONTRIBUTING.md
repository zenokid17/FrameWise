# Contributing to FrameWise

Thanks for your interest! FrameWise is a safety-first performance toolkit, so a
few rules are stricter than usual. Please read the principles before opening a PR.

## Non-negotiable principles

1. **No injection or hooking.** Anything that reads game performance must do so
   passively (PresentMon / ETW / OS counters). PRs that inject DLLs, hook
   graphics APIs, or read another process's memory will be declined.
2. **Reversible, explained changes only.** Any optimization that modifies system
   state must:
   - explain what it changes and the expected impact,
   - require explicit per-action user consent (never automatic),
   - record enough to fully revert it (written to `framewise-changes.jsonl`),
   - have a corresponding revert path.
3. **No registry hacks, no RAM cleaners, no silent process killing.** Only
   documented, vendor-supported settings with measurable effects.
4. **Graceful degradation.** Features unavailable on the user's Windows build
   must be detected and disabled, never crash.
5. **Stay lightweight.** Keep the idle footprint near the ~30 MB RAM target and
   avoid adding background services or heavyweight dependencies without
   discussion.

## Development setup

- Install Rust via [rustup](https://rustup.rs/) with the MSVC toolchain.
- `cargo build` / `cargo run` (run elevated — see README).
- Before pushing:
  ```sh
  cargo fmt --all
  cargo clippy --all-targets -- -D warnings
  cargo test --all
  ```
  CI runs the same checks on `windows-latest` and must be green.

## Commit / PR conventions

- Conventional-commits style is encouraged (`feat:`, `fix:`, `docs:`, …).
- One logical change per PR. The optimization *apply/revert* work especially
  should land as small, individually reviewable PRs (one finding at a time).
- Update `CHANGELOG.md` under "Unreleased".
- The project follows [Semantic Versioning](https://semver.org/).

## Adding a new optimization check

A new audit finding should live in `src/audit/` and implement the finding model
in `src/audit/mod.rs`. A read-only **detector** can be merged on its own. The
matching **apply/revert** logic is a separate PR and must satisfy principle #2.
Document the check in `docs/optimizations.md`.

## Reporting bugs

Include your Windows edition + build (`winver`), whether PresentMon was present,
and the relevant lines from `framewise.log`.
