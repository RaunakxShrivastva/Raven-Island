use crate::graphics::{DirectCompositionBackend, RenderBackend, RenderBackendKind, RenderScene};
use crate::motion::{MotionState, NotchGeometry, NotchPhase};
use crate::services::RuntimeSnapshot;
use crate::settings::{settings_path, RavenSettings};
use crate::widgets::{NativeAction, NativeTab, WidgetModel};
use std::path::Path;
use windows::Win32::Foundation::{COLORREF, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, RoundRect, SelectObject,
    SetBkMode, SetTextColor, TextOutW, HGDIOBJ, PAINTSTRUCT, TRANSPARENT, ValidateRect,
};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

pub struct NativeRenderer {
    settings: RavenSettings,
    motion: MotionState,
    widgets: WidgetModel,
    dcomp_backend: DirectCompositionBackend,
    snapshot: RuntimeSnapshot,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ClickOutcome {
    pub animate: bool,
    pub action: Option<NativeAction>,
}

impl NativeRenderer {
    pub fn new(settings: RavenSettings) -> Self {
        let width = settings.appearance.idle_width.max(10.0);
        let height = settings.appearance.idle_height.max(10.0);
        let radius = settings.appearance.border_radius;
        Self {
            settings,
            motion: MotionState::closed(width, height, radius),
            widgets: WidgetModel::new(),
            dcomp_backend: DirectCompositionBackend::new(),
            snapshot: RuntimeSnapshot::now(),
        }
    }

    pub fn geometry(&self) -> NotchGeometry {
        self.motion.geometry
    }

    pub fn phase(&self) -> NotchPhase {
        self.motion.phase
    }

    pub fn content_opacity(&self) -> f32 {
        self.motion.content_opacity
    }

    pub fn toggle(&mut self) {
        self.motion.begin_toggle();
    }

    pub fn tick(&mut self) -> bool {
        self.motion.advance_frame()
    }

    pub fn handle_click(&mut self, x: i32, y: i32) -> ClickOutcome {
        if !self.motion.is_openish() {
            self.toggle();
            return ClickOutcome {
                animate: true,
                action: None,
            };
        }

        let geometry = self.motion.geometry;
        if y <= 34 && x > geometry.width.round() as i32 - 80 {
            self.toggle();
            return ClickOutcome {
                animate: true,
                action: None,
            };
        }

        if self.widgets.current_tab == NativeTab::Media && (224..=248).contains(&y) {
            let action = if (24..=112).contains(&x) {
                Some(NativeAction::MediaPrevious)
            } else if (122..=250).contains(&x) {
                Some(NativeAction::MediaPlayPause)
            } else if (260..=348).contains(&x) {
                Some(NativeAction::MediaNext)
            } else {
                None
            };
            if action.is_some() {
                return ClickOutcome {
                    animate: false,
                    action,
                };
            }
        }

        if self.widgets.current_tab == NativeTab::Clock {
            let action = if (190..=214).contains(&y) {
                if (24..=220).contains(&x) {
                    Some(NativeAction::TimerToggle)
                } else if (230..=360).contains(&x) {
                    Some(NativeAction::TimerReset)
                } else {
                    None
                }
            } else if (242..=268).contains(&y) {
                if (24..=250).contains(&x) {
                    Some(NativeAction::StopwatchToggle)
                } else if (260..=430).contains(&x) {
                    Some(NativeAction::StopwatchReset)
                } else {
                    None
                }
            } else {
                None
            };
            if action.is_some() {
                return ClickOutcome {
                    animate: false,
                    action,
                };
            }
        }

        if self.widgets.current_tab == NativeTab::Drop {
            let action = if (geometry.height.round() as i32 - 76..=geometry.height.round() as i32 - 48).contains(&y) {
                if (24..=130).contains(&x) {
                    Some(NativeAction::ShelfOpenFirst)
                } else if (140..=272).contains(&x) {
                    Some(NativeAction::ShelfRevealFirst)
                } else if (282..=365).contains(&x) {
                    Some(NativeAction::ShelfClear)
                } else {
                    None
                }
            } else {
                None
            };
            if action.is_some() {
                return ClickOutcome {
                    animate: false,
                    action,
                };
            }
        }

        if self.widgets.current_tab == NativeTab::Notifications {
            let action = if (geometry.height.round() as i32 - 76..=geometry.height.round() as i32 - 48).contains(&y)
                && (24..=260).contains(&x)
            {
                Some(NativeAction::OpenNotificationSettings)
            } else {
                None
            };
            if action.is_some() {
                return ClickOutcome {
                    animate: false,
                    action,
                };
            }
        }

        if self.widgets.current_tab == NativeTab::Capture {
            let action = if (geometry.height.round() as i32 - 76..=geometry.height.round() as i32 - 48).contains(&y) {
                if (24..=138).contains(&x) {
                    Some(NativeAction::CaptureScreenshot)
                } else if (148..=230).contains(&x) {
                    Some(NativeAction::CaptureRegion)
                } else if (240..=342).contains(&x) {
                    Some(NativeAction::CaptureOpenLast)
                } else if (352..=482).contains(&x) {
                    Some(NativeAction::CaptureOpenFolder)
                } else {
                    None
                }
            } else {
                None
            };
            if action.is_some() {
                return ClickOutcome {
                    animate: false,
                    action,
                };
            }
        }

        if self.widgets.current_tab == NativeTab::Calendar {
            let action = if (geometry.height.round() as i32 - 76..=geometry.height.round() as i32 - 48).contains(&y)
                && (24..=190).contains(&x)
            {
                Some(NativeAction::CalendarRefresh)
            } else {
                None
            };
            if action.is_some() {
                return ClickOutcome {
                    animate: false,
                    action,
                };
            }
        }

        if self.widgets.current_tab == NativeTab::Stats {
            let bottom = geometry.height.round() as i32;
            let action = if (bottom - 100..=bottom - 76).contains(&y) && (24..=180).contains(&x) {
                Some(NativeAction::CaffeineToggle)
            } else if (bottom - 76..=bottom - 48).contains(&y) {
                if (24..=88).contains(&x) {
                    Some(NativeAction::VolumeDown)
                } else if (98..=168).contains(&x) {
                    Some(NativeAction::VolumeMute)
                } else if (178..=246).contains(&x) {
                    Some(NativeAction::VolumeUp)
                } else if (256..=354).contains(&x) {
                    Some(NativeAction::BrightnessDown)
                } else if (364..=470).contains(&x) {
                    Some(NativeAction::BrightnessUp)
                } else {
                    None
                }
            } else {
                None
            };
            if action.is_some() {
                return ClickOutcome {
                    animate: false,
                    action,
                };
            }
        }

        if self.widgets.current_tab == NativeTab::Settings {
            let bottom = geometry.height.round() as i32;
            let action = if (bottom - 104..=bottom - 78).contains(&y) {
                if (24..=112).contains(&x) {
                    Some(NativeAction::SettingsWidthDown)
                } else if (122..=210).contains(&x) {
                    Some(NativeAction::SettingsWidthUp)
                } else if (220..=318).contains(&x) {
                    Some(NativeAction::SettingsOpacityDown)
                } else if (328..=426).contains(&x) {
                    Some(NativeAction::SettingsOpacityUp)
                } else {
                    None
                }
            } else if (bottom - 76..=bottom - 48).contains(&y) {
                if (24..=132).contains(&x) {
                    Some(NativeAction::SettingsHoverToggle)
                } else if (142..=300).contains(&x) {
                    Some(NativeAction::SettingsOpenFile)
                } else {
                    None
                }
            } else {
                None
            };
            if action.is_some() {
                return ClickOutcome {
                    animate: false,
                    action,
                };
            }
        }

        let nav_y = geometry.height.round() as i32 - 46;
        if y >= nav_y {
            self.widgets.select_tab_at(x, geometry.width);
        }

        ClickOutcome {
            animate: false,
            action: None,
        }
    }

    pub fn backend_kind(&self) -> RenderBackendKind {
        self.dcomp_backend.kind()
    }

    pub fn set_snapshot(&mut self, snapshot: RuntimeSnapshot) {
        self.snapshot = snapshot;
    }

    pub fn set_settings(&mut self, settings: RavenSettings) {
        self.settings = settings;
    }

    pub fn select_tab(&mut self, tab: NativeTab) {
        self.widgets.current_tab = tab;
    }

    fn scene(&self) -> RenderScene {
        RenderScene {
            geometry: self.motion.geometry,
            is_open: self.motion.is_openish() || self.motion.content_opacity > 0.0,
            current_tab: self.widgets.current_tab,
            notch_opacity: self.motion.notch_opacity * (self.settings.appearance.notch_opacity / 100.0).clamp(0.0, 1.0),
            content_opacity: self.motion.content_opacity,
            clock_text: self.snapshot.clock_text.clone(),
            status_text: self.snapshot.status_text.clone(),
            cpu_pct: self.snapshot.cpu_pct,
            ram_pct: self.snapshot.ram_pct,
            battery_text: self
                .snapshot
                .battery_pct
                .map(|pct| format!("Battery: {pct:.0}%"))
                .unwrap_or_else(|| "Battery: unknown".to_string()),
            power_text: self
                .snapshot
                .on_ac_power
                .map(|on_ac| {
                    if on_ac {
                        "Power: plugged in".to_string()
                    } else {
                        "Power: battery".to_string()
                    }
                })
                .unwrap_or_else(|| "Power: unknown".to_string()),
            caffeine_text: if self.snapshot.caffeine.enabled {
                "Caffeine: on, keeping system awake".to_string()
            } else {
                "Caffeine: off".to_string()
            },
            media_title: self.snapshot.media.title.clone(),
            media_artist: self.snapshot.media.artist.clone(),
            media_album: self.snapshot.media.album.clone(),
            media_album_art_path: self.snapshot.media.album_art_path.clone(),
            media_source: self.snapshot.media.source_id.clone(),
            media_is_playing: self.snapshot.media.is_playing,
            media_has_media: self.snapshot.media.has_media,
            media_progress_pct: self.snapshot.media.progress_pct(),
            timer_label: self.snapshot.clock.timer_label(),
            timer_running: self.snapshot.clock.timer_running,
            stopwatch_label: self.snapshot.clock.stopwatch_label(),
            stopwatch_running: self.snapshot.clock.stopwatch_running,
            shelf_items: self
                .snapshot
                .shelf_items
                .iter()
                .map(|item| format!("{}  ({:.1} MB)", item.name, item.size as f64 / 1_048_576.0))
                .collect(),
            shelf_image_paths: self
                .snapshot
                .shelf_items
                .iter()
                .filter(|item| item.is_image)
                .map(|item| item.path.clone())
                .collect(),
            notification_access: self.snapshot.notification_access.clone(),
            notifications: self
                .snapshot
                .notifications
                .iter()
                .map(|item| {
                    if item.body.is_empty() {
                        format!("      {}: {}", item.app_name, item.title)
                    } else {
                        format!("      {}: {} - {}", item.app_name, item.title, item.body)
                    }
                })
                .collect(),
            notification_icon_paths: self
                .snapshot
                .notifications
                .iter()
                .map(|item| item.icon_path.clone())
                .collect(),
            capture_enabled: self.snapshot.capture.enabled,
            capture_mode: self.snapshot.capture.screenshot_mode.clone(),
            capture_recording_mode: self.snapshot.capture.recording_mode.clone(),
            capture_dir: self.snapshot.capture.screenshot_dir.clone(),
            capture_last: self
                .snapshot
                .capture
                .last_capture
                .as_ref()
                .map(|capture| {
                    format!(
                        "Last: {}  {}x{}  {:.1} MB",
                        capture.name,
                        capture.width,
                        capture.height,
                        capture.size_bytes as f64 / 1_048_576.0
                    )
                })
                .unwrap_or_else(|| "No captures yet".to_string()),
            capture_last_path: self
                .snapshot
                .capture
                .last_capture
                .as_ref()
                .map(|capture| capture.path.clone())
                .unwrap_or_default(),
            capture_message: self.snapshot.capture.message.clone(),
            calendar_source: self.snapshot.calendar.source.clone(),
            calendar_status: if self.snapshot.calendar.google_connected {
                format!(
                    "Google: {}  |  selected {}",
                    self.snapshot.calendar.google_email,
                    self.snapshot.calendar.selected_google_calendars
                )
            } else {
                self.snapshot.calendar.message.clone()
            },
            calendar_events: self
                .snapshot
                .calendar
                .items
                .iter()
                .map(|event| format!("{}  {}", event.date_str, event.title))
                .collect(),
            settings_path: settings_path().to_string_lossy().to_string(),
            settings_summary: format!(
                "Notch {:.0}x{:.0}, opacity {:.0}%, hover {}",
                self.settings.appearance.idle_width,
                self.settings.appearance.idle_height,
                self.settings.appearance.notch_opacity,
                if self.settings.hover.enabled { "on" } else { "off" }
            ),
            static_asset_paths: static_asset_paths(),
        }
    }

    pub fn render(&mut self, hwnd: HWND) {
        let scene = self.scene();
        let did_render_d2d = self.dcomp_backend.render_scene(hwnd, &scene);
        if did_render_d2d {
            unsafe {
                ValidateRect(hwnd, None);
            }
            return;
        }
        unsafe {
            let scale_bits = crate::window::PILL_SCALE_FACTOR.load(std::sync::atomic::Ordering::SeqCst);
            let scale = if scale_bits > 0 {
                f32::from_bits(scale_bits)
            } else {
                1.0
            };

            let mut paint = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut paint);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);

            if !did_render_d2d {
                let clear = CreateSolidBrush(COLORREF(0x000000));
                FillRect(hdc, &rect, clear);
                let _ = DeleteObject(HGDIOBJ(clear.0));

                let opacity = self.motion.notch_opacity * (self.settings.appearance.notch_opacity / 100.0).clamp(0.0, 1.0);
                let shade = (opacity * 8.0).round() as u32;
                let notch_brush = CreateSolidBrush(COLORREF(shade | (shade << 8) | (shade << 16)));
                let old = SelectObject(hdc, HGDIOBJ(notch_brush.0));

                let width = (self.motion.geometry.width * scale).round() as i32;
                let height = (self.motion.geometry.height * scale).round() as i32;
                let radius = (self.motion.geometry.radius * scale).round() as i32;
                let x = ((rect.right - rect.left) - width) / 2;
                RoundRect(hdc, x, 0, x + width, height, radius * 2, radius * 2);

                let _ = SelectObject(hdc, old);
                let _ = DeleteObject(HGDIOBJ(notch_brush.0));
            }

            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, COLORREF(0x00F5F5F7));
            let label = if self.motion.is_openish() {
                wide("Raven Native  |  migration shell")
            } else {
                wide("Raven Native")
            };
            let width_scaled = (self.motion.geometry.width * scale).round() as i32;
            let height_scaled = (self.motion.geometry.height * scale).round() as i32;
            let x = ((rect.right - rect.left) - width_scaled) / 2;
            TextOutW(hdc, x + (24.0 * scale).round() as i32, (9.0 * scale).round() as i32, &label);

            if self.motion.is_openish() {
                SetTextColor(hdc, COLORREF(0x00909090));
                TextOutW(hdc, x + (24.0 * scale).round() as i32, (44.0 * scale).round() as i32, &wide(&self.snapshot.clock_text));

                self.render_panel(hdc, x, height_scaled, scale);
            }

            EndPaint(hwnd, &paint);
        }
    }

    unsafe fn render_panel(&self, hdc: windows::Win32::Graphics::Gdi::HDC, x: i32, height: i32, scale: f32) {
        let panel = self.widgets.current_tab.descriptor();

        SetTextColor(hdc, COLORREF(0x00FFFFFF));
        TextOutW(hdc, x + (24.0 * scale).round() as i32, (88.0 * scale).round() as i32, &wide(panel.title));
        SetTextColor(hdc, COLORREF(0x00A8A8A8));
        TextOutW(hdc, x + (24.0 * scale).round() as i32, (112.0 * scale).round() as i32, &wide(panel.detail));

        let nav_top = height - (42.0 * scale).round() as i32;
        let tab_width = ((self.motion.geometry.width * scale) / NativeTab::ALL.len() as f32).round() as i32;
        for (index, tab) in NativeTab::ALL.iter().enumerate() {
            let tab_x = x + index as i32 * tab_width + (18.0 * scale).round() as i32;
            let color = if *tab == self.widgets.current_tab {
                COLORREF(0x00FFFFFF)
            } else {
                COLORREF(0x007A7A7A)
            };
            SetTextColor(hdc, color);
            TextOutW(hdc, tab_x, nav_top, &wide(tab.label()));
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

fn static_asset_paths() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .join("raven-tauri")
        .join("src")
        .join("assets");
    ["app_logo.png", "inbox-topbar.png", "localsend.png"]
        .iter()
        .map(|name| root.join(name).to_string_lossy().to_string())
        .collect()
}
