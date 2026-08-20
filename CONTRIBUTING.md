# Contributing to Raven Notch

Thank you for considering contributing! This guide helps you get started.

## Quick Start

1. **Fork** the repo → `git clone https://github.com/<your-username>/Raven-Island.git`
2. **Branch**: `git checkout -b feat/your-feature-name`
3. **Code**: Make changes, follow style guides below
4. **Test**: `cargo test && cargo clippy && cargo fmt --check`
5. **Commit**: Clear, atomic commits with conventional messages
6. **Push**: `git push origin feat/your-feature-name`
7. **PR**: Open PR against `main` with description

## Development Setup

```powershell
# Prerequisites
rustup default stable-msvc
# Visual Studio Build Tools with Windows 10 SDK

# Build & run
cd raven-native
cargo run              # Debug
cargo build --release  # Optimized
```

## Code Style

- **Rust**: `cargo fmt` (rustfmt), `cargo clippy` (lints) — must pass
- **Commits**: Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`)
- **Slint**: 4-space indent, trailing commas in multi-line structs

## Project Structure

```
raven-native/src/
├── main.rs       # App entry, UI, timers, widget lifecycle
├── window.rs     # Win32: overlay, hit-testing, hotkeys, tray, appbar
├── services.rs   # Media, notifications, capture, stats, calendar, shelf
├── renderer.rs   # Hit-test + scene, D2D→GDI fallback
├── graphics.rs   # Direct2D/DWrite backend
├── settings.rs   # Schema + load/save/validate
├── motion.rs     # Spring-morph physics
├── license.rs    # Premium/trial API
└── widgets.rs    # Volume/brightness, tab enums
```

## Adding a Feature

1. **Widget**: Add to `ui/`, export in `ui/index.slint`, settings in `settings.rs`, lifecycle in `main.rs`
2. **Service**: Add to `ServiceRegistry` in `services.rs`, implement `read()`/`refresh_*()`, update `RuntimeSnapshot`
3. **Hotkey**: Add to `ShortcutsSettings`, parse in `window.rs::parse_shortcut`, register in `register_raven_hotkeys`
4. **Setting**: Add field in `settings.rs`, UI in `settings.slint`, sync in `main.rs` callbacks

## Testing

- Unit tests: `cargo test` (currently `monitor_math` only)
- Manual: run app, test feature end-to-end
- No CI yet — ensure `cargo check` passes

## Pull Request Checklist

- [ ] `cargo fmt` clean
- [ ] `cargo clippy` clean (no new warnings)
- [ ] `cargo test` passes
- [ ] Commits are atomic and descriptive
- [ ] README/docs updated if user-facing change
- [ ] No secrets, personal paths, or debug code

## Good First Issues

- Linux/macOS stub (Windows-only currently)
- More share providers (AirDrop, Syncthing, etc.)
- Plugin system for custom widgets
- Unit tests for `services.rs` / `settings.rs` / `window.rs`
- Accessibility improvements (screen reader, high contrast)

## Questions?

Open a Discussion or issue — happy to help!