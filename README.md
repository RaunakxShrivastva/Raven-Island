# Raven Notch

<p align="center">
  <img src="website/logo.png" alt="Raven Notch" width="200"/>
</p>

A native Windows "Dynamic Island" — a floating, always-on-top notch at the top-center of your screen that expands into a full control center.

<p align="center">
  <img src="website/demo.mp4" alt="Demo" width="600"/>
</p>

## Features

- **Media Control** — Play/pause/next/prev, album art, lyrics (synced via LRCLIB), source switching
- **Clock & World Clocks** — Timer, stopwatch, focus sessions, 8 world timezones (NY, London, Tokyo, Delhi, Sydney, Paris, Dubai, Singapore)
- **Calendar** — Google Calendar (OAuth) + ICS URL support, mini-calendar picker
- **System Stats** — CPU/RAM/GPU with sparkline history, top processes, battery + power status
- **Drop Shelf** — Drag files to the notch; share via LocalSend, Quick Share, or KDE Connect
- **Capture** — Screenshots (full/region) saved to Pictures\Raven Captures
- **Notifications** — Windows toast listener (last 5, with app icons)
- **Widgets** — Desktop widgets: clock, progress (year/day/month), notes, todo, quotes, picture, video, battery ring, focus score, streak, apps container
- **Customization** — Shape, opacity, colors, border radius, auto-hide, hover delays, per-tab visibility
- **Hotkeys** — 23 configurable shortcuts + "magic chord" (Ctrl+Alt+Shift+Win)
- **Tray** — Right-click for Settings / Restart / Quit
- **Licensing** — Trial / premium via ravennotch.me API

## Requirements

- Windows 10/11 (x64)
- Rust toolchain (MSVC) — `rustup default stable-msvc`
- Visual Studio Build Tools with Windows 10 SDK

## Build & Run

```powershell
# Clone
git clone https://github.com/RaunakxShrivastva/Raven-Island.git
cd Raven-Island/raven-native

# Build release
cargo build --release

# Run
.\target\release\Raven-Notch.exe
```

The app creates `%APPDATA%\RavenIsland\settings.json` on first run.

## Configuration

| Setting | Location |
|---------|----------|
| App settings | `%APPDATA%\RavenIsland\settings.json` |
| Logs | `%LOCALAPPDATA%\Raven Notch\logs\compatibility.log` |
| Captures | `%USERPROFILE%\Pictures\Raven Captures\` |

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `RAVEN_GOOGLE_CLIENT_SECRET` | Google Calendar OAuth client secret (required for Calendar sync) |

## Architecture

```
raven-native/
├── src/
│   ├── main.rs          # Monolithic app entry (UI, widgets, threads, timers)
│   ├── window.rs        # Win32 overlay, hit-testing, hotkeys, tray, appbar
│   ├── services.rs      # Media, notifications, capture, stats, calendar, shelf, caffeine
│   ├── renderer.rs      # Hit-test + scene building, D2D→GDI fallback
│   ├── graphics.rs      # Direct2D/DWrite backend
│   ├── settings.rs      # Settings schema + load/save/validate
│   ├── motion.rs        # Notch spring-morph physics
│   ├── license.rs       # Premium/trial licensing via API
│   └── widgets.rs       # Volume/brightness helpers, tab enums
├── ui/                  # Slint UI (pill.slint, settings.slint, panels/, widgets/)
└── build.rs             # Compiles Slint + embeds icon/manifest
```

## License

MIT — see [LICENSE](LICENSE).

## Project Status

Active development. Migrating from Tauri/WebView → native Rust + Win32 + DirectComposition.