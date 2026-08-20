# Raven Native

This is the native-first migration path for Raven. The current Tauri/WebView app remains the
reference implementation while this binary grows toward feature parity.

Implemented native migration surface:

- load the existing Raven settings file from `RavenIsland/settings.json`
- create a transparent, borderless, always-on-top native Windows overlay
- render the closed Raven notch without WebView2
- preserve the current open/close motion constants and states in Rust
- provide typed service/event seams in place of Tauri IPC
- native tabs for home, media, clock, drop, capture, calendar, notifications, stats, and settings
- native media controls, clock/timer/stopwatch, shelf persistence, screenshots, ICS calendar,
  notification status, battery/power status, caffeine, hotkeys, and tray lifecycle

Useful commands:

```powershell
cargo check
cargo build --release
target\release\raven-native.exe
```

The final migration target is Rust + Win32 + DirectComposition + DirectX 11 + Direct2D +
DirectWrite, with render-on-demand behavior and no always-running WebView UI.
