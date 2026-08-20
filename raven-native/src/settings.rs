use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RavenSettings {
    pub appearance: AppearanceSettings,
    pub hover: HoverSettings,
    pub advanced: AdvancedSettings,
    pub capture: CaptureSettings,
    pub media: MediaSettings,
    pub tabs: TabsSettings,
    pub clock: ClockSettings,
    pub sounds: SoundsSettings,
    pub cal: CalSettings,
    pub shortcuts: ShortcutsSettings,
    pub raven_alert: RavenAlertSettings,
    pub intelligence: IntelligenceSettings,
    pub drop: DropSettings,
    pub widgets: WidgetSettings,
    pub focus_sessions: Vec<FocusSession>,
    pub focus_goal_presets: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FocusSession {
    pub goal: String,
    pub duration_mins: i32,
    pub completed_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: i32,
    pub text: String,
    pub completed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ClockInstanceSettings {
    pub show_cpu: bool,
    pub show_ram: bool,
    pub show_battery: bool,
    pub show_percentage: bool,
    pub cpu_color: String,
    pub ram_color: String,
    pub battery_color: String,
    pub border_radius: f64,
    pub opacity: f64,
    pub size: String,
    pub pos_x: f64,
    pub pos_y: f64,
}

impl Default for ClockInstanceSettings {
    fn default() -> Self {
        Self {
            show_cpu: true,
            show_ram: true,
            show_battery: true,
            show_percentage: true,
            cpu_color: "#FFFFFF".to_string(),
            ram_color: "#AF52DE".to_string(),
            battery_color: "#34C759".to_string(),
            border_radius: 24.0,
            opacity: 0.85,
            size: "M".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetSettings {
    pub enabled: bool,
    pub click_through: bool,
    pub locked: bool,
    pub opacity: f32,
    pub instances: Vec<WidgetInstanceSettings>,
    pub clock_enabled: bool,
    pub stats_enabled: bool,
    pub clock_count: f64,
    pub actions_enabled: bool,
    pub clock_pos_x: f64,
    pub clock_pos_y: f64,
    pub stats_pos_x: f64,
    pub stats_pos_y: f64,
    pub actions_pos_x: f64,
    pub actions_pos_y: f64,
    pub stats_show_cpu: bool,
    pub stats_show_ram: bool,
    pub stats_show_battery: bool,
    pub stats_show_percentage: bool,
    pub stats_cpu_color: String,
    pub stats_ram_color: String,
    pub stats_battery_color: String,
    pub stats_border_radius: f64,
    #[serde(default)]
    pub clock_instances: Vec<ClockInstanceSettings>,
    #[serde(default)]
    pub year_journey_enabled: bool,
    #[serde(default = "default_year_journey_pos_x")]
    pub year_journey_pos_x: f64,
    #[serde(default = "default_year_journey_pos_y")]
    pub year_journey_pos_y: f64,
    #[serde(default)]
    pub day_journey_enabled: bool,
    #[serde(default = "default_day_journey_pos_x")]
    pub day_journey_pos_x: f64,
    #[serde(default = "default_day_journey_pos_y")]
    pub day_journey_pos_y: f64,
    #[serde(default)]
    pub month_journey_enabled: bool,
    #[serde(default = "default_month_journey_pos_x")]
    pub month_journey_pos_x: f64,
    #[serde(default = "default_month_journey_pos_y")]
    pub month_journey_pos_y: f64,
    #[serde(default)]
    pub media_enabled: bool,
    #[serde(default = "default_media_pos_x")]
    pub media_pos_x: f64,
    #[serde(default = "default_media_pos_y")]
    pub media_pos_y: f64,

    // Notes Widget
    #[serde(default)]
    pub notes_enabled: bool,
    #[serde(default = "default_notes_pos_x")]
    pub notes_pos_x: f64,
    #[serde(default = "default_notes_pos_y")]
    pub notes_pos_y: f64,
    #[serde(default)]
    pub notes_text: String,

    // To-Do List Widget
    #[serde(default)]
    pub todo_enabled: bool,
    #[serde(default = "default_todo_pos_x")]
    pub todo_pos_x: f64,
    #[serde(default = "default_todo_pos_y")]
    pub todo_pos_y: f64,
    #[serde(default)]
    pub todo_items: Vec<TodoItem>,
    #[serde(default = "default_todo_accent_color")]
    pub todo_accent_color: String,
    #[serde(default)]
    pub todo_hide_completed: bool,

    // Quotes Widget
    #[serde(default)]
    pub quotes_enabled: bool,
    #[serde(default = "default_quotes_pos_x")]
    pub quotes_pos_x: f64,
    #[serde(default = "default_quotes_pos_y")]
    pub quotes_pos_y: f64,
    #[serde(default = "default_true")]
    pub quotes_cycle_enabled: bool,
    #[serde(default = "default_quotes_interval")]
    pub quotes_change_interval_mins: i32,
    #[serde(default)]
    pub quotes_custom_quotes: Vec<String>,
    #[serde(default)]
    pub quotes_current_index: i32,
    #[serde(default)]
    pub quotes_last_changed: String,

    // Picture Widget
    #[serde(default)]
    pub picture_enabled: bool,
    #[serde(default = "default_picture_pos_x")]
    pub picture_pos_x: f64,
    #[serde(default = "default_picture_pos_y")]
    pub picture_pos_y: f64,
    #[serde(default)]
    pub picture_path: String,

    // Video Frame Widget
    #[serde(default)]
    pub video_enabled: bool,
    #[serde(default = "default_video_pos_x")]
    pub video_pos_x: f64,
    #[serde(default = "default_video_pos_y")]
    pub video_pos_y: f64,
    #[serde(default)]
    pub video_path: String,

    // Battery Percentage Widget
    #[serde(default)]
    pub battery_widget_enabled: bool,
    #[serde(default = "default_battery_widget_pos_x")]
    pub battery_widget_pos_x: f64,
    #[serde(default = "default_battery_widget_pos_y")]
    pub battery_widget_pos_y: f64,

    // Calendar Focus Widget
    #[serde(default)]
    pub calendar_focus_enabled: bool,
    #[serde(default = "default_calendar_focus_pos_x")]
    pub calendar_focus_pos_x: f64,
    #[serde(default = "default_calendar_focus_pos_y")]
    pub calendar_focus_pos_y: f64,
    #[serde(default = "default_focus_timer_minutes")]
    pub focus_timer_minutes: f64,

    // System Stats Widget
    #[serde(default)]
    pub system_stats_widget_enabled: bool,
    #[serde(default = "default_system_stats_widget_pos_x")]
    pub system_stats_widget_pos_x: f64,
    #[serde(default = "default_system_stats_widget_pos_y")]
    pub system_stats_widget_pos_y: f64,

    // Apps Container Widget
    #[serde(default)]
    pub apps_container_enabled: bool,
    #[serde(default = "default_apps_container_pos_x")]
    pub apps_container_pos_x: f64,
    #[serde(default = "default_apps_container_pos_y")]
    pub apps_container_pos_y: f64,
    #[serde(default)]
    pub apps_container_items: Vec<AppShortcutItem>,

    // Focus Score Widget
    #[serde(default)]
    pub focus_score_widget_enabled: bool,
    #[serde(default = "default_focus_score_widget_pos_x")]
    pub focus_score_widget_pos_x: f64,
    #[serde(default = "default_focus_score_widget_pos_y")]
    pub focus_score_widget_pos_y: f64,
    #[serde(default = "default_focus_score_goal_hours")]
    pub focus_score_goal_hours: f64,

    // Streak Widget
    #[serde(default)]
    pub streak_widget_enabled: bool,
    #[serde(default = "default_streak_widget_pos_x")]
    pub streak_widget_pos_x: f64,
    #[serde(default = "default_streak_widget_pos_y")]
    pub streak_widget_pos_y: f64,
    #[serde(default = "default_streak_name")]
    pub streak_name: String,

    #[serde(default)]
    pub always_on_top_widget_ids: Vec<String>,

    /// Tracks the order in which widgets were enabled (for the active-widgets panel row)
    #[serde(default)]
    pub widget_order: Vec<String>,
}

fn default_year_journey_pos_x() -> f64 { 40.0 }
fn default_year_journey_pos_y() -> f64 { 200.0 }
fn default_day_journey_pos_x() -> f64 { 40.0 }
fn default_day_journey_pos_y() -> f64 { 360.0 }
fn default_month_journey_pos_x() -> f64 { 40.0 }
fn default_month_journey_pos_y() -> f64 { 520.0 }
fn default_media_pos_x() -> f64 { 40.0 }
fn default_media_pos_y() -> f64 { 680.0 }

fn default_notes_pos_x() -> f64 { 40.0 }
fn default_notes_pos_y() -> f64 { 840.0 }
fn default_todo_pos_x() -> f64 { 400.0 }
fn default_todo_pos_y() -> f64 { 200.0 }
fn default_quotes_pos_x() -> f64 { 400.0 }
fn default_quotes_pos_y() -> f64 { 360.0 }
fn default_picture_pos_x() -> f64 { 400.0 }
fn default_picture_pos_y() -> f64 { 520.0 }
fn default_video_pos_x() -> f64 { 400.0 }
fn default_video_pos_y() -> f64 { 680.0 }
fn default_battery_widget_pos_x() -> f64 { 720.0 }
fn default_battery_widget_pos_y() -> f64 { 200.0 }
fn default_calendar_focus_pos_x() -> f64 { 720.0 }
fn default_calendar_focus_pos_y() -> f64 { 420.0 }
fn default_focus_timer_minutes() -> f64 { 25.0 }
fn default_system_stats_widget_pos_x() -> f64 { 720.0 }
fn default_system_stats_widget_pos_y() -> f64 { 640.0 }
fn default_apps_container_pos_x() -> f64 { 1040.0 }
fn default_apps_container_pos_y() -> f64 { 200.0 }
fn default_focus_score_widget_pos_x() -> f64 { 760.0 }
fn default_focus_score_widget_pos_y() -> f64 { 420.0 }
fn default_focus_score_goal_hours() -> f64 { 15.0 }
fn default_streak_widget_pos_x() -> f64 { 700.0 }
fn default_streak_widget_pos_y() -> f64 { 650.0 }
fn default_streak_name() -> String { "My Streak".to_string() }


fn default_todo_accent_color() -> String { "#FF9500".to_string() }
fn default_true() -> bool { true }
fn default_quotes_interval() -> i32 { 15 }
fn default_bezel_opacity() -> f32 { 95.0 }


impl WidgetSettings {
    pub fn get_clock_instance(&self, idx: usize) -> ClockInstanceSettings {
        if idx < self.clock_instances.len() {
            self.clock_instances[idx].clone()
        } else {
            ClockInstanceSettings {
                show_cpu: self.stats_show_cpu,
                show_ram: self.stats_show_ram,
                show_battery: self.stats_show_battery,
                show_percentage: self.stats_show_percentage,
                cpu_color: self.stats_cpu_color.clone(),
                ram_color: self.stats_ram_color.clone(),
                battery_color: self.stats_battery_color.clone(),
                border_radius: self.stats_border_radius,
                opacity: self.opacity as f64,
                size: "M".to_string(),
                pos_x: 0.0,
                pos_y: 0.0,
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppShortcutItem {
    pub name: String,
    pub path: String,
}

impl Default for AppShortcutItem {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetInstanceSettings {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub widget_type: String,
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub opacity: i32,
    pub locked: bool,
    pub data: serde_json::Value,
}

impl Default for WidgetInstanceSettings {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: "Widget".to_string(),
            widget_type: "widget".to_string(),
            visible: true,
            x: 44,
            y: 86,
            width: 260,
            height: 160,
            opacity: 96,
            locked: false,
            data: serde_json::json!({}),
        }
    }
}

impl Default for WidgetSettings {
    fn default() -> Self {
        Self {
            enabled: false,          // OFF by default
            click_through: false,
            locked: true,
            opacity: 0.85,
            instances: Vec::new(),
            clock_enabled: false,   // Clock widget disabled by default
            stats_enabled: false,   // Stats widget disabled by default
            clock_count: 1.0,
            actions_enabled: false,
            clock_pos_x: 40.0,
            clock_pos_y: 60.0,
            stats_pos_x: 40.0,
            stats_pos_y: 220.0,
            actions_pos_x: 40.0,
            actions_pos_y: 380.0,
            stats_show_cpu: true,
            stats_show_ram: true,
            stats_show_battery: true,
            stats_show_percentage: true,
            stats_cpu_color: "#FFFFFF".to_string(),
            stats_ram_color: "#AF52DE".to_string(),
            stats_battery_color: "#34C759".to_string(),
            stats_border_radius: 24.0,
            clock_instances: Vec::new(),
            year_journey_enabled: false,
            year_journey_pos_x: 40.0,
            year_journey_pos_y: 200.0,
            day_journey_enabled: false,
            day_journey_pos_x: 40.0,
            day_journey_pos_y: 360.0,
            month_journey_enabled: false,
            month_journey_pos_x: 40.0,
            month_journey_pos_y: 520.0,
            media_enabled: false,
            media_pos_x: 40.0,
            media_pos_y: 680.0,
            notes_enabled: false,
            notes_pos_x: 40.0,
            notes_pos_y: 840.0,
            notes_text: "".to_string(),
            todo_enabled: false,
            todo_pos_x: 400.0,
            todo_pos_y: 200.0,
            todo_items: Vec::new(),
            todo_accent_color: "#FF9500".to_string(),
            todo_hide_completed: false,
            quotes_enabled: false,
            quotes_pos_x: 400.0,
            quotes_pos_y: 360.0,
            quotes_cycle_enabled: true,
            quotes_change_interval_mins: 15,
            quotes_custom_quotes: Vec::new(),
            quotes_current_index: 0,
            quotes_last_changed: "".to_string(),
            picture_enabled: false,
            picture_pos_x: 400.0,
            picture_pos_y: 520.0,
            picture_path: "".to_string(),
            video_enabled: false,
            video_pos_x: 400.0,
            video_pos_y: 680.0,
            video_path: "".to_string(),
            battery_widget_enabled: false,
            battery_widget_pos_x: 720.0,
            battery_widget_pos_y: 200.0,
            calendar_focus_enabled: false,
            calendar_focus_pos_x: 720.0,
            calendar_focus_pos_y: 420.0,
            focus_timer_minutes: 25.0,
            system_stats_widget_enabled: false,
            system_stats_widget_pos_x: 720.0,
            system_stats_widget_pos_y: 640.0,
            apps_container_enabled: false,
            apps_container_pos_x: 1040.0,
            apps_container_pos_y: 200.0,
            apps_container_items: Vec::new(),
            focus_score_widget_enabled: false,
            focus_score_widget_pos_x: 760.0,
            focus_score_widget_pos_y: 420.0,
            focus_score_goal_hours: 15.0,
            streak_widget_enabled: false,
            streak_widget_pos_x: 700.0,
            streak_widget_pos_y: 650.0,
            streak_name: "My Streak".to_string(),
            always_on_top_widget_ids: Vec::new(),
            widget_order: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DropSettings {
    pub enabled: bool,
    pub auto_expand: bool,
    pub open_after_drop: bool,
    pub keep_max: u32,
    pub default_provider: String,
}

impl Default for DropSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_expand: true,
            open_after_drop: true,
            keep_max: 10,
            default_provider: "localsend".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub idle_width: f32,
    pub idle_height: f32,
    pub border_radius: f32,
    pub idle_border_radius: f32,
    pub pill_offset: f32,
    pub pill_y_offset: f32,
    pub auto_hide: bool,
    pub auto_hide_on_fullscreen: bool,
    pub notch_opacity: f32,
    pub inactive_opacity: f32,
    pub shape: String,
    pub idle_pill_mode: String,
    pub idle_custom_name: String,
    pub notch_color: String,
    pub accent_color: String,
    pub appearance_mode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct HoverSettings {
    pub enabled: bool,
    pub open_delay: u32,
    pub close_delay: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedSettings {
    pub reserve_top_area: bool,
    pub reserve_top_height: u32,
    pub full_width_bar: bool,
    pub top_bar_widgets: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_raven: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_media: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_apps: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_stats: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_clipboard: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_volume: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_wifi: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_battery: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_timer: bool,
    #[serde(default = "default_true")]
    pub top_bar_widget_calendar: bool,
    pub run_on_startup: bool,
    #[serde(default = "default_bezel_opacity")]
    pub bezel_opacity: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureSettings {
    pub enabled: bool,
    pub default_screenshot_mode: String,
    pub default_recording_mode: String,
    pub save_screenshots_to: String,
    pub save_recordings_to: String,
    pub include_cursor: bool,
    pub show_recording_indicator: bool,
    pub mic_enabled: bool,
    pub system_audio_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaSettings {
    pub calendar_url: String,
    pub google_calendar_ids: Vec<String>,
    pub show_dashboard: bool,
    pub show_upcoming: bool,
    pub show_notifications: bool,
    pub show_calendar_strip: bool,
    pub show_lyrics: bool,
    pub show_backdrop: bool,
    pub cover_art_animate: bool,
    pub cover_art_hover: bool,
    pub adaptive_accent: bool,
    pub eq_complexity: f64,
    pub show_waveform: bool,
    pub pill_waveform: bool,
    pub auto_expand: bool,
    pub show_source: bool,
    #[serde(default)]
    pub full_calendar_on_no_media: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TabsSettings {
    pub home: bool,
    pub media: bool,
    pub calendar: bool,
    pub clock: bool,
    pub drop: bool,
    pub capture: bool,
    pub notifications: bool,
    pub stats: bool,
    pub caffeine: bool,
    pub settings: bool,
    pub battery: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockSettings {
    pub mode_24h: bool,
    pub show_seconds: bool,
    pub show_ampm: bool,
    pub blink_colon: bool,
    pub show_weekday: bool,
    pub show_date: bool,
    pub show_utc: bool,
    pub show_timer: bool,
    pub show_stopwatch: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundsSettings {
    pub enabled: bool,
    pub timer_complete: bool,
    pub stopwatch: bool,
    pub battery_low: bool,
    pub charger_connected: bool,
    pub charger_disconnected: bool,
    pub capslock_on: bool,
    pub capslock_off: bool,
    pub unlock: bool,
    pub custom_timer_complete_path: String,
    pub custom_stopwatch_path: String,
    pub custom_battery_low_path: String,
    pub custom_charger_connected_path: String,
    pub custom_charger_disconnected_path: String,
    pub custom_capslock_on_path: String,
    pub custom_capslock_off_path: String,
    pub custom_unlock_path: String,
}

static SETTINGS_CACHE: std::sync::Mutex<Option<RavenSettings>> = std::sync::Mutex::new(None);
static SETTINGS_JSON_CACHE: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

impl RavenSettings {
    pub fn validate_and_sanitize(&mut self) -> bool {
        let mut mutated = false;

        macro_rules! check_and_clamp {
            ($field:expr, $min:expr, $max:expr) => {
                let clamped = $field.clamp($min, $max);
                if ($field - clamped).abs() > 0.001 {
                    $field = clamped;
                    mutated = true;
                }
            };
        }

        macro_rules! check_and_clamp_int {
            ($field:expr, $min:expr, $max:expr) => {
                let clamped = $field.clamp($min, $max);
                if $field != clamped {
                    $field = clamped;
                    mutated = true;
                }
            };
        }

        check_and_clamp!(self.appearance.idle_width, 100.0, 800.0);
        check_and_clamp!(self.appearance.idle_height, 20.0, 150.0);
        check_and_clamp!(self.appearance.border_radius, 0.0, 100.0);
        check_and_clamp!(self.appearance.pill_offset, -500.0, 500.0);
        check_and_clamp!(self.appearance.pill_y_offset, -20.0, 100.0);
        check_and_clamp!(self.appearance.notch_opacity, 0.0, 100.0);
        check_and_clamp!(self.appearance.inactive_opacity, 0.0, 100.0);
        check_and_clamp!(self.advanced.bezel_opacity, 0.0, 100.0);

        check_and_clamp_int!(self.hover.open_delay, 0, 5000);
        check_and_clamp_int!(self.hover.close_delay, 0, 5000);
        check_and_clamp_int!(self.advanced.reserve_top_height, 0, 300);
        check_and_clamp!(self.widgets.focus_timer_minutes, 1.0, 180.0);

        for inst in &mut self.widgets.instances {
            check_and_clamp_int!(inst.width, 80, 2000);
            check_and_clamp_int!(inst.height, 40, 2000);
            check_and_clamp_int!(inst.opacity, 10, 100);
        }

        for inst in &mut self.widgets.clock_instances {
            let clamped_radius = inst.border_radius.clamp(0.0, 100.0);
            if (inst.border_radius - clamped_radius).abs() > 0.001 {
                inst.border_radius = clamped_radius;
                mutated = true;
            }
            let clamped_opacity = inst.opacity.clamp(0.1, 1.0);
            if (inst.opacity - clamped_opacity).abs() > 0.001 {
                inst.opacity = clamped_opacity;
                mutated = true;
            }
        }

        mutated
    }

    pub fn load() -> Self {
        if let Ok(guard) = SETTINGS_CACHE.lock() {
            if let Some(cached) = &*guard {
                return cached.clone();
            }
        }

        let path = settings_path();
        let raw = fs::read_to_string(&path).unwrap_or_default();
        let mut settings: RavenSettings = match serde_json::from_str(&raw) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[SETTINGS-ERROR] Failed to parse settings.json: {:?}", e);
                RavenSettings::default()
            }
        };

        let mutated = settings.validate_and_sanitize();

        if mutated {
            // Write it back to disk!
            if let Ok(raw_pretty) = serde_json::to_string_pretty(&settings) {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(&path, raw_pretty);
            }
            // Update the JSON cache
            if let Ok(mut json_guard) = SETTINGS_JSON_CACHE.lock() {
                if let Ok(val) = serde_json::to_value(&settings) {
                    *json_guard = Some(val);
                }
            }
        } else {
            if let Ok(mut json_guard) = SETTINGS_JSON_CACHE.lock() {
                if json_guard.is_none() {
                    let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}));
                    *json_guard = Some(val);
                }
            }
        }

        let clean = |p: &str| -> String {
            p.trim_start_matches("True\r\n")
                .trim_start_matches("True\n")
                .trim_start_matches("True\r")
                .trim()
                .to_string()
        };

        settings.sounds.custom_timer_complete_path = clean(&settings.sounds.custom_timer_complete_path);
        settings.sounds.custom_stopwatch_path = clean(&settings.sounds.custom_stopwatch_path);
        settings.sounds.custom_battery_low_path = clean(&settings.sounds.custom_battery_low_path);
        settings.sounds.custom_charger_connected_path = clean(&settings.sounds.custom_charger_connected_path);
        settings.sounds.custom_charger_disconnected_path = clean(&settings.sounds.custom_charger_disconnected_path);
        settings.sounds.custom_capslock_on_path = clean(&settings.sounds.custom_capslock_on_path);
        settings.sounds.custom_capslock_off_path = clean(&settings.sounds.custom_capslock_off_path);
        settings.sounds.custom_unlock_path = clean(&settings.sounds.custom_unlock_path);

        // Clear/migrate old preset shortcuts to blank if they are present in the loaded settings.json
        // NOTE: toggle_raven Alt+Space is now the intentional default, so we do NOT clear it.
        if settings.shortcuts.tab_home == "Alt+1" { settings.shortcuts.tab_home = String::new(); }
        if settings.shortcuts.tab_media == "Alt+2" { settings.shortcuts.tab_media = String::new(); }
        if settings.shortcuts.tab_calendar == "Alt+3" { settings.shortcuts.tab_calendar = String::new(); }
        if settings.shortcuts.tab_clock == "Alt+4" { settings.shortcuts.tab_clock = String::new(); }
        if settings.shortcuts.tab_drop == "Alt+5" { settings.shortcuts.tab_drop = String::new(); }
        if settings.shortcuts.tab_capture == "Alt+7" { settings.shortcuts.tab_capture = String::new(); }
        if settings.shortcuts.tab_stats == "Alt+6" { settings.shortcuts.tab_stats = String::new(); }
        if settings.shortcuts.media_play == "Alt+P" { settings.shortcuts.media_play = String::new(); }
        if settings.shortcuts.media_next == "Alt+N" { settings.shortcuts.media_next = String::new(); }
        if settings.shortcuts.media_prev == "Alt+B" { settings.shortcuts.media_prev = String::new(); }
        if settings.shortcuts.toggle_freeze == "Alt+F" { settings.shortcuts.toggle_freeze = String::new(); }
        if settings.shortcuts.quick_screenshot == "Control+Shift+S" { settings.shortcuts.quick_screenshot = String::new(); }
        if settings.shortcuts.quick_record_toggle == "Control+Shift+E" { settings.shortcuts.quick_record_toggle = String::new(); }
        if settings.shortcuts.open_settings == "Control+Shift+Comma" { settings.shortcuts.open_settings = String::new(); }
        if settings.shortcuts.restart_raven == "Control+Shift+R" { settings.shortcuts.restart_raven = String::new(); }
        if settings.shortcuts.quit_raven == "Control+Shift+X" { settings.shortcuts.quit_raven = String::new(); }

        if let Ok(mut guard) = SETTINGS_CACHE.lock() {
            *guard = Some(settings.clone());
        }
        settings
    }
}

impl Default for RavenSettings {
    fn default() -> Self {
        Self {
            appearance: AppearanceSettings::default(),
            hover: HoverSettings::default(),
            advanced: AdvancedSettings::default(),
            capture: CaptureSettings::default(),
            media: MediaSettings::default(),
            tabs: TabsSettings::default(),
            clock: ClockSettings::default(),
            sounds: SoundsSettings::default(),
            cal: CalSettings::default(),
            shortcuts: ShortcutsSettings::default(),
            raven_alert: RavenAlertSettings::default(),
            intelligence: IntelligenceSettings::default(),
            drop: DropSettings::default(),
            widgets: WidgetSettings::default(),
            focus_sessions: Vec::new(),
            focus_goal_presets: Vec::new(),
        }
    }
}

impl Default for SoundsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            timer_complete: true,
            stopwatch: true,
            battery_low: true,
            charger_connected: true,
            charger_disconnected: true,
            capslock_on: true,
            capslock_off: true,
            unlock: true,
            custom_timer_complete_path: String::new(),
            custom_stopwatch_path: String::new(),
            custom_battery_low_path: String::new(),
            custom_charger_connected_path: String::new(),
            custom_charger_disconnected_path: String::new(),
            custom_capslock_on_path: String::new(),
            custom_capslock_off_path: String::new(),
            custom_unlock_path: String::new(),
        }
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            idle_width: 206.0,
            idle_height: 37.0,
            border_radius: 28.0,
            idle_border_radius: 12.0,
            pill_offset: 0.0,
            pill_y_offset: 0.0,
            auto_hide: false,
            auto_hide_on_fullscreen: true,
            notch_opacity: 100.0,
            inactive_opacity: 100.0,
            shape: "curved".to_string(),
            idle_pill_mode: "none".to_string(),
            idle_custom_name: "".to_string(),
            notch_color: "#000000".to_string(),
            accent_color: "#0066FF".to_string(),
            appearance_mode: "solid".to_string(),
        }
    }
}

impl Default for HoverSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            open_delay: 0,
            close_delay: 60,
        }
    }
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            reserve_top_area: false,
            reserve_top_height: 67,
            full_width_bar: false,
            top_bar_widgets: false,
            top_bar_widget_raven: true,
            top_bar_widget_media: true,
            top_bar_widget_apps: true,
            top_bar_widget_stats: true,
            top_bar_widget_clipboard: true,
            top_bar_widget_volume: true,
            top_bar_widget_wifi: true,
            top_bar_widget_battery: true,
            top_bar_widget_timer: true,
            top_bar_widget_calendar: true,
            run_on_startup: false,
            bezel_opacity: 95.0,
        }
    }
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            default_screenshot_mode: "fullscreen".to_string(),
            default_recording_mode: "fullscreen".to_string(),
            save_screenshots_to: String::new(),
            save_recordings_to: String::new(),
            include_cursor: false,
            show_recording_indicator: true,
            mic_enabled: false,
            system_audio_enabled: false,
        }
    }
}

impl Default for MediaSettings {
    fn default() -> Self {
        Self {
            calendar_url: String::new(),
            google_calendar_ids: Vec::new(),
            show_dashboard: true,
            show_upcoming: true,
            show_notifications: true,
            show_calendar_strip: true,
            show_lyrics: true,
            show_backdrop: true,
            cover_art_animate: true,
            cover_art_hover: true,
            adaptive_accent: true,
            eq_complexity: 30.0,
            show_waveform: true,
            pill_waveform: true,
            auto_expand: true,
            show_source: true,
            full_calendar_on_no_media: true,
        }
    }
}

impl Default for TabsSettings {
    fn default() -> Self {
        Self {
            home: true,
            media: true,
            calendar: true,
            clock: true,
            drop: true,
            capture: true,
            notifications: true,
            stats: true,
            caffeine: true,
            settings: true,
            battery: true,
        }
    }
}

impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            mode_24h: false,
            show_seconds: false,
            show_ampm: true,
            blink_colon: true,
            show_weekday: true,
            show_date: true,
            show_utc: false,
            show_timer: false,
            show_stopwatch: false,
        }
    }
}

pub fn settings_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("RavenIsland").join("settings.json")
}

pub fn adjust_number(path: &[&str], delta: f64, min: f64, max: f64) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(value) = nested_value_mut(root, path) {
            let current = value.as_f64().unwrap_or(min);
            *value = serde_json::json!((current + delta).clamp(min, max));
        }
    })
}

pub fn set_number(path: &[&str], new_val: f64) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(value) = nested_value_mut(root, path) {
            *value = serde_json::json!(new_val);
        }
    })
}

pub fn toggle_bool(path: &[&str]) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(value) = nested_value_mut(root, path) {
            let current = value.as_bool().unwrap_or(false);
            *value = serde_json::json!(!current);
        }
    })
}

pub fn set_bool(path: &[&str], new_val: bool) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(value) = nested_value_mut(root, path) {
            *value = serde_json::json!(new_val);
        }
    })
}

pub fn set_string(path: &[&str], new_val: &str) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(value) = nested_value_mut(root, path) {
            *value = serde_json::json!(new_val);
        }
    })
}

/// Append a widget id to the ordered list (no duplicates).
pub fn push_widget_order(widget_id: &str) -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        let widgets = root.get_mut("widgets").unwrap();
        if !widgets.get("widget_order").is_some_and(|v| v.is_array()) {
            widgets["widget_order"] = serde_json::json!([]);
        }
        if let Some(arr) = widgets.get_mut("widget_order").and_then(|v| v.as_array_mut()) {
            let already = arr.iter().any(|v| v.as_str() == Some(widget_id));
            if !already {
                arr.push(serde_json::json!(widget_id));
            }
        }
    })
}

/// Remove a widget id from the ordered list.
pub fn remove_widget_order(widget_id: &str) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(widgets) = root.get_mut("widgets") {
            if let Some(arr) = widgets.get_mut("widget_order").and_then(|v| v.as_array_mut()) {
                arr.retain(|v| v.as_str() != Some(widget_id));
            }
        }
    })
}

/// Set the entire ordered list of active widgets.
pub fn set_widget_order(order: Vec<String>) -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        let widgets = root.get_mut("widgets").unwrap();
        widgets["widget_order"] = serde_json::json!(order);
    })
}

fn widget_copy_title(widget_type: &str) -> &'static str {
    match widget_type {
        "year_progress" => "Year Progress",
        "day_progress" => "Day Progress",
        "month_progress" => "Month Progress",
        "media" => "Media Player",
        "notes" => "Quick Notes",
        "todo" => "To-Do List",
        "quotes" => "Daily Quotes",
        "picture" => "Picture Frame",
        "video" => "Video Frame",
        "battery" => "Battery Status",
        "calendar_focus" => "Calendar Focus",
        "apps_container" => "Apps Container",
        "focus_score" => "Focus Score",
        "streak" => "Calendar Widget",
        _ => "Widget",
    }
}

fn widget_copy_dimensions(widget_type: &str) -> (i32, i32) {
    match widget_type {
        "battery" => (190, 190),
        "apps_container" => (320, 190),
        "focus_score" => (380, 190),
        "calendar_focus" => (440, 180),
        _ => (320, 150),
    }
}

fn widget_copy_base_position(widget_type: &str) -> (i32, i32) {
    match widget_type {
        "year_progress" => (40, 200),
        "day_progress" => (40, 360),
        "month_progress" => (40, 520),
        "media" => (40, 680),
        "notes" => (40, 840),
        "todo" => (400, 200),
        "quotes" => (400, 360),
        "picture" => (400, 520),
        "video" => (400, 680),
        "battery" => (720, 200),
        "calendar_focus" => (720, 420),
        "apps_container" => (1040, 200),
        "focus_score" => (760, 420),
        "streak" => (700, 650),
        _ => (44, 86),
    }
}

pub fn add_widget_instance_copy(widget_type: &str) -> RavenSettings {
    let widget_type = widget_type.trim();
    if widget_type.is_empty() {
        return RavenSettings::load();
    }

    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        let widgets = root.get_mut("widgets").unwrap();
        widgets["enabled"] = serde_json::json!(true);
        if !widgets.get("instances").is_some_and(|v| v.is_array()) {
            widgets["instances"] = serde_json::json!([]);
        }

        let initial_data = match widget_type {
            "notes" => serde_json::json!({
                "notes_text": widgets.get("notes_text").and_then(|v| v.as_str()).unwrap_or("")
            }),
            "todo" => serde_json::json!({
                "todo_items": widgets.get("todo_items").cloned().unwrap_or_else(|| serde_json::json!([]))
            }),
            "picture" => serde_json::json!({
                "picture_path": widgets.get("picture_path").and_then(|v| v.as_str()).unwrap_or("")
            }),
            "video" => serde_json::json!({
                "video_path": widgets.get("video_path").and_then(|v| v.as_str()).unwrap_or("")
            }),
            "apps_container" => serde_json::json!({
                "apps_container_items": widgets.get("apps_container_items").cloned().unwrap_or_else(|| serde_json::json!([]))
            }),
            "streak" => serde_json::json!({
                "streak_name": widgets.get("streak_name").and_then(|v| v.as_str()).unwrap_or("My Streak")
            }),
            _ => serde_json::json!({}),
        };
        let instances = widgets.get_mut("instances").and_then(|v| v.as_array_mut()).unwrap();
        let copy_count = instances
            .iter()
            .filter(|item| item.get("type").and_then(|v| v.as_str()) == Some(widget_type))
            .count();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let id = format!("{}_copy_{}", widget_type, now);
        let (base_x, base_y) = widget_copy_base_position(widget_type);
        let (width, height) = widget_copy_dimensions(widget_type);
        let offset = ((copy_count + 1) as i32) * 28;

        instances.push(serde_json::json!({
            "id": id,
            "title": widget_copy_title(widget_type),
            "type": widget_type,
            "visible": true,
            "x": base_x + offset,
            "y": base_y + offset,
            "width": width,
            "height": height,
            "opacity": 96,
            "locked": false,
            "data": initial_data,
        }));
    })
}

pub fn clear_widget_instances() -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        root["widgets"]["instances"] = serde_json::json!([]);
    })
}

pub fn add_apps_container_item(path: &str) -> RavenSettings {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return RavenSettings::load();
    }

    let name = std::path::Path::new(trimmed)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(trimmed)
        .to_string();

    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        let widgets = root.get_mut("widgets").unwrap();
        if !widgets.get("apps_container_items").is_some_and(|v| v.is_array()) {
            widgets["apps_container_items"] = serde_json::json!([]);
        }
        if let Some(items) = widgets.get_mut("apps_container_items").and_then(|v| v.as_array_mut()) {
            let exists = items.iter().any(|item| {
                item.get("path")
                    .and_then(|v| v.as_str())
                    .is_some_and(|existing| existing.eq_ignore_ascii_case(trimmed))
            });
            if !exists {
                items.push(serde_json::json!({
                    "name": name,
                    "path": trimmed,
                }));
            }
        }
    })
}

pub fn remove_apps_container_item(index: usize) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(items) = root
            .get_mut("widgets")
            .and_then(|w| w.get_mut("apps_container_items"))
            .and_then(|v| v.as_array_mut())
        {
            if index < items.len() {
                items.remove(index);
            }
        }
    })
}

pub fn clear_apps_container_items() -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        root["widgets"]["apps_container_items"] = serde_json::json!([]);
    })
}

pub fn add_focus_session(goal: &str, duration_mins: i32, completed_at: &str) -> RavenSettings {
    patch_settings_json(|root| {
        if root.get("focus_sessions").is_none() {
            root["focus_sessions"] = serde_json::Value::Array(Vec::new());
        }
        if let Some(arr) = root["focus_sessions"].as_array_mut() {
            arr.push(serde_json::json!({
                "goal": goal,
                "duration_mins": duration_mins,
                "completed_at": completed_at,
            }));
        }
    })
}

pub fn clear_focus_history() -> RavenSettings {
    patch_settings_json(|root| {
        root["focus_sessions"] = serde_json::Value::Array(Vec::new());
    })
}

pub fn toggle_focus_session_day(date: chrono::NaiveDate) -> RavenSettings {
    patch_settings_json(|root| {
        if root.get("focus_sessions").is_none() {
            root["focus_sessions"] = serde_json::Value::Array(Vec::new());
        }
        let target_str = date.format("%Y-%m-%d").to_string();
        if let Some(arr) = root["focus_sessions"].as_array_mut() {
            let mut found_idx = None;
            for (idx, item) in arr.iter().enumerate() {
                if let Some(completed_at) = item.get("completed_at").and_then(|v| v.as_str()) {
                    if completed_at.starts_with(&target_str) {
                        found_idx = Some(idx);
                        break;
                    }
                }
            }
            if let Some(_idx) = found_idx {
                // Remove all sessions on that day
                arr.retain(|item| {
                    if let Some(completed_at) = item.get("completed_at").and_then(|v| v.as_str()) {
                        !completed_at.starts_with(&target_str)
                    } else {
                        true
                    }
                });
            } else {
                // Add a session
                arr.push(serde_json::json!({
                    "goal": "Streak Manual",
                    "duration_mins": 30,
                    "completed_at": format!("{}T12:00:00Z", target_str),
                }));
            }
        }
    })
}

pub fn add_focus_goal_preset(goal: &str) -> RavenSettings {
    let trimmed = goal.trim();
    if trimmed.is_empty() {
        return RavenSettings::load();
    }

    patch_settings_json(|root| {
        if !root.get("focus_goal_presets").is_some_and(|v| v.is_array()) {
            root["focus_goal_presets"] = serde_json::Value::Array(Vec::new());
        }
        if let Some(arr) = root["focus_goal_presets"].as_array_mut() {
            let exists = arr
                .iter()
                .any(|item| item.as_str().is_some_and(|value| value.eq_ignore_ascii_case(trimmed)));
            if !exists {
                arr.push(serde_json::json!(trimmed));
            }
        }
    })
}

pub fn set_widget_always_on_top(widget_id: &str, enabled: bool) -> RavenSettings {
    let id = widget_id.trim();
    if id.is_empty() {
        return RavenSettings::load();
    }
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        let widgets = root.get_mut("widgets").unwrap();
        if !widgets.get("always_on_top_widget_ids").is_some_and(|v| v.is_array()) {
            widgets["always_on_top_widget_ids"] = serde_json::json!([]);
        }
        if let Some(items) = widgets.get_mut("always_on_top_widget_ids").and_then(|v| v.as_array_mut()) {
            items.retain(|item| item.as_str() != Some(id));
            if enabled { items.push(serde_json::json!(id)); }
        }
    })
}

pub fn is_widget_always_on_top(settings: &RavenSettings, widget_id: &str) -> bool {
    settings.widgets.always_on_top_widget_ids.iter().any(|id| id == widget_id)
}

pub fn set_streak_name(name: &str) -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) { root["widgets"] = serde_json::json!({}); }
        if let Some(widgets) = root.get_mut("widgets") { widgets["streak_name"] = serde_json::json!(name); }
    })
}

pub fn update_widget_instance_position(instance_id: &str, x: i32, y: i32) -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) { root["widgets"] = serde_json::json!({}); }
        if let Some(widgets) = root.get_mut("widgets") {
            if let Some(instances) = widgets.get_mut("instances") {
                if let Some(arr) = instances.as_array_mut() {
                    for item in arr {
                        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                            if id == instance_id {
                                if let Some(xv) = item.get_mut("x") { *xv = serde_json::json!(x); }
                                if let Some(yv) = item.get_mut("y") { *yv = serde_json::json!(y); }
                                break;
                            }
                        }
                    }
                }
            }
        }
    })
}

pub fn update_widget_instance_visibility(instance_id: &str, visible: bool) -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) { root["widgets"] = serde_json::json!({}); }
        if let Some(widgets) = root.get_mut("widgets") {
            if let Some(instances) = widgets.get_mut("instances") {
                if let Some(arr) = instances.as_array_mut() {
                    for item in arr {
                        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                            if id == instance_id {
                                if let Some(vis_val) = item.get_mut("visible") { *vis_val = serde_json::json!(visible); }
                                break;
                            }
                        }
                    }
                }
            }
        }
    })
}

pub fn set_widget_instance_data_value(instance_id: &str, key: &str, value: serde_json::Value) -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) { root["widgets"] = serde_json::json!({}); }
        if let Some(instances) = root["widgets"].get_mut("instances").and_then(|v| v.as_array_mut()) {
            for item in instances {
                if item.get("id").and_then(|v| v.as_str()) == Some(instance_id) {
                    if !item.get("data").is_some_and(|v| v.is_object()) {
                        item["data"] = serde_json::json!({});
                    }
                    item["data"][key] = value;
                    break;
                }
            }
        }
    })
}

pub fn instance_todo_add_item(instance_id: &str, text: &str) -> RavenSettings {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return RavenSettings::load();
    }
    patch_settings_json(|root| {
        if let Some(instances) = root["widgets"].get_mut("instances").and_then(|v| v.as_array_mut()) {
            for item in instances {
                if item.get("id").and_then(|v| v.as_str()) == Some(instance_id) {
                    if !item.get("data").is_some_and(|v| v.is_object()) {
                        item["data"] = serde_json::json!({});
                    }
                    if !item["data"].get("todo_items").is_some_and(|v| v.is_array()) {
                        item["data"]["todo_items"] = serde_json::json!([]);
                    }
                    let next_id = item["data"]["todo_items"]
                        .as_array()
                        .map(|arr| arr.iter().filter_map(|v| v.get("id").and_then(|id| id.as_i64())).max().unwrap_or(0) + 1)
                        .unwrap_or(1);
                    if let Some(arr) = item["data"]["todo_items"].as_array_mut() {
                        arr.push(serde_json::json!({
                            "id": next_id,
                            "text": trimmed,
                            "completed": false,
                        }));
                    }
                    break;
                }
            }
        }
    })
}

pub fn instance_todo_toggle_item(instance_id: &str, todo_id: i32) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(instances) = root["widgets"].get_mut("instances").and_then(|v| v.as_array_mut()) {
            for item in instances {
                if item.get("id").and_then(|v| v.as_str()) == Some(instance_id) {
                    if let Some(arr) = item["data"].get_mut("todo_items").and_then(|v| v.as_array_mut()) {
                        for todo in arr {
                            if todo.get("id").and_then(|v| v.as_i64()) == Some(todo_id as i64) {
                                let next = !todo.get("completed").and_then(|v| v.as_bool()).unwrap_or(false);
                                todo["completed"] = serde_json::json!(next);
                                break;
                            }
                        }
                    }
                    break;
                }
            }
        }
    })
}

pub fn instance_todo_delete_item(instance_id: &str, todo_id: i32) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(instances) = root["widgets"].get_mut("instances").and_then(|v| v.as_array_mut()) {
            for item in instances {
                if item.get("id").and_then(|v| v.as_str()) == Some(instance_id) {
                    if let Some(arr) = item["data"].get_mut("todo_items").and_then(|v| v.as_array_mut()) {
                        arr.retain(|todo| todo.get("id").and_then(|v| v.as_i64()) != Some(todo_id as i64));
                    }
                    break;
                }
            }
        }
    })
}

pub fn instance_todo_move_item(instance_id: &str, todo_id: i32, is_up: bool) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(instances) = root["widgets"].get_mut("instances").and_then(|v| v.as_array_mut()) {
            for item in instances {
                if item.get("id").and_then(|v| v.as_str()) == Some(instance_id) {
                    if let Some(arr) = item["data"].get_mut("todo_items").and_then(|v| v.as_array_mut()) {
                        if let Some(idx) = arr.iter().position(|todo| todo.get("id").and_then(|v| v.as_i64()) == Some(todo_id as i64)) {
                            let swap_idx = if is_up { idx.checked_sub(1) } else if idx + 1 < arr.len() { Some(idx + 1) } else { None };
                            if let Some(other) = swap_idx {
                                arr.swap(idx, other);
                            }
                        }
                    }
                    break;
                }
            }
        }
    })
}

pub fn remove_instance_apps_container_item(instance_id: &str, index: usize) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(instances) = root["widgets"].get_mut("instances").and_then(|v| v.as_array_mut()) {
            for item in instances {
                if item.get("id").and_then(|v| v.as_str()) == Some(instance_id) {
                    if let Some(arr) = item["data"].get_mut("apps_container_items").and_then(|v| v.as_array_mut()) {
                        if index < arr.len() {
                            arr.remove(index);
                        }
                    }
                    break;
                }
            }
        }
    })
}

pub fn update_clock_instance_setting<F>(idx: usize, updater: F) -> RavenSettings
where
    F: FnOnce(&mut ClockInstanceSettings),
{
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        let widgets = root.get_mut("widgets").unwrap();

        let default_inst = ClockInstanceSettings {
            show_cpu: widgets.get("stats_show_cpu").and_then(|v| v.as_bool()).unwrap_or(true),
            show_ram: widgets.get("stats_show_ram").and_then(|v| v.as_bool()).unwrap_or(true),
            show_battery: widgets.get("stats_show_battery").and_then(|v| v.as_bool()).unwrap_or(true),
            show_percentage: widgets.get("stats_show_percentage").and_then(|v| v.as_bool()).unwrap_or(true),
            cpu_color: widgets.get("stats_cpu_color").and_then(|v| v.as_str()).unwrap_or("#FFFFFF").to_string(),
            ram_color: widgets.get("stats_ram_color").and_then(|v| v.as_str()).unwrap_or("#AF52DE").to_string(),
            battery_color: widgets.get("stats_battery_color").and_then(|v| v.as_str()).unwrap_or("#34C759").to_string(),
            border_radius: widgets.get("stats_border_radius").and_then(|v| v.as_f64()).unwrap_or(24.0),
            opacity: widgets.get("opacity").and_then(|v| v.as_f64()).unwrap_or(0.85),
            size: "M".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        };

        if !widgets.get("clock_instances").is_some_and(|v| v.is_array()) {
            widgets["clock_instances"] = serde_json::json!([]);
        }
        let clock_instances_val = widgets.get_mut("clock_instances").unwrap();
        let arr = clock_instances_val.as_array_mut().unwrap();

        while arr.len() <= idx {
            arr.push(serde_json::to_value(&default_inst).unwrap_or_default());
        }

        let mut inst: ClockInstanceSettings = serde_json::from_value(arr[idx].clone()).unwrap_or_default();
        updater(&mut inst);
        arr[idx] = serde_json::to_value(&inst).unwrap_or_default();
    })
}

/// Remove the clock instance at `idx`, shifting later instances down, and decrement clock_count.
pub fn remove_clock_instance(idx: usize) -> RavenSettings {
    patch_settings_json(|root| {
        // Ensure "widgets" section exists — on a fresh install root is {}
        // and get_mut("widgets") would return None, silently skipping everything.
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        if let Some(widgets) = root.get_mut("widgets") {
            // Remove from clock_instances array
            if let Some(arr) = widgets.get_mut("clock_instances").and_then(|v| v.as_array_mut()) {
                if idx < arr.len() {
                    arr.remove(idx);
                }
            }
            // Decrement clock_count
            let current_count = widgets.get("clock_count").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let new_count = (current_count - 1.0).max(0.0);
            widgets["clock_count"] = serde_json::json!(new_count);
            // If count reaches 0, disable only the clock widget.
            // The global widgets.enabled switch may still be powering other active widgets.
            if new_count == 0.0 {
                widgets["stats_enabled"] = serde_json::json!(false);
                widgets["clock_enabled"] = serde_json::json!(false);
            }
        }
    })
}

pub fn toggle_google_calendar_id(calendar_id: &str) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(value) = nested_value_mut(root, &["media", "google_calendar_ids"]) {
            let mut ids: Vec<String> = serde_json::from_value(value.clone()).unwrap_or_default();
            if let Some(pos) = ids.iter().position(|id| id == calendar_id) {
                ids.remove(pos);
            } else {
                ids.push(calendar_id.to_string());
            }
            *value = serde_json::json!(ids);
        }
    })
}

pub fn todo_clear_completed() -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(widgets) = root.get_mut("widgets") {
            if let Some(todo_items) = widgets.get_mut("todo_items") {
                if let Some(arr) = todo_items.as_array_mut() {
                    arr.retain(|item| {
                        item.get("completed").and_then(|c| c.as_bool()).unwrap_or(false) == false
                    });
                }
            }
        }
    })
}

pub fn todo_add_item(text: &str) -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        if let Some(widgets) = root.get_mut("widgets") {
            if !widgets.get("todo_items").is_some_and(|v| v.is_array()) {
                widgets["todo_items"] = serde_json::json!([]);
            }
            if let Some(todo_items) = widgets.get_mut("todo_items") {
                if let Some(arr) = todo_items.as_array_mut() {
                    let max_id = arr.iter()
                        .filter_map(|item| item.get("id").and_then(|id| id.as_i64()))
                        .max()
                        .unwrap_or(0) as i32;
                    let new_item = serde_json::json!({
                        "id": max_id + 1,
                        "text": text,
                        "completed": false
                    });
                    arr.push(new_item);
                }
            }
        }
    })
}

pub fn todo_toggle_item(id: i32) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(widgets) = root.get_mut("widgets") {
            if let Some(todo_items) = widgets.get_mut("todo_items") {
                if let Some(arr) = todo_items.as_array_mut() {
                    for item in arr {
                        if item.get("id").and_then(|val| val.as_i64()) == Some(id as i64) {
                            if let Some(completed) = item.get_mut("completed") {
                                let current = completed.as_bool().unwrap_or(false);
                                *completed = serde_json::json!(!current);
                            }
                        }
                    }
                }
            }
        }
    })
}

pub fn todo_delete_item(id: i32) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(widgets) = root.get_mut("widgets") {
            if let Some(todo_items) = widgets.get_mut("todo_items") {
                if let Some(arr) = todo_items.as_array_mut() {
                    arr.retain(|item| {
                        item.get("id").and_then(|val| val.as_i64()) != Some(id as i64)
                    });
                }
            }
        }
    })
}

pub fn todo_move_item(id: i32, is_up: bool) -> RavenSettings {
    patch_settings_json(|root| {
        if let Some(widgets) = root.get_mut("widgets") {
            if let Some(todo_items) = widgets.get_mut("todo_items") {
                if let Some(arr) = todo_items.as_array_mut() {
                    if let Some(idx) = arr.iter().position(|item| {
                        item.get("id").and_then(|val| val.as_i64()) == Some(id as i64)
                    }) {
                        if is_up && idx > 0 {
                            arr.swap(idx, idx - 1);
                        } else if !is_up && idx < arr.len() - 1 {
                            arr.swap(idx, idx + 1);
                        }
                    }
                }
            }
        }
    })
}

pub fn quotes_add_custom(text: &str, author: &str) -> RavenSettings {
    patch_settings_json(|root| {
        if !root.get("widgets").is_some_and(|v| v.is_object()) {
            root["widgets"] = serde_json::json!({});
        }
        if let Some(widgets) = root.get_mut("widgets") {
            if !widgets.get("quotes_custom_quotes").is_some_and(|v| v.is_array()) {
                widgets["quotes_custom_quotes"] = serde_json::json!([]);
            }
            if let Some(quotes_custom) = widgets.get_mut("quotes_custom_quotes") {
                if let Some(arr) = quotes_custom.as_array_mut() {
                    let formatted = format!("{}|{}", text.replace('|', ""), author.replace('|', ""));
                    arr.push(serde_json::json!(formatted));
                }
            }
        }
    })
}

fn patch_settings_json(update: impl FnOnce(&mut serde_json::Value)) -> RavenSettings {
    let start = std::time::Instant::now();
    let _ = RavenSettings::load(); // Ensure initialized
    let mut root = {
        let guard = SETTINGS_JSON_CACHE.lock().unwrap();
        guard.clone().unwrap_or_else(|| serde_json::json!({}))
    };
    update(&mut root);
    let updated: RavenSettings = serde_json::from_value(root.clone()).unwrap_or_default();
    if let Ok(mut guard) = SETTINGS_CACHE.lock() {
        *guard = Some(updated.clone());
    }
    if let Ok(mut guard) = SETTINGS_JSON_CACHE.lock() {
        *guard = Some(root.clone());
    }

    // Persist to settings.json disk file
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(&root) {
        let _ = fs::write(&path, raw);
    }

    println!("[SETTINGS-LOG] patch_settings_json completed in {:?}", start.elapsed());
    updated
}

fn nested_value_mut<'a>(
    root: &'a mut serde_json::Value,
    path: &[&str],
) -> Option<&'a mut serde_json::Value> {
    let mut cursor = root;
    for key in &path[..path.len().saturating_sub(1)] {
        if !cursor.get(key).is_some_and(|value| value.is_object()) {
            cursor[key] = serde_json::json!({});
        }
        cursor = cursor.get_mut(key)?;
    }
    let key = path.last()?;
    if cursor.get(key).is_none() {
        cursor[key] = serde_json::json!(null);
    }
    cursor.get_mut(key)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CalSettings {
    pub width: f32,
    pub height: f32,
}

impl Default for CalSettings {
    fn default() -> Self {
        Self {
            width: 900.0,
            height: 480.0,
        }
    }
}

fn empty_string_if_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutsSettings {
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub toggle_raven: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub tab_home: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub tab_media: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub tab_calendar: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub tab_clock: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub tab_drop: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub tab_capture: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub tab_stats: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub clipboard_history: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub media_play: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub media_next: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub media_prev: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub toggle_freeze: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub quick_screenshot: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub quick_record_toggle: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub open_settings: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub restart_raven: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub quit_raven: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub topbar_stats: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub topbar_volume: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub topbar_wifi: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub topbar_timer: String,
    #[serde(default, deserialize_with = "empty_string_if_null")]
    pub topbar_calendar: String,
}

impl Default for ShortcutsSettings {
    fn default() -> Self {
        Self {
            toggle_raven: "Alt+Space".to_string(),
            tab_home: String::new(),
            tab_media: String::new(),
            tab_calendar: String::new(),
            tab_clock: String::new(),
            tab_drop: String::new(),
            tab_capture: String::new(),
            tab_stats: String::new(),
            clipboard_history: String::new(),
            media_play: String::new(),
            media_next: String::new(),
            media_prev: String::new(),
            toggle_freeze: String::new(),
            quick_screenshot: String::new(),
            quick_record_toggle: String::new(),
            open_settings: String::new(),
            restart_raven: String::new(),
            quit_raven: String::new(),
            topbar_stats: String::new(),
            topbar_volume: String::new(),
            topbar_wifi: String::new(),
            topbar_timer: String::new(),
            topbar_calendar: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RavenAlertSettings {
    pub enabled: bool,
    pub monitor_charger_in: bool,
    pub monitor_charger_out: bool,
    pub monitor_low_battery: bool,
    pub monitor_unlock: bool,
    pub monitor_bluetooth: bool,
    pub monitor_keys: bool,
    pub monitor_volume_hud: bool,
    pub monitor_brightness_hud: bool,
    pub monitor_camera: bool,
    pub monitor_caffeine: bool,
    pub duration: u32,
    pub animation_style: String,
}

impl Default for RavenAlertSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            monitor_charger_in: true,
            monitor_charger_out: true,
            monitor_low_battery: true,
            monitor_unlock: true,
            monitor_bluetooth: true,
            monitor_keys: true,
            monitor_volume_hud: true,
            monitor_brightness_hud: true,
            monitor_camera: true,
            monitor_caffeine: true,
            duration: 3000,
            animation_style: "expressive".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct IntelligenceSettings {
    pub always_on_charging: bool,
    pub always_on_low_battery: bool,
    pub always_on_charging_mode: String,
    pub power_notifs: bool,
    pub bt_notifs: bool,
    pub system_notifs: bool,
    pub auto_accent: bool,
    pub privacy_mode: bool,
    pub bt_pct: bool,
    pub kb_notifs: bool,
    pub duration: u32,
    pub sentinel_kb: bool,
    pub sentinel_volume: bool,
    pub sentinel_brightness: bool,
    pub sentinel_power: bool,
}

impl Default for IntelligenceSettings {
    fn default() -> Self {
        Self {
            always_on_charging: true,
            always_on_low_battery: true,
            always_on_charging_mode: "bolt".to_string(),
            power_notifs: true,
            bt_notifs: true,
            system_notifs: true,
            auto_accent: true,
            privacy_mode: true,
            bt_pct: true,
            kb_notifs: true,
            duration: 3500,
            sentinel_kb: true,
            sentinel_volume: true,
            sentinel_brightness: true,
            sentinel_power: true,
        }
    }
}
