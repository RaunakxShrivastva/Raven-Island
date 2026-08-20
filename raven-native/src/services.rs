use crate::events::EventBus;
use crate::settings::{settings_path, CaptureSettings, MediaSettings, RavenSettings};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Local;
use image::{DynamicImage, GenericImageView, ImageFormat};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::System;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession,
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::NotificationKinds;
use windows::Win32::System::Power::{
    GetSystemPowerStatus, SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED,
    ES_SYSTEM_REQUIRED, SYSTEM_POWER_STATUS,
};
use windows::Storage::Streams::{DataReader, IRandomAccessStreamReference};

#[derive(Clone)]
pub struct ServiceRegistry {
    pub settings: SettingsService,
    pub window: WindowService,
    pub media: MediaService,
    pub notifications: NotificationService,
    pub capture: CaptureService,
    pub stats: StatsService,
    pub clock: ClockService,
    pub shortcuts: ShortcutService,
    pub shelf: ShelfService,
    pub calendar: CalendarService,
    pub caffeine: CaffeineService,
    snapshot: Arc<Mutex<RuntimeSnapshot>>,
}

impl ServiceRegistry {
    pub fn new(settings: RavenSettings, events: EventBus) -> Self {
        let snapshot = Arc::new(Mutex::new(RuntimeSnapshot::now()));
        Self {
            settings: SettingsService { current: settings, events: events.clone() },
            window: WindowService { events: events.clone() },
            media: MediaService {
                events: events.clone(),
                state: Arc::new(Mutex::new(MediaServiceState {
                    last_valid_session: None,
                    last_valid_media: None,
                    selected_source_id: None,
                    active_sessions: Vec::new(),
                    last_active_playing_source: None,
                })),
            },
            notifications: NotificationService { events: events.clone() },
            capture: CaptureService {
                events: events.clone(),
                last_result: Arc::new(Mutex::new(None)),
            },
            stats: StatsService {
                events: events.clone(),
                system: Arc::new(Mutex::new(System::new())),
            },
            clock: ClockService {
                events: events.clone(),
                state: Arc::new(Mutex::new(ClockState::default())),
            },
            shortcuts: ShortcutService { events: events.clone() },
            shelf: ShelfService {
                events: events.clone(),
                items: Arc::new(Mutex::new(load_shelf_items())),
            },
            calendar: CalendarService {
                events: events.clone(),
                items: Arc::new(Mutex::new(Vec::new())),
                last_message: Arc::new(Mutex::new("Not refreshed yet".to_string())),
                google_calendars: Arc::new(Mutex::new(Vec::new())),
            },
            caffeine: CaffeineService {
                events,
                state: Arc::new(Mutex::new(CaffeineRuntime::default())),
            },
            snapshot,
        }
    }

    pub fn event_bus(&self) -> EventBus {
        self.settings.events.clone()
    }

    pub fn refresh_clock(&self) -> RuntimeSnapshot {
        let next = RuntimeSnapshot::now();
        let clock = self.clock.read();
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.clock_text = next.clock_text;
            snapshot.clock = clock;
        }
        self.snapshot()
    }

    pub fn refresh_stats(&self) -> RuntimeSnapshot {
        self.refresh_stats_inner(true)
    }

    pub fn refresh_stats_light(&self) -> RuntimeSnapshot {
        self.refresh_stats_inner(false)
    }

    fn refresh_stats_inner(&self, include_processes: bool) -> RuntimeSnapshot {
        let stats = self.stats.read();
        let process_stats = include_processes.then(|| self.stats.get_processes());
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.cpu_pct = stats.cpu_pct;
            snapshot.ram_pct = stats.ram_pct;
            snapshot.ram_used_gb = stats.ram_used_gb;
            snapshot.ram_total_gb = stats.ram_total_gb;
            snapshot.battery_pct = stats.battery_pct;
            snapshot.on_ac_power = stats.on_ac_power;
            if let Some(process_stats) = process_stats {
                snapshot.process_stats = process_stats;
            }
            snapshot.status_text = format!(
                "CPU {:.0}%  RAM {:.0}%  Battery {}",
                stats.cpu_pct,
                stats.ram_pct,
                stats
                    .battery_pct
                    .map(|pct| format!("{pct:.0}%"))
                    .unwrap_or_else(|| "unknown".to_string())
            );
        }
        self.snapshot()
    }

    pub fn refresh_media(&self) -> RuntimeSnapshot {
        let media = self.media.read();
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.media = media.clone();
            if media.has_media {
                snapshot.status_text = format!(
                    "{} - {}",
                    media.title_or_placeholder(),
                    if media.artist.is_empty() { "Unknown artist" } else { &media.artist }
                );
            }
        }
        self.snapshot()
    }

    pub fn refresh_shelf(&self) -> RuntimeSnapshot {
        let items = self.shelf.items();
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.shelf_items = items;
        }
        self.snapshot()
    }

    pub fn add_shelf_paths(&self, paths: Vec<String>) -> RuntimeSnapshot {
        self.shelf.add_paths(paths);
        self.refresh_shelf()
    }

    pub fn open_first_shelf_item(&self) -> RuntimeSnapshot {
        self.shelf.open_first();
        self.refresh_shelf()
    }

    pub fn reveal_first_shelf_item(&self) -> RuntimeSnapshot {
        self.shelf.reveal_first();
        self.refresh_shelf()
    }

    pub fn clear_shelf(&self) -> RuntimeSnapshot {
        self.shelf.clear();
        self.refresh_shelf()
    }

    pub fn refresh_notifications(&self) -> RuntimeSnapshot {
        let notifications = self.notifications.read();
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.notification_access = notifications.access;
            snapshot.notifications = notifications.items;
        }
        self.snapshot()
    }

    pub fn open_notification_settings(&self) -> RuntimeSnapshot {
        self.notifications.open_settings();
        self.refresh_notifications()
    }

    pub fn refresh_capture(&self) -> RuntimeSnapshot {
        let settings = crate::settings::RavenSettings::load();
        let capture = self.capture.status(&settings.capture);
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.capture = capture;
        }
        self.snapshot()
    }

    pub fn capture_screenshot(&self) -> RuntimeSnapshot {
        let settings = crate::settings::RavenSettings::load();
        let result = self.capture.capture_screenshot(&settings.capture);
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.capture = self.capture.status(&settings.capture);
            snapshot.status_text = match result {
                Ok(result) => format!("Captured {}", result.name),
                Err(message) => format!("Capture failed: {message}"),
            };
        }
        self.snapshot()
    }

    pub fn capture_region(&self) -> RuntimeSnapshot {
        let settings = crate::settings::RavenSettings::load();
        let result = self.capture.capture_center_region(&settings.capture);
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.capture = self.capture.status(&settings.capture);
            snapshot.status_text = match result {
                Ok(result) => format!("Captured region {}", result.name),
                Err(message) => format!("Region capture failed: {message}"),
            };
        }
        self.snapshot()
    }

    pub fn capture_region_rect(&self, region: CaptureRegion) -> RuntimeSnapshot {
        let settings = crate::settings::RavenSettings::load();
        let result = self.capture.capture_region_rect(&settings.capture, region);
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.capture = self.capture.status(&settings.capture);
            snapshot.status_text = match result {
                Ok(result) => format!("Captured region {}", result.name),
                Err(message) => format!("Region capture failed: {message}"),
            };
        }
        self.snapshot()
    }

    pub fn open_last_capture(&self) -> RuntimeSnapshot {
        let settings = crate::settings::RavenSettings::load();
        self.capture.open_last_or_folder(&settings.capture);
        self.refresh_capture()
    }

    pub fn open_capture_folder(&self) -> RuntimeSnapshot {
        let settings = crate::settings::RavenSettings::load();
        self.capture.open_folder(&settings.capture);
        self.refresh_capture()
    }

    pub fn refresh_calendar(&self) -> RuntimeSnapshot {
        let settings = crate::settings::RavenSettings::load();
        let calendar = self.calendar.read(&settings.media);
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.calendar = calendar;
        }
        self.snapshot()
    }

    pub fn connect_google_calendar(&self) -> RuntimeSnapshot {
        let settings = crate::settings::RavenSettings::load();
        let calendar = self.calendar.connect_and_read(&settings.media);
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.calendar = calendar;
        }
        self.snapshot()
    }

    pub fn disconnect_google_calendar(&self) -> RuntimeSnapshot {
        // Delete the stored token so google_calendar_status returns false
        if let Ok(path) = google_token_path() {
            let _ = std::fs::remove_file(path);
        }
        // Clear cached calendar items in snapshot
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.calendar.google_connected = false;
            snapshot.calendar.google_email = String::new();
            snapshot.calendar.google_calendars = Vec::new();
            snapshot.calendar.items = Vec::new();
            snapshot.calendar.message = "Disconnected from Google Calendar.".to_string();
        }
        if let Ok(mut stored) = self.calendar.google_calendars.lock() {
            *stored = Vec::new();
        }
        self.snapshot()
    }

    pub fn toggle_caffeine(&self) -> RuntimeSnapshot {
        let caffeine = self.caffeine.toggle();
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.caffeine = caffeine;
        }
        self.snapshot()
    }

    pub fn toggle_caffeine_screen(&self, keep_screen_awake: bool) -> RuntimeSnapshot {
        let enabled = self.caffeine.state.lock().map(|runtime| runtime.status.enabled).unwrap_or(false);
        let screen_awake = self.caffeine.state.lock().map(|runtime| runtime.status.keep_screen_awake).unwrap_or(false);
        
        let caffeine = if enabled && screen_awake == keep_screen_awake {
            self.caffeine.stop()
        } else {
            self.caffeine.start(keep_screen_awake)
        };
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.caffeine = caffeine;
        }
        self.snapshot()
    }

    pub fn volume_down(&self) -> RuntimeSnapshot {
        send_volume_key(VolumeKey::Down);
        self.snapshot()
    }

    pub fn volume_mute(&self) -> RuntimeSnapshot {
        send_volume_key(VolumeKey::Mute);
        self.snapshot()
    }

    pub fn volume_up(&self) -> RuntimeSnapshot {
        send_volume_key(VolumeKey::Up);
        self.snapshot()
    }

    pub fn brightness_down(&self) -> RuntimeSnapshot {
        adjust_brightness(-10);
        self.snapshot()
    }

    pub fn brightness_up(&self) -> RuntimeSnapshot {
        adjust_brightness(10);
        self.snapshot()
    }

    pub fn open_settings_file(&self) -> RuntimeSnapshot {
        let path = settings_path();
        if path.exists() {
            open_path(&path.to_string_lossy());
        } else if let Some(parent) = path.parent() {
            open_path(&parent.to_string_lossy());
        }
        self.snapshot()
    }

    pub fn start_focus_session(&self, goal: String, duration_str: String) -> RuntimeSnapshot {
        self.clock.start_focus_session(goal, duration_str);
        self.refresh_clock()
    }

    pub fn stop_focus_session(&self) -> RuntimeSnapshot {
        self.clock.stop_focus_session();
        self.refresh_clock()
    }

    pub fn toggle_timer(&self) -> RuntimeSnapshot {
        self.clock.toggle_timer();
        self.refresh_clock()
    }

    pub fn reset_timer(&self) -> RuntimeSnapshot {
        self.clock.reset_timer();
        self.refresh_clock()
    }

    pub fn set_timer_duration(&self, secs: u64) -> RuntimeSnapshot {
        self.clock.set_timer_duration(secs);
        self.refresh_clock()
    }

    pub fn toggle_stopwatch(&self) -> RuntimeSnapshot {
        self.clock.toggle_stopwatch();
        self.refresh_clock()
    }

    pub fn reset_stopwatch(&self) -> RuntimeSnapshot {
        self.clock.reset_stopwatch();
        self.refresh_clock()
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        self.snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_else(|_| RuntimeSnapshot::now())
    }
}

#[derive(Clone, Debug)]
pub struct NativeProcessStat {
    pub name: String,
    pub exe_path: String,
    pub cpu_pct: f32,
    pub ram_pct: f32,
    pub gpu_pct: f32,
}

#[derive(Clone, Debug)]
pub struct RuntimeSnapshot {
    pub clock_text: String,
    pub status_text: String,
    pub cpu_pct: f32,
    pub ram_pct: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub battery_pct: Option<f32>,
    pub on_ac_power: Option<bool>,
    pub media: NativeMedia,
    pub clock: NativeClock,
    pub shelf_items: Vec<ShelfItem>,
    pub notification_access: String,
    pub notifications: Vec<NativeNotification>,
    pub capture: NativeCaptureStatus,
    pub calendar: NativeCalendarStatus,
    pub caffeine: NativeCaffeineStatus,
    pub process_stats: Vec<NativeProcessStat>,
}

impl RuntimeSnapshot {
    pub fn now() -> Self {
        let now = Local::now();
        Self {
            clock_text: now.format("%a %d %b %I:%M %p").to_string(),
            status_text: "Native services online".to_string(),
            cpu_pct: 0.0,
            ram_pct: 0.0,
            ram_used_gb: 0.0,
            ram_total_gb: 0.0,
            battery_pct: None,
            on_ac_power: None,
            media: NativeMedia::default(),
            clock: NativeClock::default(),
            shelf_items: Vec::new(),
            notification_access: "unknown".to_string(),
            notifications: Vec::new(),
            capture: NativeCaptureStatus::default(),
            calendar: NativeCalendarStatus::default(),
            caffeine: NativeCaffeineStatus::default(),
            process_stats: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct SettingsService {
    pub current: RavenSettings,
    pub events: EventBus,
}

#[derive(Clone)]
pub struct WindowService {
    pub events: EventBus,
}

#[derive(Clone)]
pub struct MediaService {
    pub events: EventBus,
    state: Arc<Mutex<MediaServiceState>>,
}

struct MediaServiceState {
    last_valid_session: Option<GlobalSystemMediaTransportControlsSession>,
    last_valid_media: Option<NativeMedia>,
    selected_source_id: Option<String>,
    active_sessions: Vec<GlobalSystemMediaTransportControlsSession>,
    last_active_playing_source: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct NativeMediaSession {
    pub source_id: String,
    pub clean_name: String,
    pub icon_path: String,
    pub is_active: bool,
}

#[derive(Clone, Debug)]
pub struct NativeMedia {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source_id: String,
    pub raw_source_id: String,
    pub source_icon_path: String,
    pub is_playing: bool,
    pub has_media: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub album_art_path: String,
    pub sessions: Vec<NativeMediaSession>,
}

impl Default for NativeMedia {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            source_id: String::new(),
            raw_source_id: String::new(),
            source_icon_path: String::new(),
            is_playing: false,
            has_media: false,
            position_seconds: 0.0,
            duration_seconds: 0.0,
            album_art_path: String::new(),
            sessions: Vec::new(),
        }
    }
}

impl NativeMedia {
    pub fn title_or_placeholder(&self) -> &str {
        if self.title.is_empty() {
            "No media"
        } else {
            &self.title
        }
    }

    pub fn progress_pct(&self) -> f32 {
        if self.duration_seconds <= 0.0 {
            return 0.0;
        }
        ((self.position_seconds / self.duration_seconds) * 100.0).clamp(0.0, 100.0) as f32
    }
}

impl MediaService {
    pub fn read(&self) -> NativeMedia {
        let Some(manager) = get_session_manager() else {
            if let Ok(state) = self.state.lock() {
                if let Some(ref last_media) = state.last_valid_media {
                    let mut media = last_media.clone();
                    media.is_playing = false;
                    return media;
                }
            }
            return NativeMedia::default();
        };

        let mut current_sessions = Vec::new();
        if let Ok(sessions_view) = manager.GetSessions() {
            if let Ok(count) = sessions_view.Size() {
                for i in 0..count {
                    if let Ok(session) = sessions_view.GetAt(i) {
                        current_sessions.push(session);
                    }
                }
            }
        }

        let system_current = manager.GetCurrentSession().ok();
        let system_current_is_playing = system_current.as_ref().map(|s| {
            s.GetPlaybackInfo()
                .and_then(|info| info.PlaybackStatus())
                .map(|status| status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                .unwrap_or(false)
        }).unwrap_or(false);

        if let Some(ref sys_sess) = system_current {
            if system_current_is_playing {
                if let Ok(sys_id) = sys_sess.SourceAppUserModelId() {
                    let sys_id_str = sys_id.to_string();
                    if let Ok(mut state) = self.state.lock() {
                        let should_switch = state.last_active_playing_source.as_ref()
                            .map(|last_id| *last_id != sys_id_str)
                            .unwrap_or(true);
                        if should_switch {
                            state.selected_source_id = Some(sys_id_str.clone());
                            state.last_active_playing_source = Some(sys_id_str);
                        }
                    }
                }
            }
        }

        let mut target_session = None;
        if let Ok(mut state) = self.state.lock() {
            state.active_sessions = current_sessions.clone();

            if let Some(ref selected_id) = state.selected_source_id {
                target_session = current_sessions.iter().find(|s| {
                    s.SourceAppUserModelId()
                        .map(|id| id.to_string() == *selected_id)
                        .unwrap_or(false)
                }).cloned();
            }

            if target_session.is_none() {
                target_session = system_current.clone().or_else(|| current_sessions.first().cloned());
                if let Some(ref session) = target_session {
                    state.selected_source_id = session.SourceAppUserModelId().map(|id| id.to_string()).ok();
                }
            }
        }

        let mut media = if let Some(ref session) = target_session {
            get_session_media(session)
        } else {
            NativeMedia::default()
        };

        if !media.has_media || media.title.trim().is_empty() {
            let mut first_valid: Option<(NativeMedia, GlobalSystemMediaTransportControlsSession)> = None;
            let mut playing_valid: Option<(NativeMedia, GlobalSystemMediaTransportControlsSession)> = None;

            for session in &current_sessions {
                let candidate = get_session_media(session);
                if candidate.has_media && !candidate.title.trim().is_empty() {
                    if candidate.is_playing {
                        playing_valid = Some((candidate, session.clone()));
                        break;
                    }
                    if first_valid.is_none() {
                        first_valid = Some((candidate, session.clone()));
                    }
                }
            }

            if let Some((fallback_media, fallback_session)) = playing_valid.or(first_valid) {
                target_session = Some(fallback_session);
                media = fallback_media;
            }
        }

        if let Ok(mut state) = self.state.lock() {
            if let Some(ref last) = state.last_valid_media {
                let source_still_active = current_sessions.iter().any(|s| {
                    s.SourceAppUserModelId()
                        .map(|id| id.to_string() == last.raw_source_id)
                        .unwrap_or(false)
                });
                if !source_still_active {
                    state.last_valid_media = None;
                    state.last_valid_session = None;
                }
            }

            if media.has_media && !media.title.trim().is_empty() {
                state.last_valid_session = target_session.clone();
                state.last_valid_media = Some(media.clone());
            } else {
                if let Some(ref last_media) = state.last_valid_media {
                    media = last_media.clone();
                    media.is_playing = false;
                }
            }
        }

        if let Some(ref session) = target_session {
            let raw_id = session.SourceAppUserModelId().map(|id| id.to_string()).unwrap_or_default();
            let clean_id = raw_id.replace(".exe", "");
            let mut capitalized = clean_id;
            if let Some(first_char) = capitalized.get_mut(0..1) {
                first_char.make_ascii_uppercase();
            }

            if current_sessions.len() > 1 {
                if let Some(idx) = current_sessions.iter().position(|s| {
                    s.SourceAppUserModelId()
                        .map(|id| id.to_string() == raw_id)
                        .unwrap_or(false)
                }) {
                    media.source_id = format!("{} ({} of {})", capitalized, idx + 1, current_sessions.len());
                } else {
                    media.source_id = capitalized;
                }
            } else {
                media.source_id = capitalized;
            }
        }

        let mut sessions_list = Vec::new();
        if let Ok(state) = self.state.lock() {
            for s in &current_sessions {
                if let Ok(raw_id) = s.SourceAppUserModelId() {
                    let raw_id_str = raw_id.to_string();
                    let clean_id = raw_id_str.replace(".exe", "");
                    let mut clean_name = clean_id;
                    if let Some(first_char) = clean_name.get_mut(0..1) {
                        first_char.make_ascii_uppercase();
                    }
                    
                    let icon_path = extract_app_icon_path(&raw_id_str).unwrap_or_default();
                    let is_active = Some(raw_id_str.clone()) == state.selected_source_id;
                    
                    sessions_list.push(NativeMediaSession {
                        source_id: raw_id_str,
                        clean_name,
                        icon_path,
                        is_active,
                    });
                }
            }
        }
        media.sessions = sessions_list;

        media
    }

    pub fn play_pause(&self) {
        if let Some(session) = self.get_target_session() {
            if let Ok(op) = session.TryTogglePlayPauseAsync() {
                let _ = op.get();
            }
        }
    }

    pub fn next(&self) {
        if let Some(session) = self.get_target_session() {
            if let Ok(op) = session.TrySkipNextAsync() {
                let _ = op.get();
            }
        }
    }

    pub fn previous(&self) {
        if let Some(session) = self.get_target_session() {
            if let Ok(op) = session.TrySkipPreviousAsync() {
                let _ = op.get();
            }
        }
    }

    pub fn seek(&self, forward: bool) {
        if let Some(session) = self.get_target_session() {
            if let Ok(timeline) = session.GetTimelineProperties() {
                if let Ok(position) = timeline.Position() {
                    let current_pos_ticks = position.Duration; // 100ns units (ticks)
                    let offset_ticks = 10 * 10_000_000; // 10 seconds
                    let target_pos_ticks = if forward {
                        current_pos_ticks + offset_ticks
                    } else {
                        current_pos_ticks.saturating_sub(offset_ticks)
                    };
                    if let Ok(op) = session.TryChangePlaybackPositionAsync(target_pos_ticks) {
                        let _ = op.get();
                    }
                }
            }
        }
    }

    pub fn cycle_session(&self, forward: bool) {
        let Some(manager) = get_session_manager() else { return; };
        let mut current_sessions = Vec::new();
        if let Ok(sessions_view) = manager.GetSessions() {
            if let Ok(count) = sessions_view.Size() {
                for i in 0..count {
                    if let Ok(session) = sessions_view.GetAt(i) {
                        current_sessions.push(session);
                    }
                }
            }
        }

        if current_sessions.len() <= 1 {
            return;
        }

        if let Ok(mut state) = self.state.lock() {
            let current_id = state.selected_source_id.clone().unwrap_or_default();
            let current_idx = current_sessions.iter().position(|s| {
                s.SourceAppUserModelId()
                    .map(|id| id.to_string() == current_id)
                    .unwrap_or(false)
            }).unwrap_or(0);

            let new_idx = if forward {
                (current_idx + 1) % current_sessions.len()
            } else {
                (current_idx + current_sessions.len() - 1) % current_sessions.len()
            };

            if let Some(new_session) = current_sessions.get(new_idx) {
                if let Ok(new_id) = new_session.SourceAppUserModelId() {
                    let new_id_str = new_id.to_string();
                    state.selected_source_id = Some(new_id_str.clone());
                    state.last_valid_session = Some(new_session.clone());

                    // Set last_active_playing_source to the currently playing session (if any)
                    // so we don't immediately switch back to it on the next poll.
                    let playing_session = current_sessions.iter().find(|s| {
                        s.GetPlaybackInfo()
                            .and_then(|info| info.PlaybackStatus())
                            .map(|status| status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                            .unwrap_or(false)
                    });
                    if let Some(p_sess) = playing_session {
                        if let Ok(p_id) = p_sess.SourceAppUserModelId() {
                            state.last_active_playing_source = Some(p_id.to_string());
                        }
                    } else {
                        state.last_active_playing_source = None;
                    }
                }
            }
        }

        self.events.emit(crate::events::RavenEvent::MediaChanged);
    }

    pub fn switch_to_session(&self, source_id: &str) {
        let Some(manager) = get_session_manager() else { return; };
        let mut current_sessions = Vec::new();
        if let Ok(sessions_view) = manager.GetSessions() {
            if let Ok(count) = sessions_view.Size() {
                for i in 0..count {
                    if let Ok(session) = sessions_view.GetAt(i) {
                        current_sessions.push(session);
                    }
                }
            }
        }

        if let Ok(mut state) = self.state.lock() {
            if let Some(session) = current_sessions.iter().find(|s| {
                s.SourceAppUserModelId()
                    .map(|id| id.to_string() == source_id)
                    .unwrap_or(false)
            }) {
                state.selected_source_id = Some(source_id.to_string());
                state.last_valid_session = Some(session.clone());

                // Set last_active_playing_source to the currently playing session (if any)
                let playing_session = current_sessions.iter().find(|s| {
                    s.GetPlaybackInfo()
                        .and_then(|info| info.PlaybackStatus())
                        .map(|status| status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                        .unwrap_or(false)
                });
                if let Some(p_sess) = playing_session {
                    if let Ok(p_id) = p_sess.SourceAppUserModelId() {
                        state.last_active_playing_source = Some(p_id.to_string());
                    }
                } else {
                    state.last_active_playing_source = None;
                }
            }
        }

        self.events.emit(crate::events::RavenEvent::MediaChanged);
    }

    fn get_target_session(&self) -> Option<GlobalSystemMediaTransportControlsSession> {
        let manager = get_session_manager()?;
        let mut current_sessions = Vec::new();
        if let Ok(sessions_view) = manager.GetSessions() {
            if let Ok(count) = sessions_view.Size() {
                for i in 0..count {
                    if let Ok(session) = sessions_view.GetAt(i) {
                        current_sessions.push(session);
                    }
                }
            }
        }

        if let Ok(state) = self.state.lock() {
            if let Some(ref selected_id) = state.selected_source_id {
                if let Some(session) = current_sessions.iter().find(|s| {
                    s.SourceAppUserModelId()
                        .map(|id| id.to_string() == *selected_id)
                        .unwrap_or(false)
                }) {
                    return Some(session.clone());
                }
            }
            if let Some(ref session) = state.last_valid_session {
                return Some(session.clone());
            }
        }
        manager.GetCurrentSession().ok()
    }
}

#[derive(Clone, Copy, Debug)]
enum MediaCommand {
    PlayPause,
    Next,
    Previous,
}

fn get_session_manager() -> Option<GlobalSystemMediaTransportControlsSessionManager> {
    let op = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?;
    op.get().ok()
}

fn get_session_media(session: &GlobalSystemMediaTransportControlsSession) -> NativeMedia {
    let mut media = NativeMedia::default();
    media.has_media = true;
    media.source_id = session
        .SourceAppUserModelId()
        .unwrap_or_default()
        .to_string();
    media.raw_source_id = media.source_id.clone();

    // Dynamically extract the native app icon and cache it locally as a file path
    if !media.source_id.is_empty() {
        if let Some(path) = extract_app_icon_path(&media.source_id) {
            media.source_icon_path = path;
        }
    }

    if let Ok(props_op) = session.TryGetMediaPropertiesAsync() {
        if let Ok(props) = props_op.get() {
            media.title = props.Title().unwrap_or_default().to_string();
            media.artist = props.Artist().unwrap_or_default().to_string();
            media.album = props.AlbumTitle().unwrap_or_default().to_string();
            if let Ok(thumbnail) = props.Thumbnail() {
                if let Some(path) = save_stream_to_temp(&thumbnail, "raven_media_art.png") {
                    media.album_art_path = path;
                }
            }
        }
    }

    if let Ok(playback_info) = session.GetPlaybackInfo() {
        if let Ok(status) = playback_info.PlaybackStatus() {
            media.is_playing =
                status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
        }
    }

    if let Ok(timeline) = session.GetTimelineProperties() {
        if let Ok(position) = timeline.Position() {
            media.position_seconds = position.Duration as f64 / 10_000_000.0;
        }
        if let Ok(end) = timeline.EndTime() {
            media.duration_seconds = end.Duration as f64 / 10_000_000.0;
        }
    }

    // If the session has no title and artist, mark has_media as false to trigger the cache fallback
    if media.title.trim().is_empty() && media.artist.trim().is_empty() {
        media.has_media = false;
    }

    media
}

fn extract_app_icon_path(sid: &str) -> Option<String> {
    let clean_sid = sid.trim();
    let cache_filename = format!("raven_source_icon_{}.png", sha256_hash(clean_sid));
    let target_path = std::env::temp_dir().join(&cache_filename);

    // If it already exists, return the cached path
    if target_path.exists() {
        return Some(target_path.to_string_lossy().to_string());
    }

    let escaped_sid = clean_sid.replace("'", "''");
    let target_path_str = target_path.to_string_lossy().replace("'", "''");

    let script = format!(
        r#"
        Add-Type -AssemblyName System.Drawing
        $sid = '{}'
        $targetPath = '{}'
        $cleanSid = $sid.Trim()
        if ($cleanSid -like "*.exe") {{
            $cleanSid = $cleanSid.Substring(0, $cleanSid.Length - 4)
        }}
        
        $lowerSid = $cleanSid.ToLower()
        $procName = $cleanSid
        if ($lowerSid -match "chrome") {{ $procName = "chrome" }}
        elseif ($lowerSid -match "edge") {{ $procName = "msedge" }}
        elseif ($lowerSid -match "firefox") {{ $procName = "firefox" }}
        elseif ($lowerSid -match "brave") {{ $procName = "brave" }}
        elseif ($lowerSid -match "opera") {{ $procName = "opera" }}
        elseif ($lowerSid -match "vivaldi") {{ $procName = "vivaldi" }}
        elseif ($lowerSid -match "spotify") {{ $procName = "spotify" }}
        elseif ($lowerSid -match "vlc") {{ $procName = "vlc" }}
        elseif ($cleanSid -match "^([^.!_]+)") {{
            $procName = $Matches[1]
        }}

        $path = ""
        $isImage = $false
        $candidateName = $cleanSid
        if ($cleanSid -match "!(.+)$") {{
            $candidateName = $Matches[1]
        }}
        
        # 1. Try finding running process
        $proc = Get-Process | Where-Object {{
            $_.ProcessName -eq $cleanSid -or 
            $_.Name -eq $cleanSid -or 
            $_.ProcessName -eq $candidateName -or 
            $_.Name -eq $candidateName -or
            $_.ProcessName -eq $procName -or
            $_.Name -eq $procName
        }} | Select-Object -First 1
        if ($proc) {{ $path = $proc.Path }}
        
        # 2. Try App Paths Registry Registry fallback
        if (-not $path) {{
            $exeName = $procName
            if ($exeName -notlike "*.exe") {{ $exeName = "$exeName.exe" }}
            $regPaths = @(
                "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\$exeName",
                "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\$exeName"
            )
            foreach ($rp in $regPaths) {{
                if (Test-Path $rp) {{
                    $val = (Get-ItemProperty $rp -ErrorAction SilentlyContinue).'(default)'
                    if ($val -and (Test-Path $val)) {{ $path = $val; break }}
                }}
            }}
        }}

        # 3. Try standard installation fallback paths
        if (-not $path) {{
            if ($procName -eq "chrome") {{
                $checkPaths = @(
                    "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
                    "${{env:ProgramFiles(x86)}}\Google\Chrome\Application\chrome.exe",
                    "$env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe"
                )
                foreach ($p in $checkPaths) {{ if (Test-Path $p) {{ $path = $p; break }} }}
            }} elseif ($procName -eq "msedge") {{
                $checkPaths = @(
                    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
                    "${{env:ProgramFiles(x86)}}\Microsoft\Edge\Application\msedge.exe"
                )
                foreach ($p in $checkPaths) {{ if (Test-Path $p) {{ $path = $p; break }} }}
            }} elseif ($procName -eq "spotify") {{
                $checkPaths = @(
                    "$env:APPDATA\Spotify\Spotify.exe",
                    "$env:LOCALAPPDATA\Microsoft\WindowsApps\Spotify.exe",
                    "$env:ProgramFiles\Spotify\Spotify.exe"
                )
                foreach ($p in $checkPaths) {{ if (Test-Path $p) {{ $path = $p; break }} }}
            }}
        }}

        if (-not $path -and ($sid -match "(.+?)!(.+)")) {{
            $pfn = $Matches[1]
            $pkg = Get-AppxPackage | Where-Object {{ $_.PackageFamilyName -eq $pfn }} | Select-Object -First 1
            if ($pkg -and $pkg.InstallLocation) {{
                $manifestPath = Join-Path $pkg.InstallLocation "AppxManifest.xml"
                if (Test-Path $manifestPath) {{
                    [xml]$xml = Get-Content $manifestPath
                    $logoRelPath = ""
                    $nodes = $xml.SelectNodes("//*[local-name()='VisualElements' or local-name()='Properties']")
                    foreach ($node in $nodes) {{
                        if ($node.Square44x44Logo) {{ $logoRelPath = $node.Square44x44Logo; break }}
                        if ($node.Logo) {{ $logoRelPath = $node.Logo; break }}
                        if ($node.Square30x30Logo) {{ $logoRelPath = $node.Square30x30Logo; break }}
                    }}
                    if ($logoRelPath) {{
                        $logoRelPath = $logoRelPath.TrimStart("\/.")
                        $logoFullPath = Join-Path $pkg.InstallLocation $logoRelPath
                        $logoDir = Split-Path $logoFullPath
                        $logoFile = Split-Path $logoFullPath -Leaf
                        $filePattern = $logoFile -replace '\.png$', '*.png' -replace '\.jpg$', '*.jpg'
                        if (Test-Path $logoDir) {{
                            $matchingFiles = Get-ChildItem -Path $logoDir -Filter $filePattern | Sort-Object Length -Descending
                            if ($matchingFiles) {{
                                $path = $matchingFiles[0].FullName
                                $isImage = $true
                            }}
                        }}
                    }}
                }}
            }}
        }}
        if (-not $path) {{
            $app = Get-StartApps | Where-Object {{ $_.AppID -eq $sid -or $_.AppID -eq $cleanSid -or $_.AppID -like "*$cleanSid*" }} | Select-Object -First 1
            if ($app) {{
                if (Test-Path $app.AppID) {{ $path = $app.AppID }}
            }}
        }}
        if ($path -and (Test-Path $path)) {{
            try {{
                $bmp = $null
                if ($isImage) {{
                    $bmp = New-Object System.Drawing.Bitmap($path)
                }} else {{
                    $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($path)
                    if ($icon) {{
                        $bmp = $icon.ToBitmap()
                    }}
                }}
                if ($bmp) {{
                    $bmp.Save($targetPath, [System.Drawing.Imaging.ImageFormat]::Png)
                    $bmp.Dispose()
                }}
            }} catch {{}}
        }}
        "#,
        escaped_sid, target_path_str
    );

    let mut cmd = std::process::Command::new("powershell");
    cmd.args(&["-NoProfile", "-Command", &script]);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let _ = cmd.output();

    if target_path.exists() {
        Some(target_path.to_string_lossy().to_string())
    } else {
        None
    }
}

pub fn extract_file_icon_path(file_path: &str) -> Option<String> {
    let clean_path = file_path.trim();
    if clean_path.is_empty() { return None; }
    let cache_filename = format!("raven_file_icon_{}.png", sha256_hash(clean_path));
    let target_path = std::env::temp_dir().join(&cache_filename);
    if target_path.exists() {
        return Some(target_path.to_string_lossy().to_string());
    }
    let escaped_path = clean_path.replace("'", "''");
    let target_path_str = target_path.to_string_lossy().replace("'", "''");
    let script = format!(
        r#"
        Add-Type -AssemblyName System.Drawing
        $filePath = '{}'
        $targetPath = '{}'
        if (Test-Path $filePath) {{
            try {{
                $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($filePath)
                if ($icon) {{
                    $bmp = $icon.ToBitmap()
                    $bmp.Save($targetPath, [System.Drawing.Imaging.ImageFormat]::Png)
                    $bmp.Dispose()
                }}
            }} catch {{}}
        }}
        "#,
        escaped_path, target_path_str
    );
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(&["-NoProfile", "-Command", &script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let _ = cmd.output();
    if target_path.exists() {
        Some(target_path.to_string_lossy().to_string())
    } else {
        None
    }
}

fn sha256_hash(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn save_stream_to_temp(reference: &IRandomAccessStreamReference, filename: &str) -> Option<String> {
    let stream = reference.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 { return None; }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size).ok()?.get().ok()?;
    let mut buffer = vec![0u8; size as usize];
    reader.ReadBytes(&mut buffer).ok()?;
    
    let path = std::env::temp_dir().join(filename);
    std::fs::write(&path, &buffer).ok()?;
    Some(path.to_string_lossy().to_string())
}

#[derive(Clone)]
pub struct NotificationService {
    pub events: EventBus,
}

#[derive(Clone, Debug)]
pub struct NativeNotification {
    pub id: u32,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub icon_path: String,
}

#[derive(Clone, Debug, Default)]
pub struct NotificationSnapshot {
    pub access: String,
    pub items: Vec<NativeNotification>,
}

impl NotificationService {
    pub fn read(&self) -> NotificationSnapshot {
        let Ok(listener) = UserNotificationListener::Current() else {
            return NotificationSnapshot {
                access: "unavailable".to_string(),
                items: Vec::new(),
            };
        };

        let access = listener
            .GetAccessStatus()
            .map(access_status_label)
            .unwrap_or("unknown")
            .to_string();
        if access != "allowed" {
            return NotificationSnapshot {
                access,
                items: Vec::new(),
            };
        }

        let mut items = Vec::new();
        if let Ok(op) = listener.GetNotificationsAsync(NotificationKinds::Toast) {
            if let Ok(notifications) = op.get() {
                let size = notifications.Size().unwrap_or(0);
                let start = size.saturating_sub(5);
                for index in start..size {
                    if let Ok(notification) = notifications.GetAt(index) {
                        let id = notification.Id().unwrap_or(0);
                        let mut item = NativeNotification {
                            id,
                            app_name: "System".to_string(),
                            title: String::new(),
                            body: String::new(),
                            icon_path: String::new(),
                        };
                        if let Ok(app_info) = notification.AppInfo() {
                            if let Ok(display_info) = app_info.DisplayInfo() {
                                if let Ok(name) = display_info.DisplayName() {
                                    item.app_name = name.to_string();
                                }
                                if let Ok(logo) = display_info.GetLogo(windows::Foundation::Size::default()) {
                                    use windows::core::ComInterface;
                                    if let Ok(reference) = logo.cast::<windows::Storage::Streams::IRandomAccessStreamReference>() {
                                        if let Some(path) = save_stream_to_temp(&reference, &format!("raven_app_icon_{}.png", id)) {
                                            item.icon_path = path;
                                        }
                                    }
                                }
                            }
                        }
                        let lines = notification_text_lines(&notification);
                        if let Some(title) = lines.first() {
                            item.title = title.clone();
                        }
                        if lines.len() > 1 {
                            item.body = lines[1..].join(" · ");
                        }
                        if item.title.is_empty() {
                            item.title = item.app_name.clone();
                        }
                        items.push(item);
                    }
                }
            }
        }
        items.reverse();
        NotificationSnapshot { access, items }
    }

    pub fn open_settings(&self) {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-WindowStyle",
                    "Hidden",
                    "-Command",
                    "Start-Process 'ms-settings:notifications'",
                ])
                .creation_flags(0x08000000)
                .spawn();
        }
    }
}

fn access_status_label(status: UserNotificationListenerAccessStatus) -> &'static str {
    if status == UserNotificationListenerAccessStatus::Allowed {
        "allowed"
    } else if status == UserNotificationListenerAccessStatus::Denied {
        "denied"
    } else {
        "unknown"
    }
}

fn notification_text_lines(
    user_notification: &windows::UI::Notifications::UserNotification,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Ok(notification) = user_notification.Notification() {
        if let Ok(visual) = notification.Visual() {
            if let Ok(bindings) = visual.Bindings() {
                let binding_count = bindings.Size().unwrap_or(0);
                for binding_index in 0..binding_count {
                    if let Ok(binding) = bindings.GetAt(binding_index) {
                        if let Ok(text_elements) = binding.GetTextElements() {
                            let text_count = text_elements.Size().unwrap_or(0);
                            for text_index in 0..text_count {
                                if let Ok(text_element) = text_elements.GetAt(text_index) {
                                    if let Ok(text_node) = text_element.Text() {
                                        let text = text_node.to_string().trim().to_string();
                                        if !text.is_empty()
                                            && !lines.iter().any(|existing| existing == &text)
                                        {
                                            lines.push(text);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    lines
}

#[derive(Clone)]
pub struct CaptureService {
    pub events: EventBus,
    last_result: Arc<Mutex<Option<CaptureResult>>>,
}

#[derive(Clone, Debug)]
pub struct CaptureResult {
    pub path: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub created_at: String,
}

#[derive(Clone, Copy, Debug)]
pub struct CaptureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct NativeCaptureStatus {
    pub enabled: bool,
    pub screenshot_mode: String,
    pub recording_mode: String,
    pub screenshot_dir: String,
    pub recording_dir: String,
    pub include_cursor: bool,
    pub mic_enabled: bool,
    pub system_audio_enabled: bool,
    pub recording_supported: bool,
    pub last_capture: Option<CaptureResult>,
    pub message: String,
}

impl Default for NativeCaptureStatus {
    fn default() -> Self {
        let screenshot_dir = default_screenshot_dir();
        let recording_dir = default_recording_dir();
        Self {
            enabled: true,
            screenshot_mode: "fullscreen".to_string(),
            recording_mode: "fullscreen".to_string(),
            screenshot_dir: screenshot_dir.to_string_lossy().to_string(),
            recording_dir: recording_dir.to_string_lossy().to_string(),
            include_cursor: false,
            mic_enabled: false,
            system_audio_enabled: false,
            recording_supported: false,
            last_capture: None,
            message: "Ready".to_string(),
        }
    }
}

impl CaptureService {
    pub fn status(&self, settings: &CaptureSettings) -> NativeCaptureStatus {
        let screenshot_dir = capture_screenshot_dir(settings);
        let recording_dir = capture_recording_dir(settings);
        NativeCaptureStatus {
            enabled: settings.enabled,
            screenshot_mode: settings.default_screenshot_mode.clone(),
            recording_mode: settings.default_recording_mode.clone(),
            screenshot_dir: screenshot_dir.to_string_lossy().to_string(),
            recording_dir: recording_dir.to_string_lossy().to_string(),
            include_cursor: settings.include_cursor,
            mic_enabled: settings.mic_enabled,
            system_audio_enabled: settings.system_audio_enabled,
            recording_supported: false,
            last_capture: self.last_result.lock().ok().and_then(|result| result.clone()),
            message: "Ready".to_string(),
        }
    }

    pub fn capture_screenshot(&self, settings: &CaptureSettings) -> Result<CaptureResult, String> {
        if !settings.enabled {
            return Err("Capture Studio is disabled in settings.".to_string());
        }

        let screens = screenshots::Screen::all().map_err(|error| error.to_string())?;
        let screen = screens.first().ok_or_else(|| "No screen found".to_string())?;
        let captured = screen.capture().map_err(|error| error.to_string())?;
        let image = DynamicImage::ImageRgba8(captured);
        let (width, height) = image.dimensions();

        let save_dir = capture_screenshot_dir(settings);
        fs::create_dir_all(&save_dir).map_err(|error| error.to_string())?;
        let created_at = Local::now();
        let file_path = save_dir.join(format!(
            "Raven Screenshot {}.png",
            created_at.format("%Y-%m-%d %H-%M-%S")
        ));
        image
            .save_with_format(&file_path, ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        let size_bytes = fs::metadata(&file_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let result = CaptureResult {
            name: file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Raven Screenshot.png")
                .to_string(),
            path: file_path.to_string_lossy().to_string(),
            width,
            height,
            size_bytes,
            created_at: created_at.to_rfc3339(),
        };

        if let Ok(mut last_result) = self.last_result.lock() {
            *last_result = Some(result.clone());
        }
        Ok(result)
    }

    pub fn capture_center_region(&self, settings: &CaptureSettings) -> Result<CaptureResult, String> {
        if !settings.enabled {
            return Err("Capture Studio is disabled in settings.".to_string());
        }

        let screens = screenshots::Screen::all().map_err(|error| error.to_string())?;
        let screen = screens.first().ok_or_else(|| "No screen found".to_string())?;
        let captured = screen.capture().map_err(|error| error.to_string())?;
        let image = DynamicImage::ImageRgba8(captured);
        let (full_width, full_height) = image.dimensions();
        let width = (full_width as f32 * 0.7).round().max(1.0) as u32;
        let height = (full_height as f32 * 0.7).round().max(1.0) as u32;
        let x = full_width.saturating_sub(width) / 2;
        let y = full_height.saturating_sub(height) / 2;
        let image = image.crop_imm(x, y, width, height);

        let save_dir = capture_screenshot_dir(settings);
        fs::create_dir_all(&save_dir).map_err(|error| error.to_string())?;
        let created_at = Local::now();
        let file_path = save_dir.join(format!(
            "Raven Region {}.png",
            created_at.format("%Y-%m-%d %H-%M-%S")
        ));
        image
            .save_with_format(&file_path, ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        let size_bytes = fs::metadata(&file_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let result = CaptureResult {
            name: file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Raven Region.png")
                .to_string(),
            path: file_path.to_string_lossy().to_string(),
            width,
            height,
            size_bytes,
            created_at: created_at.to_rfc3339(),
        };

        if let Ok(mut last_result) = self.last_result.lock() {
            *last_result = Some(result.clone());
        }
        Ok(result)
    }

    pub fn capture_region_rect(
        &self,
        settings: &CaptureSettings,
        region: CaptureRegion,
    ) -> Result<CaptureResult, String> {
        if !settings.enabled {
            return Err("Capture Studio is disabled in settings.".to_string());
        }

        let screens = screenshots::Screen::all().map_err(|error| error.to_string())?;
        let screen = screens.first().ok_or_else(|| "No screen found".to_string())?;
        let captured = screen.capture().map_err(|error| error.to_string())?;
        let image = DynamicImage::ImageRgba8(captured);
        let (full_width, full_height) = image.dimensions();
        let x = region.x.min(full_width.saturating_sub(1));
        let y = region.y.min(full_height.saturating_sub(1));
        let width = region.width.min(full_width.saturating_sub(x)).max(1);
        let height = region.height.min(full_height.saturating_sub(y)).max(1);
        let image = image.crop_imm(x, y, width, height);

        let save_dir = capture_screenshot_dir(settings);
        fs::create_dir_all(&save_dir).map_err(|error| error.to_string())?;
        let created_at = Local::now();
        let file_path = save_dir.join(format!(
            "Raven Region {}.png",
            created_at.format("%Y-%m-%d %H-%M-%S")
        ));
        image
            .save_with_format(&file_path, ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        let size_bytes = fs::metadata(&file_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let result = CaptureResult {
            name: file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Raven Region.png")
                .to_string(),
            path: file_path.to_string_lossy().to_string(),
            width,
            height,
            size_bytes,
            created_at: created_at.to_rfc3339(),
        };

        if let Ok(mut last_result) = self.last_result.lock() {
            *last_result = Some(result.clone());
        }
        Ok(result)
    }

    pub fn open_last_or_folder(&self, settings: &CaptureSettings) {
        if let Some(result) = self.last_result.lock().ok().and_then(|result| result.clone()) {
            reveal_path(&result.path);
            return;
        }
        self.open_folder(settings);
    }

    pub fn open_folder(&self, settings: &CaptureSettings) {
        let folder = capture_screenshot_dir(settings);
        open_path(&folder.to_string_lossy());
    }
}

fn capture_screenshot_dir(settings: &CaptureSettings) -> PathBuf {
    settings
        .save_screenshots_to
        .trim()
        .is_empty()
        .then(default_screenshot_dir)
        .unwrap_or_else(|| PathBuf::from(settings.save_screenshots_to.trim()))
}

fn capture_recording_dir(settings: &CaptureSettings) -> PathBuf {
    settings
        .save_recordings_to
        .trim()
        .is_empty()
        .then(default_recording_dir)
        .unwrap_or_else(|| PathBuf::from(settings.save_recordings_to.trim()))
}

fn default_screenshot_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Raven Captures")
}

fn default_recording_dir() -> PathBuf {
    dirs::video_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Raven Captures")
}

#[derive(Clone)]
pub struct StatsService {
    pub events: EventBus,
    system: Arc<Mutex<System>>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeStats {
    pub cpu_pct: f32,
    pub ram_pct: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub battery_pct: Option<f32>,
    pub on_ac_power: Option<bool>,
}

impl StatsService {
    pub fn read(&self) -> NativeStats {
        let Ok(mut system) = self.system.lock() else {
            return NativeStats::default();
        };

        system.refresh_cpu_usage();
        system.refresh_memory();

        let ram_used = system.used_memory() as f32;
        let ram_total = system.total_memory() as f32;
        let ram_pct = if ram_total > 0.0 {
            ram_used / ram_total * 100.0
        } else {
            0.0
        };
        let power = read_power_status();

        NativeStats {
            cpu_pct: system.global_cpu_usage().clamp(0.0, 100.0),
            ram_pct: ram_pct.clamp(0.0, 100.0),
            ram_used_gb: ram_used / 1_073_741_824.0,
            ram_total_gb: ram_total / 1_073_741_824.0,
            battery_pct: power.battery_pct,
            on_ac_power: power.on_ac_power,
        }
    }

    pub fn get_processes(&self) -> Vec<NativeProcessStat> {
        let Ok(mut system) = self.system.lock() else {
            return Vec::new();
        };

        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let ram_total = system.total_memory() as f32;
        let mut processes: Vec<NativeProcessStat> = system.processes().values().map(|process| {
            let name = process.name().to_string_lossy().to_string();
            let exe_path = process.exe().map(|path| path.to_string_lossy().to_string()).unwrap_or_default();
            let cpu_pct = process.cpu_usage();
            let ram_used = process.memory() as f32;
            let ram_pct = if ram_total > 0.0 {
                (ram_used / ram_total) * 100.0
            } else {
                0.0
            };
            NativeProcessStat {
                name,
                exe_path,
                cpu_pct,
                ram_pct,
                gpu_pct: 0.0,
            }
        }).collect();

        // Sort by CPU usage descending
        processes.sort_by(|a, b| b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap_or(std::cmp::Ordering::Equal));
        processes.truncate(5);

        processes
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct NativePowerStatus {
    battery_pct: Option<f32>,
    on_ac_power: Option<bool>,
}

fn read_power_status() -> NativePowerStatus {
    let mut status = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut status).is_ok() } {
        NativePowerStatus {
            battery_pct: (status.BatteryLifePercent <= 100)
                .then_some(status.BatteryLifePercent as f32),
            on_ac_power: match status.ACLineStatus {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            },
        }
    } else {
        NativePowerStatus::default()
    }
}

#[derive(Clone)]
pub struct ClockService {
    pub events: EventBus,
    state: Arc<Mutex<ClockState>>,
}

#[derive(Clone, Debug)]
struct ClockState {
    timer_duration: Duration,
    timer_remaining: Duration,
    timer_running: bool,
    timer_ends_at: Option<Instant>,
    stopwatch_base: Duration,
    stopwatch_running: bool,
    stopwatch_started_at: Option<Instant>,
    focus_goal: String,
    focus_duration: Duration,
    focus_remaining: Duration,
    focus_running: bool,
    focus_paused: bool,
    focus_ends_at: Option<Instant>,
    focus_no_limit: bool,
    focus_start_instant: Option<Instant>,
}

impl Default for ClockState {
    fn default() -> Self {
        let duration = Duration::from_secs(25 * 60 + 30);
        Self {
            timer_duration: duration,
            timer_remaining: duration,
            timer_running: false,
            timer_ends_at: None,
            stopwatch_base: Duration::ZERO,
            stopwatch_running: false,
            stopwatch_started_at: None,
            focus_goal: "Focus session".to_string(),
            focus_duration: Duration::from_secs(30 * 60),
            focus_remaining: Duration::from_secs(30 * 60),
            focus_running: false,
            focus_paused: false,
            focus_ends_at: None,
            focus_no_limit: false,
            focus_start_instant: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NativeClock {
    pub timer_duration_seconds: u64,
    pub timer_remaining_seconds: u64,
    pub timer_running: bool,
    pub stopwatch_ms: u128,
    pub stopwatch_running: bool,
    pub focus_goal: String,
    pub focus_duration_seconds: u64,
    pub focus_remaining_seconds: u64,
    pub focus_running: bool,
    pub focus_paused: bool,
    pub focus_no_limit: bool,
}

impl Default for NativeClock {
    fn default() -> Self {
        Self {
            timer_duration_seconds: 25 * 60 + 30,
            timer_remaining_seconds: 25 * 60 + 30,
            timer_running: false,
            stopwatch_ms: 0,
            stopwatch_running: false,
            focus_goal: "Focus session".to_string(),
            focus_duration_seconds: 30 * 60,
            focus_remaining_seconds: 30 * 60,
            focus_running: false,
            focus_paused: false,
            focus_no_limit: false,
        }
    }
}

impl NativeClock {
    pub fn focus_remaining_label(&self) -> String {
        let minutes = self.focus_remaining_seconds / 60;
        let seconds = self.focus_remaining_seconds % 60;
        format!("{minutes:02}:{seconds:02}")
    }

    pub fn timer_label(&self) -> String {
        let hours = self.timer_remaining_seconds / 3600;
        let minutes = (self.timer_remaining_seconds % 3600) / 60;
        let seconds = self.timer_remaining_seconds % 60;
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }

    pub fn stopwatch_label(&self) -> String {
        let total_seconds = self.stopwatch_ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        let centis = (self.stopwatch_ms % 1000) / 10;
        format!("{minutes:02}:{seconds:02}.{centis:02}")
    }
}

impl ClockService {
    pub fn read(&self) -> NativeClock {
        let Ok(mut state) = self.state.lock() else {
            return NativeClock::default();
        };
        sync_clock_state(&mut state);
        clock_snapshot(&state)
    }

    pub fn start_focus_session(&self, goal: String, duration_str: String) {
        let Ok(mut state) = self.state.lock() else { return; };
        let (duration, no_limit) = parse_duration_string(&duration_str).unwrap_or((Duration::from_secs(30 * 60), false));
        state.focus_goal = goal;
        state.focus_no_limit = no_limit;
        if no_limit {
            state.focus_duration = Duration::ZERO;
            state.focus_remaining = Duration::ZERO;
            state.focus_start_instant = Some(Instant::now());
            state.focus_ends_at = None;
        } else {
            state.focus_duration = duration;
            state.focus_remaining = duration;
            state.focus_start_instant = None;
            state.focus_ends_at = Some(Instant::now() + duration);
        }
        state.focus_running = true;
        state.focus_paused = false;
    }

    pub fn stop_focus_session(&self) {
        let Ok(mut state) = self.state.lock() else { return; };
        state.focus_running = false;
        state.focus_paused = false;
        state.focus_ends_at = None;
        state.focus_start_instant = None;
    }

    pub fn toggle_pause_focus_session(&self) {
        let Ok(mut state) = self.state.lock() else { return; };
        if state.focus_running {
            state.focus_running = false;
            state.focus_paused = true;
            state.focus_ends_at = None;
        } else if state.focus_paused {
            if state.focus_no_limit {
                state.focus_start_instant = Some(Instant::now() - state.focus_remaining);
            } else {
                state.focus_ends_at = Some(Instant::now() + state.focus_remaining);
            }
            state.focus_running = true;
            state.focus_paused = false;
        }
    }

    pub fn complete_focus_session(&self) {
        let Ok(mut state) = self.state.lock() else { return; };
        state.focus_remaining = Duration::ZERO;
        state.focus_running = false;
        state.focus_paused = false;
        state.focus_ends_at = None;
        state.focus_start_instant = None;
    }

    pub fn toggle_timer(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        sync_clock_state(&mut state);
        if state.focus_running || state.focus_paused {
            state.focus_running = false;
            state.focus_paused = false;
            state.focus_ends_at = None;
            return;
        }
        if state.timer_running {
            state.timer_running = false;
            state.timer_ends_at = None;
            return;
        }

        let remaining = if state.timer_remaining.is_zero() {
            state.timer_duration
        } else {
            state.timer_remaining
        };
        state.timer_remaining = remaining;
        state.timer_ends_at = Some(Instant::now() + remaining);
        state.timer_running = true;
    }

    pub fn reset_timer(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.focus_running || state.focus_paused {
            state.focus_running = false;
            state.focus_paused = false;
            state.focus_ends_at = None;
            return;
        }
        state.timer_remaining = state.timer_duration;
        state.timer_running = false;
        state.timer_ends_at = None;
    }

    pub fn set_timer_duration(&self, secs: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.timer_duration = Duration::from_secs(secs);
        state.timer_remaining = Duration::from_secs(secs);
        state.timer_running = false;
        state.timer_ends_at = None;
    }

    pub fn set_timer_remaining(&self, secs: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.timer_remaining = Duration::from_secs(secs);
        if state.timer_running {
            state.timer_ends_at = Some(std::time::Instant::now() + state.timer_remaining);
        }
    }

    pub fn toggle_stopwatch(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.stopwatch_running {
            if let Some(started_at) = state.stopwatch_started_at {
                state.stopwatch_base += Instant::now().saturating_duration_since(started_at);
            }
            state.stopwatch_running = false;
            state.stopwatch_started_at = None;
            return;
        }
        state.stopwatch_started_at = Some(Instant::now());
        state.stopwatch_running = true;
    }

    pub fn reset_stopwatch(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.stopwatch_base = Duration::ZERO;
        state.stopwatch_running = false;
        state.stopwatch_started_at = None;
    }
}

fn sync_clock_state(state: &mut ClockState) {
    let now = Instant::now();
    if state.timer_running {
        if let Some(end) = state.timer_ends_at {
            if end <= now {
                state.timer_remaining = Duration::ZERO;
                state.timer_running = false;
                state.timer_ends_at = None;
            } else {
                state.timer_remaining = end - now;
            }
        }
    }

    // Stopwatch elapsed time is calculated dynamically in clock_snapshot to prevent delta accumulation jitters

    if state.focus_running {
        if state.focus_no_limit {
            if let Some(start) = state.focus_start_instant {
                state.focus_remaining = now.duration_since(start);
            }
        } else if let Some(end) = state.focus_ends_at {
            if end <= now {
                state.focus_remaining = Duration::ZERO;
                state.focus_running = false;
                state.focus_ends_at = None;
            } else {
                state.focus_remaining = end - now;
            }
        }
    }
}

fn clock_snapshot(state: &ClockState) -> NativeClock {
    let stopwatch_ms = if state.stopwatch_running {
        if let Some(started_at) = state.stopwatch_started_at {
            (state.stopwatch_base + Instant::now().saturating_duration_since(started_at)).as_millis()
        } else {
            state.stopwatch_base.as_millis()
        }
    } else {
        state.stopwatch_base.as_millis()
    };

    NativeClock {
        timer_duration_seconds: state.timer_duration.as_secs(),
        timer_remaining_seconds: state.timer_remaining.as_secs(),
        timer_running: state.timer_running,
        stopwatch_ms,
        stopwatch_running: state.stopwatch_running,
        focus_goal: state.focus_goal.clone(),
        focus_duration_seconds: state.focus_duration.as_secs(),
        focus_remaining_seconds: state.focus_remaining.as_secs(),
        focus_running: state.focus_running,
        focus_paused: state.focus_paused,
        focus_no_limit: state.focus_no_limit,
    }
}

#[derive(Clone)]
pub struct ShortcutService {
    pub events: EventBus,
}

#[derive(Clone)]
pub struct ShelfService {
    pub events: EventBus,
    items: Arc<Mutex<Vec<ShelfItem>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShelfItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_image: bool,
    pub is_video: bool,
}

impl ShelfService {
    pub fn items(&self) -> Vec<ShelfItem> {
        self.items
            .lock()
            .map(|items| items.clone())
            .unwrap_or_default()
    }

    pub fn add_paths(&self, paths: Vec<String>) {
        let Ok(mut items) = self.items.lock() else {
            return;
        };

        let settings = crate::settings::RavenSettings::load();
        let keep_max = settings.drop.keep_max.max(1) as usize;

        let mut incoming = paths
            .into_iter()
            .filter_map(|path| shelf_item_from_path(&path))
            .collect::<Vec<_>>();
        incoming.extend(items.clone());

        let mut seen = std::collections::HashSet::new();
        let mut merged = Vec::new();
        for item in incoming {
            let key = item.path.to_lowercase();
            if seen.insert(key) {
                merged.push(item);
            }
            if merged.len() >= keep_max {
                break;
            }
        }

        *items = merged;
        persist_shelf_items(&items);
    }

    pub fn open_first(&self) {
        if let Some(item) = self.items().first() {
            open_path(&item.path);
        }
    }

    pub fn reveal_first(&self) {
        if let Some(item) = self.items().first() {
            reveal_path(&item.path);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut items) = self.items.lock() {
            items.clear();
            persist_shelf_items(&items);
        }
    }

    pub fn remove_item(&self, id: &str) {
        if let Ok(mut items) = self.items.lock() {
            items.retain(|item| item.id != id);
            persist_shelf_items(&items);
        }
    }

    pub fn share_file(&self, path: String, provider: String) {
        let path = path.trim_matches('"').to_string();
        let provider = provider.to_lowercase();
        
        std::thread::spawn(move || {
            set_share_notice("sending", "Opening share application...");
            let res = match provider.as_str() {
                "localsend" => {
                    open_localsend_with_file(path).map(|_| "Opened LocalSend".to_string())
                }
                "quickshare" => {
                    open_quick_share_with_file(path).map(|_| "Opened Quick Share".to_string())
                }
                "kdeconnect" => {
                    set_share_notice("sending", "Sending via KDE Connect...");
                    handoff_kdeconnect_background(path);
                    return;
                }
                _ => {
                    reveal_path(&path);
                    Ok("Revealed in Explorer".to_string())
                }
            };
            
            match res {
                Ok(msg) => set_share_notice("sent", &msg),
                Err(err) => set_share_notice("error", &err),
            }
        });
    }
}

fn shelf_item_from_path(path: &str) -> Option<ShelfItem> {
    let path = path.trim_matches('"').to_string();
    if path.is_empty() {
        return None;
    }
    let name = Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("File")
        .to_string();
    let size = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    let lower = name.to_lowercase();
    Some(ShelfItem {
        id: format!("file://{}", path.to_lowercase()),
        name,
        path,
        size,
        is_image: matches!(
            Path::new(&lower).extension().and_then(|ext| ext.to_str()),
            Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "ico")
        ),
        is_video: matches!(
            Path::new(&lower).extension().and_then(|ext| ext.to_str()),
            Some("mp4" | "mov" | "m4v" | "webm" | "avi" | "mkv")
        ),
    })
}

fn shelf_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RavenIsland")
        .join("native-shelf.json")
}

fn load_shelf_items() -> Vec<ShelfItem> {
    let path = shelf_path();
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<ShelfItem>>(&raw).unwrap_or_default()
}

fn persist_shelf_items(items: &[ShelfItem]) {
    let path = shelf_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(items) {
        let _ = fs::write(path, raw);
    }
}

fn set_share_notice(state: &str, message: &str) {
    if let Some(weak_ui) = crate::window::PILL_UI_WEAK.get() {
        let state = state.to_string();
        let message = message.to_string();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak_ui.upgrade() {
                ui.set_share_notice_state(state.into());
                ui.set_share_notice_message(message.into());
            }
        });
    }
}

static KDECONNECT_LAST_DEVICE: std::sync::OnceLock<std::sync::Mutex<Option<String>>> = std::sync::OnceLock::new();

fn kdeconnect_cached_device() -> Option<String> {
    KDECONNECT_LAST_DEVICE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .ok()
        .and_then(|device| device.clone())
}

fn kdeconnect_set_cached_device(device_id: Option<String>) {
    if let Ok(mut cache) = KDECONNECT_LAST_DEVICE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
    {
        *cache = device_id;
    }
}

fn first_app_candidate(candidates: Vec<String>) -> Option<String> {
    candidates
        .into_iter()
        .find(|candidate| app_candidate_exists(candidate))
}

fn kdeconnect_first_available_device(candidate: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;

    let output = std::process::Command::new(candidate)
        .args(["--list-available", "--id-only"])
        .creation_flags(0x08000000)
        .output()
        .ok()?;

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn kdeconnect_send_to_device(candidate: &str, clean_path: &str, device_id: &str) -> bool {
    use std::os::windows::process::CommandExt;

    std::process::Command::new(candidate)
        .args(["--share", clean_path, "--device", device_id])
        .creation_flags(0x08000000)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn kdeconnect_cli_candidates() -> Vec<String> {
    let mut candidates = vec![
        "kdeconnect-cli.exe".to_string(),
        "kdeconnect-cli".to_string(),
    ];

    #[cfg(target_os = "windows")]
    {
        for root in [
            std::env::var("LOCALAPPDATA").unwrap_or_default(),
            std::env::var("ProgramFiles").unwrap_or_default(),
            std::env::var("ProgramFiles(x86)").unwrap_or_default(),
        ] {
            if root.is_empty() {
                continue;
            }

            for relative in [
                ["KDE Connect", "bin", "kdeconnect-cli.exe"].as_slice(),
                ["KDE Connect", "kdeconnect-cli.exe"].as_slice(),
                ["Programs", "KDE Connect", "bin", "kdeconnect-cli.exe"].as_slice(),
                ["Programs", "KDE Connect", "kdeconnect-cli.exe"].as_slice(),
            ] {
                let candidate = relative
                    .iter()
                    .fold(Path::new(&root).to_path_buf(), |path, part| path.join(part));
                candidates.push(candidate.to_string_lossy().to_string());
            }
        }
    }

    candidates
}

fn kdeconnect_app_candidates() -> Vec<String> {
    let mut candidates = vec![
        "kdeconnect-app.exe".to_string(),
        "kdeconnect-app".to_string(),
        "kdeconnect-indicator.exe".to_string(),
        "kdeconnect-indicator".to_string(),
    ];

    #[cfg(target_os = "windows")]
    {
        for root in [
            std::env::var("LOCALAPPDATA").unwrap_or_default(),
            std::env::var("ProgramFiles").unwrap_or_default(),
            std::env::var("ProgramFiles(x86)").unwrap_or_default(),
        ] {
            if root.is_empty() {
                continue;
            }

            for binary in [
                "kdeconnect-app.exe",
                "kdeconnect-indicator.exe",
                "kdeconnect-settings.exe",
            ] {
                for relative in [
                    ["KDE Connect", "bin", binary].as_slice(),
                    ["KDE Connect", binary].as_slice(),
                    ["Programs", "KDE Connect", "bin", binary].as_slice(),
                    ["Programs", "KDE Connect", binary].as_slice(),
                ] {
                    let candidate = relative
                        .iter()
                        .fold(Path::new(&root).to_path_buf(), |path, part| path.join(part));
                    candidates.push(candidate.to_string_lossy().to_string());
                }
            }
        }
    }

    candidates
}

fn handoff_kdeconnect_background(clean_path: String) {
    use std::os::windows::process::CommandExt;

    set_share_notice("sending", "Sending via KDE Connect...");

    let cli_candidate = first_app_candidate(kdeconnect_cli_candidates());
    let app_candidate = first_app_candidate(kdeconnect_app_candidates());

    if cli_candidate.is_none() && app_candidate.is_none() {
        set_share_notice(
            "error",
            "KDE Connect is not installed on your system.",
        );
        return;
    }

    if let Some(candidate) = cli_candidate {
        if let Some(device_id) = kdeconnect_cached_device() {
            if kdeconnect_send_to_device(&candidate, &clean_path, &device_id) {
                set_share_notice("sent", "File sent via KDE Connect!");
                return;
            }
            kdeconnect_set_cached_device(None);
        }

        let Some(device_id) = kdeconnect_first_available_device(&candidate) else {
            set_share_notice(
                "error",
                "No KDE Connect device is available. Make sure it is paired and online.",
            );
            return;
        };

        kdeconnect_set_cached_device(Some(device_id.clone()));

        if kdeconnect_send_to_device(&candidate, &clean_path, &device_id) {
            set_share_notice("sent", "File sent via KDE Connect!");
            return;
        }

        kdeconnect_set_cached_device(None);
        set_share_notice("error", "KDE Connect send failed.");
        return;
    }

    if let Some(candidate) = app_candidate {
        if std::process::Command::new(candidate)
            .creation_flags(0x08000000)
            .spawn()
            .is_ok()
        {
            let escaped = clean_path.replace('\'', "''");
            let _ = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-WindowStyle",
                    "Hidden",
                    "-Command",
                    &format!("Set-Clipboard -Value '{}'", escaped),
                ])
                .creation_flags(0x08000000)
                .status();
            set_share_notice(
                "error",
                "KDE Connect opened. Pair your device and try again.",
            );
            return;
        }
    }

    set_share_notice("error", "KDE Connect send failed.");
}

fn open_localsend_with_file(path: String) -> Result<(), String> {
    let clean_path = path.trim_matches('"').to_string();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let mut candidates = vec![
            "localsend_app.exe".to_string(),
            "localsend_app".to_string(),
            "localsend.exe".to_string(),
            "localsend".to_string(),
            "LocalSend.exe".to_string(),
            "LocalSend".to_string(),
        ];

        for root in [
            std::env::var("LOCALAPPDATA").unwrap_or_default(),
            std::env::var("ProgramFiles").unwrap_or_default(),
            std::env::var("ProgramFiles(x86)").unwrap_or_default(),
        ] {
            if root.is_empty() {
                continue;
            }

            for relative in [
                ["Programs", "LocalSend", "localsend_app.exe"].as_slice(),
                ["Programs", "LocalSend", "localsend.exe"].as_slice(),
                ["Programs", "LocalSend", "LocalSend.exe"].as_slice(),
                ["LocalSend", "localsend_app.exe"].as_slice(),
                ["LocalSend", "localsend.exe"].as_slice(),
                ["LocalSend", "LocalSend.exe"].as_slice(),
            ] {
                let candidate = relative
                    .iter()
                    .fold(Path::new(&root).to_path_buf(), |path, part| path.join(part));
                if candidate.exists() {
                    candidates.push(candidate.to_string_lossy().to_string());
                }
            }
        }

        for candidate in candidates {
            if !app_candidate_exists(&candidate) {
                continue;
            }

            if std::process::Command::new(&candidate)
                .arg(&clean_path)
                .creation_flags(0x08000000)
                .spawn()
                .is_ok()
            {
                return Ok(());
            }
        }

        if windows_protocol_exists("localsend")
            && std::process::Command::new("cmd")
                .args(["/C", "start", "", "localsend:", &clean_path])
                .creation_flags(0x08000000)
                .spawn()
                .is_ok()
        {
            return Ok(());
        }

        Err("LocalSend is not installed on your system.".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = clean_path;
        Err("LocalSend app handoff is currently supported on Windows only.".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSendPeer {
    pub alias: String,
    pub url: String,
    pub ip: String,
    pub port: u16,
    pub protocol: String,
    #[serde(default)]
    pub device_model: String,
    #[serde(default)]
    pub fingerprint: String,
}

#[derive(Debug, Deserialize)]
struct LocalSendDiscoveryPacket {
    #[serde(default)]
    alias: String,
    #[serde(default)]
    port: u16,
    #[serde(default)]
    protocol: String,
    #[serde(default)]
    device_model: String,
    #[serde(default)]
    fingerprint: String,
}

fn command_exists(command: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("where")
            .arg(command)
            .creation_flags(0x08000000)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

fn app_candidate_exists(candidate: &str) -> bool {
    let path = Path::new(candidate);
    path.exists() || command_exists(candidate)
}

fn common_windows_app_exists(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let program_files = std::env::var("ProgramFiles").unwrap_or_default();
        let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
        [local, program_files, program_files_x86]
            .iter()
            .filter(|root| !root.is_empty())
            .any(|root| Path::new(root).join(name).exists())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
        false
    }
}

fn windows_protocol_exists(protocol: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("reg")
            .args(["query", &format!("HKCR\\{protocol}")])
            .creation_flags(0x08000000)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = protocol;
        false
    }
}

fn open_quick_share_with_file(path: String) -> Result<(), String> {
    let clean_path = path.trim_matches('"').to_string();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let mut candidates = vec![
            "nearby_share.exe".to_string(),
            "nearby_share".to_string(),
            "QuickShare.exe".to_string(),
            "QuickShare".to_string(),
        ];

        for root in [
            std::env::var("ProgramFiles").unwrap_or_default(),
            std::env::var("ProgramFiles(x86)").unwrap_or_default(),
            std::env::var("LOCALAPPDATA").unwrap_or_default(),
        ] {
            if root.is_empty() {
                continue;
            }

            for relative in [
                ["Google", "NearbyShare", "nearby_share.exe"].as_slice(),
                ["Google", "QuickShare", "nearby_share.exe"].as_slice(),
                ["Google", "Quick Share", "nearby_share.exe"].as_slice(),
                ["Quick Share", "QuickShare.exe"].as_slice(),
                ["QuickShare", "QuickShare.exe"].as_slice(),
            ] {
                let candidate = relative
                    .iter()
                    .fold(Path::new(&root).to_path_buf(), |path, part| path.join(part));
                if candidate.exists() {
                    candidates.push(candidate.to_string_lossy().to_string());
                }
            }
        }

        for candidate in candidates {
            if !app_candidate_exists(&candidate) {
                continue;
            }

            if std::process::Command::new(&candidate)
                .arg(&clean_path)
                .creation_flags(0x08000000)
                .spawn()
                .is_ok()
            {
                return Ok(());
            }
        }

        Err("Quick Share is not installed on your system.".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = clean_path;
        Err("Quick Share handoff is currently supported on Windows only.".to_string())
    }
}

fn open_bluetooth_transfer(path: String) -> Result<(), String> {
    let clean_path = path.trim_matches('"').to_string();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        if !app_candidate_exists("fsquirt") && !app_candidate_exists("fsquirt.exe") {
            return Err("Bluetooth File Transfer is not installed on your system.".to_string());
        }

        std::process::Command::new("fsquirt")
            .arg("-send")
            .arg(&clean_path)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = clean_path;
        return Err("Bluetooth sharing is currently supported on Windows only.".to_string());
    }

    Ok(())
}

fn discover_localsend_peers(timeout: Duration) -> Result<Vec<LocalSendPeer>, String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_millis(220)))
        .map_err(|e| e.to_string())?;
    socket.set_multicast_ttl_v4(1).map_err(|e| e.to_string())?;

    let announcement = serde_json::json!({
        "alias": "Raven",
        "version": "2.0",
        "deviceModel": "Windows",
        "deviceType": "desktop",
        "fingerprint": "raven-notch",
        "port": 53317,
        "protocol": "http",
        "download": false,
        "announce": true,
    })
    .to_string();

    let _ = socket.send_to(announcement.as_bytes(), "224.0.0.167:53317");

    let started = std::time::Instant::now();
    let mut buf = [0_u8; 4096];
    let mut peers: std::collections::HashMap<String, LocalSendPeer> = std::collections::HashMap::new();

    while started.elapsed() < timeout {
        match socket.recv_from(&mut buf) {
            Ok((len, sender)) => {
                if let Some(peer) = parse_localsend_peer(&buf[..len], sender) {
                    peers.insert(peer.url.clone(), peer);
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock || error.kind() == std::io::ErrorKind::TimedOut => {
            }
            Err(_) => break,
        }
    }

    let mut list: Vec<LocalSendPeer> = peers.into_values().collect();
    list.sort_by(|a, b| a.alias.to_lowercase().cmp(&b.alias.to_lowercase()));
    Ok(list)
}

fn parse_localsend_peer(bytes: &[u8], sender: std::net::SocketAddr) -> Option<LocalSendPeer> {
    let packet: LocalSendDiscoveryPacket = serde_json::from_slice(bytes).ok()?;
    let port = if packet.port > 0 {
        packet.port
    } else {
        sender.port()
    };
    let protocol = if packet.protocol.eq_ignore_ascii_case("https") {
        "https"
    } else {
        "http"
    };
    let ip = sender.ip().to_string();
    let alias = if packet.alias.trim().is_empty() {
        ip.clone()
    } else {
        packet.alias
    };

    Some(LocalSendPeer {
        alias,
        url: format!("{protocol}://{ip}:{port}"),
        ip,
        port,
        protocol: protocol.to_string(),
        device_model: packet.device_model,
        fingerprint: packet.fingerprint,
    })
}

fn send_file_to_localsend(path: &str, target_url: &str) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("LocalSend can only send files in this Raven build.".to_string());
    }

    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file")
        .to_string();
    let file_id = format!("raven-{}", now_millis());
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .build();
    let base = target_url.trim_end_matches('/');

    let prepare_payload = serde_json::json!({
        "info": {
            "alias": "Raven",
            "version": "2.0",
            "deviceModel": "Windows",
            "deviceType": "desktop",
            "fingerprint": "raven-notch",
        },
        "files": {
            file_id.clone(): {
                "id": file_id,
                "fileName": file_name,
                "size": metadata.len(),
                "fileType": file_type_for(path),
            }
        }
    });

    let prepare_url = format!("{base}/api/localsend/v2/prepare-upload");
    let prepare_response: serde_json::Value = agent
        .post(&prepare_url)
        .set("Content-Type", "application/json")
        .send_json(prepare_payload)
        .map_err(|e| format!("LocalSend prepare-upload failed: {e}"))?
        .into_json()
        .map_err(|e| format!("LocalSend prepare-upload response failed: {e}"))?;

    let session_id = prepare_response
        .get("sessionId")
        .or_else(|| prepare_response.get("session_id"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| "LocalSend did not return a session id.".to_string())?;
    let token = prepare_response
        .get("files")
        .and_then(|files| files.get(&file_id))
        .and_then(|file| file.get("token"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| "LocalSend did not return an upload token.".to_string())?;

    let upload_url = format!(
        "{base}/api/localsend/v2/upload?sessionId={}&fileId={}&token={}",
        percent_encode(session_id),
        percent_encode(&file_id),
        percent_encode(token)
    );

    agent
        .post(&upload_url)
        .set("Content-Type", "application/octet-stream")
        .send_bytes(&bytes)
        .map_err(|e| format!("LocalSend upload failed: {e}"))?;

    Ok(())
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn file_type_for(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" => "image",
        "mp4" | "mov" | "m4v" | "webm" | "avi" | "mkv" => "video",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" => "audio",
        _ => "other",
    }
}

fn percent_encode(input: &str) -> String {
    input
        .bytes()
        .flat_map(|byte| {
            let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
            if keep {
                vec![byte as char]
            } else {
                format!("%{byte:02X}").chars().collect()
            }
        })
        .collect()
}


fn open_path(path: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .creation_flags(0x08000000)
            .spawn();
    }
}

fn reveal_path(path: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{path}"))
            .creation_flags(0x08000000)
            .spawn();
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq)]
pub struct GoogleCalendarEntry {
    pub id: String,
    pub summary: String,
    pub primary: Option<bool>,
}

#[derive(Clone)]
pub struct CalendarService {
    pub events: EventBus,
    items: Arc<Mutex<Vec<CalendarEvent>>>,
    last_message: Arc<Mutex<String>>,
    pub google_calendars: Arc<Mutex<Vec<GoogleCalendarEntry>>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalendarEvent {
    pub title: String,
    pub date_str: String,
    pub timestamp: i64,
}

#[derive(Clone, Debug)]
pub struct NativeCalendarStatus {
    pub source: String,
    pub google_connected: bool,
    pub google_email: String,
    pub selected_google_calendars: usize,
    pub google_calendars: Vec<GoogleCalendarEntry>,
    pub items: Vec<CalendarEvent>,
    pub message: String,
}

impl Default for NativeCalendarStatus {
    fn default() -> Self {
        Self {
            source: "none".to_string(),
            google_connected: false,
            google_email: String::new(),
            selected_google_calendars: 0,
            google_calendars: Vec::new(),
            items: Vec::new(),
            message: "No calendar source configured".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
struct GoogleCalendarToken {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    expires_at: i64,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    last_synced: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const GOOGLE_SCOPE: &str = "openid email https://www.googleapis.com/auth/calendar.readonly";
const GOOGLE_CLIENT_ID: &str = "300613359057-makmtm2umo6bh29pqr5ohgk7gtg7eoio.apps.googleusercontent.com";

fn google_client_secret() -> String {
    std::env::var("RAVEN_GOOGLE_CLIENT_SECRET").unwrap_or_default()
}

impl CalendarService {
    pub fn read(&self, settings: &MediaSettings) -> NativeCalendarStatus {
        let mut message = "Ready".to_string();
        let mut items = self.items.lock().map(|items| items.clone()).unwrap_or_default();
        let google_connected = google_calendar_status().0;
        
        let mut google_calendars = self.google_calendars.lock().map(|c| c.clone()).unwrap_or_default();
        if google_connected && google_calendars.is_empty() {
            if let Ok(cals) = fetch_google_calendars() {
                google_calendars = cals;
                if let Ok(mut stored) = self.google_calendars.lock() {
                    *stored = google_calendars.clone();
                }
            }
        }

        let source = if google_connected {
            let calendar_ids = if !settings.google_calendar_ids.is_empty() {
                settings.google_calendar_ids.clone()
            } else {
                vec!["primary".to_string()]
            };
            
            match fetch_google_events(&calendar_ids) {
                Ok(next_items) => {
                    items = next_items;
                    if let Ok(mut stored) = self.items.lock() {
                        *stored = items.clone();
                    }
                    "google".to_string()
                }
                Err(error) => {
                    message = format!("Google refresh failed: {error}");
                    "google".to_string()
                }
            }
        } else if !settings.calendar_url.trim().is_empty() {
            match fetch_ics_events(settings.calendar_url.trim()) {
                Ok(next_items) => {
                    items = next_items;
                    if let Ok(mut stored) = self.items.lock() {
                        *stored = items.clone();
                    }
                    "ics".to_string()
                }
                Err(error) => {
                    message = format!("ICS refresh failed: {error}");
                    "ics".to_string()
                }
            }
        } else {
            message = "No calendar source configured".to_string();
            "none".to_string()
        };

        if let Ok(mut stored_message) = self.last_message.lock() {
            *stored_message = message.clone();
        }

        let google = google_calendar_status();
        NativeCalendarStatus {
            source,
            google_connected: google.0,
            google_email: google.1,
            selected_google_calendars: if settings.google_calendar_ids.is_empty() && google.0 { 1 } else { settings.google_calendar_ids.len() },
            google_calendars,
            items,
            message,
        }
    }

    pub fn connect_and_read(&self, settings: &MediaSettings) -> NativeCalendarStatus {
        match google_calendar_connect_blocking() {
            Ok(()) => {
                let mut status = self.read(settings);
                if status.google_connected {
                    status.message = "Google Calendar connected.".to_string();
                }
                status
            }
            Err(error) => {
                let mut status = self.read(settings);
                status.google_connected = false;
                status.message = error;
                status
            }
        }
    }
}

#[derive(Clone)]
pub struct CaffeineService {
    pub events: EventBus,
    state: Arc<Mutex<CaffeineRuntime>>,
}

#[derive(Clone, Debug)]
pub struct NativeCaffeineStatus {
    pub enabled: bool,
    pub keep_screen_awake: bool,
}

impl Default for NativeCaffeineStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            keep_screen_awake: false,
        }
    }
}

#[derive(Debug, Default)]
struct CaffeineRuntime {
    status: NativeCaffeineStatus,
    stop_flag: Option<Arc<AtomicBool>>,
}

impl CaffeineService {
    pub fn toggle(&self) -> NativeCaffeineStatus {
        let enabled = self
            .state
            .lock()
            .map(|runtime| runtime.status.enabled)
            .unwrap_or(false);
        if enabled {
            self.stop()
        } else {
            self.start(true)
        }
    }

    pub fn start(&self, keep_screen_awake: bool) -> NativeCaffeineStatus {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let thread_flag = stop_flag.clone();
        if let Ok(mut runtime) = self.state.lock() {
            if let Some(existing) = runtime.stop_flag.take() {
                existing.store(true, Ordering::SeqCst);
            }
            runtime.status = NativeCaffeineStatus {
                enabled: true,
                keep_screen_awake,
            };
            runtime.stop_flag = Some(stop_flag);
        }

        std::thread::spawn(move || {
            while !thread_flag.load(Ordering::SeqCst) {
                apply_keep_awake(keep_screen_awake);
                std::thread::sleep(Duration::from_secs(25));
            }
            clear_keep_awake();
        });

        self.state
            .lock()
            .map(|runtime| runtime.status.clone())
            .unwrap_or_default()
    }

    fn stop(&self) -> NativeCaffeineStatus {
        if let Ok(mut runtime) = self.state.lock() {
            if let Some(existing) = runtime.stop_flag.take() {
                existing.store(true, Ordering::SeqCst);
            }
            runtime.status = NativeCaffeineStatus::default();
        }
        clear_keep_awake();
        NativeCaffeineStatus::default()
    }
}

fn apply_keep_awake(keep_screen_awake: bool) {
    let mut state = ES_CONTINUOUS | ES_SYSTEM_REQUIRED;
    if keep_screen_awake {
        state |= ES_DISPLAY_REQUIRED;
    }
    unsafe {
        SetThreadExecutionState(state);
    }
}

fn clear_keep_awake() {
    unsafe {
        SetThreadExecutionState(ES_CONTINUOUS);
    }
}

#[derive(Clone, Copy, Debug)]
enum VolumeKey {
    Down,
    Mute,
    Up,
}

fn send_volume_key(key: VolumeKey) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYEVENTF_KEYUP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
    };
    let vk = match key {
        VolumeKey::Down => VK_VOLUME_DOWN.0 as u8,
        VolumeKey::Mute => VK_VOLUME_MUTE.0 as u8,
        VolumeKey::Up => VK_VOLUME_UP.0 as u8,
    };
    unsafe {
        keybd_event(vk, 0, Default::default(), 0);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
}

fn adjust_brightness(delta: i32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let script = format!(
            "$b=(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue | Select-Object -First 1).CurrentBrightness; if ($null -ne $b) {{ $n=[Math]::Max(0,[Math]::Min(100,$b+({delta}))); Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue | ForEach-Object {{ $_.WmiSetBrightness(1,$n) | Out-Null }} }}"
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .creation_flags(0x08000000)
            .spawn();
    }
}

fn fetch_ics_events(url: &str) -> Result<Vec<CalendarEvent>, String> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(5))
        .call()
        .map_err(|error| error.to_string())?;
    let ics_data = response.into_string().map_err(|error| error.to_string())?;
    Ok(parse_ics_events(&ics_data, Local::now().format("%Y%m%d%H%M%S").to_string().parse().unwrap_or(0)))
}

fn parse_ics_events(ics_data: &str, now_int: i64) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    let mut current: Option<CalendarEvent> = None;
    for raw_line in ics_data.lines() {
        let line = raw_line.trim();
        if line.starts_with("BEGIN:VEVENT") {
            current = Some(CalendarEvent::default());
            continue;
        }
        if line.starts_with("END:VEVENT") {
            if let Some(event) = current.take() {
                if !event.title.is_empty() && event.timestamp >= now_int {
                    events.push(event);
                }
            }
            continue;
        }
        let Some(event) = current.as_mut() else {
            continue;
        };
        if let Some(summary) = line.strip_prefix("SUMMARY:") {
            event.title = summary.replace("\\,", ",").replace("\\n", " ");
        } else if line.starts_with("DTSTART") {
            if let Some(date_val) = line.split(':').nth(1) {
                event.timestamp = parse_ical_date(date_val);
                event.date_str = format_ical_date(date_val);
            }
        }
    }
    events.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    events.truncate(250);
    events
}

fn parse_ical_date(date_val: &str) -> i64 {
    date_val
        .replace('T', "")
        .replace('Z', "")
        .replace(' ', "")
        .get(0..14)
        .unwrap_or("")
        .parse::<i64>()
        .unwrap_or(0)
}

fn format_ical_date(date_val: &str) -> String {
    if date_val.len() < 8 {
        return date_val.to_string();
    }
    let month = match &date_val[4..6] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => "",
    };
    let day = &date_val[6..8];
    if date_val.len() >= 15 && date_val.contains('T') {
        let t_index = date_val.find('T').unwrap_or(8);
        let hour = date_val
            .get(t_index + 1..t_index + 3)
            .and_then(|hour| hour.parse::<u8>().ok())
            .unwrap_or(0);
        let minute = date_val.get(t_index + 3..t_index + 5).unwrap_or("00");
        let ampm = if hour >= 12 { "PM" } else { "AM" };
        let hour12 = if hour == 0 {
            12
        } else if hour > 12 {
            hour - 12
        } else {
            hour
        };
        return format!("{month} {day}, {hour12}:{minute} {ampm}");
    }
    format!("{month} {day}")
}

fn google_calendar_status() -> (bool, String) {
    let path = google_token_path().unwrap_or_else(|_| PathBuf::from("google_calendar.json"));
    let Ok(raw) = fs::read_to_string(path) else {
        return (false, String::new());
    };
    let Ok(token) = serde_json::from_str::<GoogleCalendarToken>(&raw) else {
        return (false, String::new());
    };
    (
        !token.refresh_token.is_empty(),
        token.email.unwrap_or_else(|| "Google Calendar".to_string()),
    )
}

fn fetch_google_events(calendar_ids: &[String]) -> Result<Vec<CalendarEvent>, String> {
    let mut token = load_google_calendar_token()?.ok_or_else(|| "Google Calendar is not connected".to_string())?;
    refresh_google_token_if_needed(&mut token)?;
    if token.access_token.trim().is_empty() {
        return Err("Google access token is missing; reconnect calendar in settings".to_string());
    }

    let now = Local::now();
    let time_min = (now - chrono::Duration::days(180)).to_rfc3339();
    let time_max = (now + chrono::Duration::days(185)).to_rfc3339();
    let mut events = Vec::new();
    for calendar_id in calendar_ids.iter().take(8) {
        if calendar_id == "virtual_tasks" {
            continue;
        }
        let is_birthday = calendar_id == "virtual_birthdays";
        let query_calendar_id = if is_birthday { "primary" } else { calendar_id };
        let url = if is_birthday {
            format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events?singleEvents=true&orderBy=startTime&timeMin={}&timeMax={}&maxResults=250&eventTypes=birthday",
                url_encode(query_calendar_id),
                url_encode(&time_min),
                url_encode(&time_max)
            )
        } else {
            format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events?singleEvents=true&orderBy=startTime&timeMin={}&timeMax={}&maxResults=250",
                url_encode(query_calendar_id),
                url_encode(&time_min),
                url_encode(&time_max)
            )
        };
        let response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token.access_token))
            .timeout(Duration::from_secs(6))
            .call()
            .map_err(|error| error.to_string())?;
        let raw = response.into_string().map_err(|error| error.to_string())?;
        let parsed: GoogleEventsResponse = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
        for item in parsed.items {
            let date_value = item
                .start
                .as_ref()
                .and_then(|start| start.date_time.as_ref().or(start.date.as_ref()))
                .cloned()
                .unwrap_or_default();
            let timestamp = google_event_timestamp(&date_value);
            events.push(CalendarEvent {
                title: item.summary.unwrap_or_else(|| "Untitled event".to_string()),
                date_str: google_event_label(&date_value),
                timestamp,
            });
        }
    }
    events.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    events.truncate(250);
    token.last_synced = Some(unix_now());
    save_google_calendar_token(&token)?;
    Ok(events)
}

#[derive(Debug, Deserialize, Default)]
struct GoogleEventsResponse {
    #[serde(default)]
    items: Vec<GoogleEventItem>,
}

#[derive(Debug, Deserialize, Default)]
struct GoogleEventItem {
    summary: Option<String>,
    start: Option<GoogleEventDate>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GoogleEventDate {
    date_time: Option<String>,
    date: Option<String>,
}

fn load_google_calendar_token() -> Result<Option<GoogleCalendarToken>, String> {
    let path = google_token_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&raw).map(Some).map_err(|error| error.to_string())
}

fn save_google_calendar_token(token: &GoogleCalendarToken) -> Result<(), String> {
    let path = google_token_path()?;
    let json = serde_json::to_vec(token).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn google_token_path() -> Result<PathBuf, String> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| "Unable to find config directory.".to_string())?;
    let app_dir = config_dir.join("RavenIsland");
    fs::create_dir_all(&app_dir).map_err(|error| error.to_string())?;
    Ok(app_dir.join("google_calendar.json"))
}

fn google_calendar_connect_blocking() -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    listener.set_nonblocking(true).map_err(|error| error.to_string())?;
    let port = listener.local_addr().map_err(|error| error.to_string())?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth2redirect");
    let verifier = random_string(64);
    let challenge = pkce_challenge(&verifier);
    let state = random_string(32);

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&code_challenge={}&code_challenge_method=S256&state={}",
        GOOGLE_AUTH_URL,
        url_encode(GOOGLE_CLIENT_ID),
        url_encode(&redirect_uri),
        url_encode(GOOGLE_SCOPE),
        url_encode(&challenge),
        url_encode(&state),
    );

    open_url(&auth_url)?;

    let deadline = Instant::now() + Duration::from_secs(180);
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(value) => break value,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                return Err("Google sign-in timed out.".to_string());
            }
            Err(error) => return Err(error.to_string()),
        }
    };

    let mut buffer = [0_u8; 4096];
    let size = stream.read(&mut buffer).map_err(|error| error.to_string())?;
    let request = String::from_utf8_lossy(&buffer[..size]).to_string();
    let first_line = request.lines().next().unwrap_or_default();
    let path = first_line.split_whitespace().nth(1).unwrap_or_default();
    let query = path.split_once('?').map(|(_, query)| query).unwrap_or_default();

    let returned_state = query_param(query, "state").unwrap_or_default();
    let code = query_param(query, "code").unwrap_or_default();
    let error = query_param(query, "error").unwrap_or_default();

    if !error.is_empty() {
        let _ = write_oauth_response(
            &mut stream,
            "Raven Notch Calendar connection was cancelled. You can close this tab.",
        );
        return Err(format!("Google returned an OAuth error: {error}"));
    }
    if returned_state != state || code.is_empty() {
        let _ = write_oauth_response(
            &mut stream,
            "Raven Notch Calendar could not verify Google's response. You can close this tab.",
        );
        return Err("Google OAuth response could not be verified.".to_string());
    }

    let body = form_body(&[
        ("client_id", GOOGLE_CLIENT_ID),
        ("client_secret", &google_client_secret()),
        ("code", &code),
        ("code_verifier", &verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", &redirect_uri),
    ]);
    let token_json = match post_form(GOOGLE_TOKEN_URL, &body) {
        Ok(value) => value,
        Err(error) => {
            let message = format!("Raven Notch Calendar could not finish connecting: {error}");
            let _ = write_oauth_response(&mut stream, &message);
            return Err(message);
        }
    };
    let token_response: TokenResponse = match serde_json::from_str(&token_json) {
        Ok(value) => value,
        Err(error) => {
            let message =
                format!("Raven Notch Calendar could not read Google's token response: {error}");
            let _ = write_oauth_response(&mut stream, &message);
            return Err(message);
        }
    };
    let Some(refresh_token) = token_response.refresh_token else {
        let message = "Google did not return a refresh token. Disconnect and connect again.".to_string();
        let _ = write_oauth_response(&mut stream, &message);
        return Err(message);
    };

    let access_token = token_response.access_token;
    let token = GoogleCalendarToken {
        email: fetch_google_email(&access_token).ok(),
        access_token,
        refresh_token,
        expires_at: unix_now() + token_response.expires_in.unwrap_or(3600) - 60,
        last_synced: None,
    };
    if let Err(error) = save_google_calendar_token(&token) {
        let message = format!(
            "Raven Notch Calendar connected, but could not save the token: {error}"
        );
        let _ = write_oauth_response(&mut stream, &message);
        return Err(message);
    }

    let _ = write_oauth_response(
        &mut stream,
        "Raven Notch Calendar is connected. You can close this tab.",
    );
    Ok(())
}

fn refresh_google_token_if_needed(token: &mut GoogleCalendarToken) -> Result<(), String> {
    if token.expires_at > unix_now() + 90 {
        return Ok(());
    }
    let body = form_body(&[
        ("client_id", GOOGLE_CLIENT_ID),
        ("client_secret", &google_client_secret()),
        ("refresh_token", &token.refresh_token),
        ("grant_type", "refresh_token"),
    ]);
    let token_json = post_form(GOOGLE_TOKEN_URL, &body)?;
    let token_response: TokenResponse =
        serde_json::from_str(&token_json).map_err(|error| error.to_string())?;
    token.access_token = token_response.access_token;
    token.expires_at = unix_now() + token_response.expires_in.unwrap_or(3600) - 60;
    save_google_calendar_token(token)
}

fn fetch_google_email(access_token: &str) -> Result<String, String> {
    let response = ureq::get(GOOGLE_USERINFO_URL)
        .set("Authorization", &format!("Bearer {access_token}"))
        .timeout(Duration::from_secs(8))
        .call()
        .map_err(|error| error.to_string())?
        .into_string()
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&response).map_err(|error| error.to_string())?;
    value
        .get("email")
        .and_then(|value| value.as_str())
        .map(|email| email.to_string())
        .ok_or_else(|| "Google account email was not returned.".to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarListResponse {
    #[serde(default)]
    items: Vec<GoogleCalendarEntry>,
    next_page_token: Option<String>,
}

fn fetch_google_calendars() -> Result<Vec<GoogleCalendarEntry>, String> {
    let mut token = load_google_calendar_token()?.ok_or_else(|| "Google Calendar is not connected".to_string())?;
    refresh_google_token_if_needed(&mut token)?;
    if token.access_token.trim().is_empty() {
        return Err("Google access token is missing".to_string());
    }

    let mut calendars = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = "https://www.googleapis.com/calendar/v3/users/me/calendarList?maxResults=250&showHidden=true".to_string();
        if let Some(ref tok) = page_token {
            url.push_str(&format!("&pageToken={}", url_encode(tok)));
        }

        let response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token.access_token))
            .timeout(Duration::from_secs(8))
            .call()
            .map_err(|error| error.to_string())?;
        let raw = response.into_string().map_err(|error| error.to_string())?;
        let parsed: GoogleCalendarListResponse = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
        
        calendars.extend(parsed.items);

        if parsed.next_page_token.is_none() || parsed.next_page_token.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            break;
        }
        page_token = parsed.next_page_token;
    }

    // Inject Birthdays calendar virtual entry
    calendars.insert(1, GoogleCalendarEntry {
        id: "virtual_birthdays".to_string(),
        summary: "Birthdays".to_string(),
        primary: Some(false),
    });

    // Inject Tasks calendar virtual entry
    calendars.insert(2, GoogleCalendarEntry {
        id: "virtual_tasks".to_string(),
        summary: "Tasks".to_string(),
        primary: Some(false),
    });

    Ok(calendars)
}

fn google_event_timestamp(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| date.timestamp())
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map(|date| date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
        })
        .unwrap_or(0)
}

fn google_event_label(value: &str) -> String {
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(value) {
        return date.with_timezone(&Local).format("%b %d, %-I:%M %p").to_string();
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return date.format("%b %d").to_string();
    }
    value.to_string()
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn form_body(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", url_encode(key), url_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|part| {
        let (candidate_key, value) = part.split_once('=')?;
        if candidate_key == key {
            Some(url_decode(value))
        } else {
            None
        }
    })
}

fn url_decode(input: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = input.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let hi = chars.next().unwrap_or(b'0');
            let lo = chars.next().unwrap_or(b'0');
            if let Ok(hex) = std::str::from_utf8(&[hi, lo]) {
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    bytes.push(value);
                    continue;
                }
            }
        }
        bytes.push(if byte == b'+' { b' ' } else { byte });
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn post_form(url: &str, body: &str) -> Result<String, String> {
    let response = ureq::post(url)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .timeout(Duration::from_secs(10))
        .send_string(body)
        .map_err(|error| match error {
            ureq::Error::Status(code, response) => {
                let body = response.into_string().unwrap_or_else(|_| String::new());
                if body.is_empty() {
                    format!("{url}: status code {code}")
                } else {
                    format!("{url}: status code {code}: {body}")
                }
            }
            ureq::Error::Transport(transport) => transport.to_string(),
        })?;

    response.into_string().map_err(|error| error.to_string())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn build_oauth_html(title: &str, subtitle: &str, is_success: bool) -> String {
    let logo_bytes = include_bytes!("../ui/assets/app_logo.png");
    let logo_b64 = base64::engine::general_purpose::STANDARD.encode(logo_bytes);
    let escaped_title = escape_html(title);
    let escaped_subtitle = escape_html(subtitle);
    let headline = if is_success {
        "You have successfully authenticated."
    } else {
        "Authentication could not be completed."
    };
    let helper = if is_success {
        "Raven Notch Calendar is connected. You can close this tab now."
    } else {
        "Return to Raven Notch and try connecting Google Calendar again."
    };
    let page_title = if is_success {
        "Successfully Authenticated"
    } else {
        "Authentication Failed"
    };

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} | Raven Notch</title>
    <style>
        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}

        body {{
            background: #ffffff;
            color: #202124;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            overflow: hidden;
            position: relative;
        }}

        #particle-svg {{
            position: absolute;
            inset: 0;
            width: 100%;
            height: 100%;
            z-index: 1;
            pointer-events: none;
            transform-origin: 50% 50%;
            animation: rotate-spiral 80s linear infinite;
        }}

        .content {{
            position: relative;
            z-index: 2;
            text-align: center;
            width: min(720px, calc(100vw - 40px));
            padding: 30px 24px;
            transform: translateY(10px);
            opacity: 0;
            animation: content-in 700ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
        }}

        .logo-row {{
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 14px;
            margin-bottom: 24px;
        }}

        .logo {{
            width: 42px;
            height: 42px;
            object-fit: contain;
        }}

        .brand {{
            font-size: clamp(30px, 4vw, 42px);
            font-weight: 560;
            letter-spacing: -1.2px;
            color: #202124;
        }}

        .mark {{
            display: inline-flex;
            align-items: center;
            justify-content: center;
            width: 28px;
            height: 28px;
            border-radius: 50%;
            margin-left: 2px;
            background: {};
            color: white;
            font-size: 17px;
            font-weight: 800;
            vertical-align: 3px;
            box-shadow: 0 8px 24px {};
        }}

        h1 {{
            font-size: clamp(28px, 3.4vw, 36px);
            font-weight: 400;
            letter-spacing: -0.7px;
            line-height: 1.2;
            color: #202124;
            margin-bottom: 24px;
        }}

        .subline {{
            font-size: 14px;
            color: #5f6368;
            line-height: 1.7;
            margin-bottom: 26px;
        }}

        .details {{
            font-size: 15px;
            color: {};
            line-height: 1.6;
            margin-bottom: 24px;
            min-height: 24px;
        }}

        .links {{
            display: flex;
            justify-content: center;
            gap: 16px;
            font-size: 14px;
        }}

        a {{
            color: #1a73e8;
            text-decoration: none;
        }}

        a:hover {{
            text-decoration: underline;
        }}

        @keyframes rotate-spiral {{
            from {{ transform: rotate(0deg); }}
            to {{ transform: rotate(360deg); }}
        }}

        @keyframes content-in {{
            to {{
                transform: translateY(0);
                opacity: 1;
            }}
        }}

        @media (prefers-reduced-motion: reduce) {{
            #particle-svg, .content {{
                animation: none;
            }}
            .content {{
                transform: none;
                opacity: 1;
            }}
        }}

        @media (max-width: 560px) {{
            .logo-row {{ gap: 10px; }}
            .logo {{ width: 34px; height: 34px; }}
            .links {{ gap: 12px; }}
        }}
    </style>
</head>
<body>
    <svg id="particle-svg" aria-hidden="true"></svg>

    <main class="content">
        <div class="logo-row">
            <img class="logo" src="data:image/png;base64,{}" alt="Raven Notch" />
            <div class="brand">Raven Notch <span class="mark">{}</span></div>
        </div>
        <h1>{}</h1>
        <p class="subline">{}</p>
        <p class="details">{}</p>
        <nav class="links" aria-label="Helpful links">
            <a href="https://ravennotch.me/">Website</a>
            <a href="javascript:window.close()">Close tab</a>
        </nav>
    </main>

    <script>
        (function () {{
            const svg = document.getElementById('particle-svg');
            const colors = ['#4285F4', '#EA4335', '#FBBC05', '#34A853', '#7E57C2', '#FF7A00'];

            function drawSpiral() {{
                const width = window.innerWidth || 1440;
                const height = window.innerHeight || 820;
                const centerX = width * 0.5;
                const centerY = height * 0.5;
                const maxRadius = Math.sqrt(width * width + height * height) * 0.62;
                const count = Math.min(620, Math.max(360, Math.floor((width * height) / 2600)));

                svg.setAttribute('viewBox', `0 0 ${{width}} ${{height}}`);
                svg.setAttribute('width', width);
                svg.setAttribute('height', height);
                svg.innerHTML = '';

                for (let i = 0; i < count; i++) {{
                    const t = i / count;
                    const angle = 0.42 + i * 0.155;
                    const radius = 22 + Math.pow(t, 0.92) * maxRadius;
                    const noiseX = (Math.random() - 0.5) * 20;
                    const noiseY = (Math.random() - 0.5) * 20;
                    const x = centerX + radius * Math.cos(angle) + noiseX;
                    const y = centerY + radius * Math.sin(angle) + noiseY;

                    const length = 2.5 + Math.random() * 7.5;
                    const dx = -Math.sin(angle) * length;
                    const dy = Math.cos(angle) * length;
                    const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');

                    line.setAttribute('x1', x.toFixed(1));
                    line.setAttribute('y1', y.toFixed(1));
                    line.setAttribute('x2', (x + dx).toFixed(1));
                    line.setAttribute('y2', (y + dy).toFixed(1));
                    line.setAttribute('stroke', colors[i % colors.length]);
                    line.setAttribute('stroke-width', (0.9 + Math.random() * 1.7).toFixed(1));
                    line.setAttribute('stroke-linecap', 'round');

                    let opacity = 0.86;
                    if (radius < 130) {{
                        opacity = radius / 130;
                    }} else if (radius > maxRadius * 0.78) {{
                        opacity = Math.max(0.1, 1 - (radius - maxRadius * 0.78) / (maxRadius * 0.28));
                    }}
                    line.setAttribute('opacity', opacity.toFixed(2));
                    svg.appendChild(line);
                }}
            }}

            drawSpiral();
            window.addEventListener('resize', drawSpiral);
        }})();
    </script>
</body>
</html>"#,
        page_title,
        if is_success { "#34A853" } else { "#EA4335" },
        if is_success { "rgba(52,168,83,0.22)" } else { "rgba(234,67,53,0.22)" },
        if is_success { "#188038" } else { "#b3261e" },
        logo_b64,
        if is_success { "✓" } else { "!" },
        headline,
        helper,
        if is_success { escaped_title } else { escaped_subtitle }
    )
}

fn write_oauth_response(stream: &mut impl Write, response_body: &str) -> std::io::Result<()> {
    let is_success = response_body.contains("connected") && !response_body.contains("could not save") && !response_body.contains("error");
    let title = if is_success { "Connection Successful" } else { "Connection Failed" };
    let html = build_oauth_html(title, response_body, is_success);
    
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        html.len(),
        html
    );
    stream.write_all(response.as_bytes())
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let operation = wide_null("open");
        let file = wide_null(url);
        let result = unsafe {
            ShellExecuteW(
                HWND(0),
                PCWSTR(operation.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };

        if result.0 as isize <= 32 {
            return Err(format!(
                "Unable to open Google sign-in in your browser. Windows ShellExecute error code: {}",
                result.0 as isize
            ));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn parse_duration_string(s: &str) -> Option<(Duration, bool)> {
    let s = s.trim().to_lowercase();
    if s == "no limit" || s == "no_limit" {
        return Some((Duration::ZERO, true));
    }
    
    if s == "until noon" {
        return parse_until_time(12, 0, false);
    }
    
    if s.starts_with("until ") {
        let time_str = s["until ".len()..].trim();
        return parse_until_time_str(time_str);
    }
    
    // Check HH:MM format like "1:30"
    if let Some((h_str, m_str)) = s.split_once(':') {
        if let (Ok(h), Ok(m)) = (h_str.trim().parse::<u64>(), m_str.trim().parse::<u64>()) {
            return Some((Duration::from_secs(h * 3600 + m * 60), false));
        }
    }
    
    // Check for units
    let mut num_str = String::new();
    let mut unit_str = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            num_str.push(c);
        } else if c.is_alphabetic() {
            unit_str.push(c);
        }
    }
    
    if let Ok(num) = num_str.parse::<u64>() {
        let unit = unit_str.trim();
        if unit.starts_with("min") || unit == "m" {
            return Some((Duration::from_secs(num * 60), false));
        } else if unit.starts_with("hour") || unit == "h" {
            return Some((Duration::from_secs(num * 3600), false));
        } else if unit.is_empty() {
            return Some((Duration::from_secs(num * 60), false));
        }
    }
    
    // Check space separated formats like "1 hour 30 minutes"
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() >= 4 {
        if let (Ok(h), Ok(m)) = (parts[0].parse::<u64>(), parts[2].parse::<u64>()) {
            let unit1 = parts[1];
            let unit2 = parts[3];
            if (unit1.starts_with("hour") || unit1 == "h") && (unit2.starts_with("min") || unit2 == "m") {
                return Some((Duration::from_secs(h * 3600 + m * 60), false));
            }
        }
    }
    
    None
}

fn parse_until_time_str(time_str: &str) -> Option<(Duration, bool)> {
    let time_str = time_str.trim().to_lowercase();
    let is_pm = time_str.contains("pm");
    let _is_am = time_str.contains("am");
    let clean_str = time_str.replace("pm", "").replace("am", "").replace(" ", "");
    
    if let Some((h_str, m_str)) = clean_str.split_once(':') {
        let h = h_str.parse::<u32>().ok()?;
        let m = m_str.parse::<u32>().ok()?;
        parse_until_time(h, m, is_pm)
    } else {
        let h = clean_str.parse::<u32>().ok()?;
        parse_until_time(h, 0, is_pm)
    }
}

fn parse_until_time(target_hour: u32, target_minute: u32, is_pm: bool) -> Option<(Duration, bool)> {
    use chrono::{Local, TimeZone};
    let now = Local::now();
    let mut target_h = target_hour;
    if is_pm && target_h < 12 {
        target_h += 12;
    } else if !is_pm && target_h == 12 {
        target_h = 0; // 12am is 00:00
    }
    
    let today = now.date_naive();
    let target_naive = today.and_hms_opt(target_h, target_minute, 0)?;
    let mut target_time = Local.from_local_datetime(&target_naive).single()?;
    
    if target_time <= now {
        let tomorrow = today + chrono::Duration::days(1);
        let target_naive_tomorrow = tomorrow.and_hms_opt(target_h, target_minute, 0)?;
        target_time = Local.from_local_datetime(&target_naive_tomorrow).single()?;
    }
    
    let diff = target_time.signed_duration_since(now);
    let secs = diff.num_seconds().max(0) as u64;
    Some((Duration::from_secs(secs), false))
}

