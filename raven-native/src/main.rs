#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod app;
use chrono::Timelike;
mod events;
mod graphics;
mod license;
mod motion;
mod renderer;
mod services;
mod settings;
mod widgets;
mod window;
mod monitor_math;
#[cfg(test)]
mod monitor_tests;
pub mod diagnostics;
use std::collections::{HashMap, HashSet};

slint::include_modules!();
use slint::Model;

enum ExtraWidgetWindow {
    Year(YearProgressWidgetWindow),
    Day(DayProgressWidgetWindow),
    Month(MonthProgressWidgetWindow),
    Media(MediaWidgetWindow),
    Notes(NotesWidgetWindow),
    Todo(TodoWidgetWindow),
    Quotes(QuotesWidgetWindow),
    Picture(PictureWidgetWindow),
    Video(VideoFrameWidgetWindow),
    Battery(BatteryPercentageWidgetWindow),
    CalendarFocus(CalendarFocusWidgetWindow),
    Apps(AppsContainerWidgetWindow),
    FocusScore(FocusScoreWidgetWindow),
    Streak(StreakWidgetWindow),
}

impl ExtraWidgetWindow {
    fn hide(&self) -> Result<(), slint::PlatformError> {
        match self {
            Self::Year(w) => w.hide(),
            Self::Day(w) => w.hide(),
            Self::Month(w) => w.hide(),
            Self::Media(w) => w.hide(),
            Self::Notes(w) => w.hide(),
            Self::Todo(w) => w.hide(),
            Self::Quotes(w) => w.hide(),
            Self::Picture(w) => w.hide(),
            Self::Video(w) => w.hide(),
            Self::Battery(w) => w.hide(),
            Self::CalendarFocus(w) => w.hide(),
            Self::Apps(w) => w.hide(),
            Self::FocusScore(w) => w.hide(),
            Self::Streak(w) => w.hide(),
        }
    }

    fn hwnd(&self) -> Option<windows::Win32::Foundation::HWND> {
        use raw_window_handle::HasWindowHandle;
        let raw = match self {
            Self::Year(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Day(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Month(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Media(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Notes(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Todo(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Quotes(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Picture(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Video(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Battery(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::CalendarFocus(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Apps(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::FocusScore(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
            Self::Streak(w) => w.window().window_handle().window_handle().ok()?.as_raw(),
        };
        if let raw_window_handle::RawWindowHandle::Win32(win32) = raw {
            Some(windows::Win32::Foundation::HWND(win32.hwnd.get() as _))
        } else {
            None
        }
    }
}

fn get_window_hwnd(window: &slint::Window) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::HasWindowHandle;
    let w_handle = window.window_handle();
    let handle = w_handle.window_handle().ok()?;
    if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
        Some(windows::Win32::Foundation::HWND(win32.hwnd.get() as _))
    } else {
        None
    }
}

fn drag_widget_window(window: &slint::Window) {
    if crate::widgets::WIDGET_DRAG_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    use raw_window_handle::HasWindowHandle;
    if let Ok(handle) = window.window_handle().window_handle() {
        if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
            let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
            unsafe {
                let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    hwnd, 161,
                    windows::Win32::Foundation::WPARAM(2),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
        }
    }
}

fn fetch_profile_picture_bytes(url: &str) -> Option<Vec<u8>> {
    if url.starts_with("data:image/") {
        let comma_idx = url.find(',')?;
        let b64_part = &url[comma_idx + 1..];
        let clean_b64 = b64_part.trim();
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(clean_b64).ok()
    } else {
        let response = ureq::get(url).call().ok()?;
        let mut bytes = Vec::new();
        use std::io::Read;
        response.into_reader().read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }
}

fn load_image_from_bytes(bytes: &[u8]) -> Option<slint::Image> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
        rgba.as_raw(),
        width,
        height,
    );
    Some(slint::Image::from_rgba8(buffer))
}

fn load_profile_picture(settings_ui: &SettingsWindow, pic_url: &str) {
    let pic_url = pic_url.to_string();
    let settings_ui_weak = settings_ui.as_weak();
    std::thread::spawn(move || {
        if let Some(bytes) = fetch_profile_picture_bytes(&pic_url) {
            // Save to disk cache
            if let Some(mut path) = dirs::config_dir() {
                path.push("RavenIsland");
                let _ = std::fs::create_dir_all(&path);
                path.push("profile_picture.png");
                let _ = std::fs::write(path, &bytes);
            }

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(settings_ui) = settings_ui_weak.upgrade() {
                    if let Some(slint_img) = load_image_from_bytes(&bytes) {
                        settings_ui.set_account_picture_image(slint_img);
                        settings_ui.set_account_has_picture_image(true);
                    }
                }
            });
        }
    });
}

fn apply_license_status(
    ui: &Pill,
    settings_ui: &SettingsWindow,
    status: &license::LicenseStatus,
) {
    let locked = license::is_premium_locked(status);
    let label = license::status_label(status);
    let trial_label = license_trial_countdown_label(status);
    ui.set_premium_locked(locked);
    settings_ui.set_premium_locked(locked);
    settings_ui.set_force_trial_expired_preview(status.force_trial_expired_preview);
    settings_ui.set_license_status_label(label.into());
    settings_ui.set_license_trial_countdown_label(trial_label.into());
    settings_ui.set_account_email_label(status.account_email.clone().unwrap_or_default().into());
    settings_ui.set_account_name_label(status.account_name.clone().unwrap_or_default().into());
    settings_ui.set_account_username_label(status.account_username.clone().unwrap_or_default().into());

    if let Some(pic_url) = status.account_picture.clone() {
        if !pic_url.is_empty() {
            load_profile_picture(settings_ui, &pic_url);
        } else {
            settings_ui.set_account_has_picture_image(false);
        }
    } else {
        settings_ui.set_account_has_picture_image(false);
    }

    if let Some(message) = &status.message {
        settings_ui.set_license_action_message(message.clone().into());
    } else if status.status == "paid_active" {
        settings_ui.set_license_action_message("License activated successfully.".into());
    } else {
        settings_ui.set_license_action_message("".into());
    }
}

fn license_trial_countdown_label(status: &license::LicenseStatus) -> String {
    if status.status == "paid_active" {
        return "Lifetime license active".to_string();
    }

    if status.force_trial_expired_preview {
        return "Preview mode: trial shown as expired".to_string();
    }

    let Some(expires_at) = status.trial_expires_at.as_deref() else {
        return "Trial countdown will appear after first license check".to_string();
    };

    let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expires_at) else {
        return "Trial countdown unavailable".to_string();
    };

    let now = chrono::Utc::now();
    let remaining = expires.with_timezone(&chrono::Utc) - now;
    if remaining.num_seconds() <= 0 {
        return "Trial expired".to_string();
    }

    let days = remaining.num_days();
    let hours = remaining.num_hours() % 24;
    if days > 1 {
        format!("{days} days {hours} hours left in trial")
    } else if days == 1 {
        format!("1 day {hours} hours left in trial")
    } else {
        let hours_total = remaining.num_hours().max(1);
        format!("{hours_total} hours left in trial")
    }
}

fn open_external_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(0x08000000)
            .spawn();
    }
}

fn account_token_from_args() -> Option<String> {
    for arg in std::env::args().skip(1) {
        let value = arg.trim();
        if !value.to_ascii_lowercase().starts_with("ravennotch://auth") {
            continue;
        }
        let query = value.split_once('?').map(|(_, query)| query).unwrap_or("");
        for part in query.split('&') {
            let Some((key, val)) = part.split_once('=') else {
                continue;
            };
            if key == "token" && !val.trim().is_empty() {
                return urlencoding::decode(val).ok().map(|decoded| decoded.into_owned());
            }
        }
    }
    None
}

fn sync_calendar_widget_date(w: &StreakWidgetWindow) {
    use chrono::{Datelike, Local, NaiveDate};

    let today = Local::now().date_naive();
    let year = today.year();
    let month = today.month();
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(today);
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap_or(first_day)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap_or(first_day)
    };
    let days_in_month = (next_month - chrono::Duration::days(1)).day();
    let start_offset = first_day.weekday().num_days_from_sunday() as usize;

    let weekday_name = today.format("%A").to_string().to_uppercase();
    let month_name = today.format("%B").to_string();
    w.set_today_day_name(weekday_name.into());
    w.set_today_month_name(month_name.into());
    w.set_today_day_number(format!("{:02}", today.day()).into());
    w.set_selected_month(month as i32);
    w.set_selected_year(year);
    w.set_month_name(today.format("%B").to_string().into());
    w.set_view_mode(0);

    let mut days = Vec::with_capacity(42);
    for index in 0..42 {
        let day_num = index as i32 - start_offset as i32 + 1;
        if day_num >= 1 && day_num <= days_in_month as i32 {
            days.push(SlintStreakDay {
                label: "current".into(),
                day_number: day_num.to_string().into(),
                focus: false,
                today: day_num as u32 == today.day(),
                missed: false,
            });
        } else {
            days.push(SlintStreakDay {
                label: "".into(),
                day_number: "".into(),
                focus: false,
                today: false,
                missed: false,
            });
        }
    }
    w.set_mini_days(std::rc::Rc::new(slint::VecModel::from(days)).into());
}

fn format_focus_minutes(minutes: i32) -> String {
    let mins = minutes.max(0);
    if mins >= 60 {
        let hours = mins / 60;
        let rem = mins % 60;
        if rem == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, rem)
        }
    } else {
        format!("{}m", mins)
    }
}

fn focus_score_widget_data(settings: &settings::RavenSettings) -> (String, i32, Vec<SlintFocusScoreRow>) {
    let mut presets: Vec<String> = Vec::new();
    for goal in &settings.focus_goal_presets {
        let trimmed = goal.trim();
        if !trimmed.is_empty()
            && !presets.iter().any(|existing| existing.eq_ignore_ascii_case(trimmed))
        {
            presets.push(trimmed.to_string());
        }
    }

    let mut totals: Vec<(String, i32)> = presets
        .into_iter()
        .map(|goal| {
            let minutes = settings
                .focus_sessions
                .iter()
                .filter(|session| session.goal.trim().eq_ignore_ascii_case(goal.trim()))
                .map(|session| session.duration_mins.max(0))
                .sum();
            (goal, minutes)
        })
        .collect();
    totals.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let total_minutes: i32 = totals.iter().map(|(_, mins)| *mins).sum();
    let goal_minutes = (settings.widgets.focus_score_goal_hours.max(0.25) * 60.0).round() as i32;
    let score = if goal_minutes > 0 {
        ((total_minutes as f64 / goal_minutes as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as i32
    } else {
        0
    };

    let max_row_minutes = totals.iter().map(|(_, mins)| *mins).max().unwrap_or(1).max(1);
    let accents = [
        slint::Color::from_argb_u8(255, 180, 92, 255),
        slint::Color::from_argb_u8(255, 34, 211, 238),
        slint::Color::from_argb_u8(255, 255, 159, 28),
        slint::Color::from_argb_u8(255, 52, 199, 89),
        slint::Color::from_argb_u8(255, 61, 165, 255),
    ];
    let rows = totals
        .into_iter()
        .enumerate()
        .map(|(idx, (goal, minutes))| SlintFocusScoreRow {
            goal: slint::SharedString::from(goal),
            duration_label: slint::SharedString::from(format_focus_minutes(minutes)),
            progress: (minutes as f32 / max_row_minutes as f32).clamp(0.0, 1.0),
            accent: accents[idx % accents.len()],
        })
        .collect();

    (format_focus_minutes(total_minutes), score, rows)
}

fn render_focus_score_ring(score: i32) -> slint::Image {
    let size = 144u32;
    let center = size as f32 / 2.0;
    let radius = 52.0f32;
    let half_thickness = 7.0f32;
    let progress = (score as f32).clamp(0.0, 100.0) / 100.0;
    let start_angle = -std::f32::consts::FRAC_PI_2;
    let end_angle = start_angle + progress * std::f32::consts::TAU;
    let start_point = (
        center + radius * start_angle.cos(),
        center + radius * start_angle.sin(),
    );
    let end_point = (
        center + radius * end_angle.cos(),
        center + radius * end_angle.sin(),
    );

    let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(size, size);
    for (index, pixel) in buffer.make_mut_slice().iter_mut().enumerate() {
        let x = (index as u32 % size) as f32 + 0.5;
        let y = (index as u32 / size) as f32 + 0.5;
        let dx = x - center;
        let dy = y - center;
        let distance = (dx * dx + dy * dy).sqrt();
        let on_track = (distance - radius).abs() <= half_thickness;
        let mut angle = dy.atan2(dx) - start_angle;
        if angle < 0.0 {
            angle += std::f32::consts::TAU;
        }
        let on_progress_arc =
            on_track && progress > 0.0 && angle / std::f32::consts::TAU <= progress;
        let start_cap = progress > 0.0
            && ((x - start_point.0).powi(2) + (y - start_point.1).powi(2)).sqrt()
                <= half_thickness;
        let end_cap = progress > 0.0
            && ((x - end_point.0).powi(2) + (y - end_point.1).powi(2)).sqrt()
                <= half_thickness;

        *pixel = if on_progress_arc || start_cap || end_cap {
            slint::Rgba8Pixel::new(166, 108, 255, 255)
        } else if on_track {
            slint::Rgba8Pixel::new(78, 45, 136, 150)
        } else {
            slint::Rgba8Pixel::new(0, 0, 0, 0)
        };
    }
    slint::Image::from_rgba8(buffer)
}

fn sync_focus_data_to_ui(ui: &Pill, settings: &settings::RavenSettings) {
    let presets: Vec<slint::SharedString> = settings
        .focus_goal_presets
        .iter()
        .filter_map(|goal| {
            let trimmed = goal.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(slint::SharedString::from(trimmed))
            }
        })
        .collect();
    ui.set_focus_goal_presets(std::rc::Rc::new(slint::VecModel::from(presets)).into());

    let history: Vec<SlintFocusSession> = settings
        .focus_sessions
        .iter()
        .map(|session| SlintFocusSession {
            goal: session.goal.clone().into(),
            duration_mins: session.duration_mins,
            completed_at: session.completed_at.clone().into(),
        })
        .collect();
    ui.set_focus_session_history(std::rc::Rc::new(slint::VecModel::from(history)).into());
}

#[link(name = "winmm")]
extern "system" {
    fn PlaySoundW(
        pszSound: *const u16,
        hmod: *mut std::ffi::c_void,
        fdwSound: u32,
    ) -> i32;

    fn mciSendStringW(
        lpstrCommand: *const u16,
        lpstrReturnString: *mut u16,
        uReturnLength: u32,
        hwndCallback: windows::Win32::Foundation::HWND,
    ) -> i32;
}

pub fn play_audio_file(path_str: &str) {
    let clean_path = path_str.trim_start_matches(r#"\\?\"#).replace('/', "\\");
    let lower_path = clean_path.to_lowercase();
    if lower_path.ends_with(".mp3") {
        // Play MP3 using MCI SendString in a background thread
        let path_str_cloned = clean_path.to_string();
        std::thread::spawn(move || unsafe {
            // Encode command to close any existing my_audio
            let close_cmd = "close my_audio\0".encode_utf16().collect::<Vec<u16>>();
            let _ = mciSendStringW(close_cmd.as_ptr(), std::ptr::null_mut(), 0, windows::Win32::Foundation::HWND(0));

            // Encode command to open the file: open "path" type mpegvideo alias my_audio
            let open_cmd = format!("open \"{}\" type mpegvideo alias my_audio\0", path_str_cloned);
            let wide_open: Vec<u16> = open_cmd.encode_utf16().collect();
            let open_res = mciSendStringW(wide_open.as_ptr(), std::ptr::null_mut(), 0, windows::Win32::Foundation::HWND(0));
            println!("[SOUND] mciSendStringW open path: {}, result: {}", path_str_cloned, open_res);

            if open_res == 0 {
                let play_cmd = "play my_audio from 0\0".encode_utf16().collect::<Vec<u16>>();
                let play_res = mciSendStringW(play_cmd.as_ptr(), std::ptr::null_mut(), 0, windows::Win32::Foundation::HWND(0));
                println!("[SOUND] mciSendStringW play result: {}", play_res);
            }
        });
    } else {
        // Play WAV using PlaySoundW in a spawned background thread using SND_SYNC to prevent early cutoff
        let path_str_cloned = clean_path.to_string();
        std::thread::spawn(move || {
            let mut wide_path: Vec<u16> = path_str_cloned.encode_utf16().collect();
            wide_path.push(0);

            let flags = 0x00020000 | 0x00000002; // SND_FILENAME | SND_NODEFAULT (SND_SYNC is 0)
            unsafe {
                let res = PlaySoundW(wide_path.as_ptr(), std::ptr::null_mut(), flags);
                println!("[SOUND] PlaySoundW on path: {}, result: {}", path_str_cloned, res);
            }
        });
    }
}

pub fn play_sound_by_name(name: &str) {
    play_sound_by_name_bypass(name, false);
}

pub fn play_sound_by_name_bypass(name: &str, is_preview: bool) {
    let settings = settings::RavenSettings::load();
    
    // If not a preview, respect global enabled toggle
    if !is_preview && !settings.sounds.enabled {
        return;
    }

    // If not a preview, respect specific toggle and raven_alert config
    if !is_preview {
        let specific_enabled = match name {
            "timer_complete" => settings.sounds.timer_complete,
            "stopwatch" => settings.sounds.stopwatch,
            "battery_low" => settings.sounds.battery_low,
            "charger_connected" => settings.sounds.charger_connected,
            "charger_disconnected" => settings.sounds.charger_disconnected,
            "capslock_on" => settings.sounds.capslock_on,
            "capslock_off" => settings.sounds.capslock_off,
            "unlock" => settings.sounds.unlock,
            _ => false,
        };
        if !specific_enabled {
            return;
        }

        // Check raven alert settings
        match name {
            "charger_connected" | "charger_disconnected" | "battery_low" | "unlock" | "capslock_on" | "capslock_off" => {
                if !settings.raven_alert.enabled {
                    return;
                }
                let monitor_enabled = match name {
                    "charger_connected" => settings.raven_alert.monitor_charger_in,
                    "charger_disconnected" => settings.raven_alert.monitor_charger_out,
                    "battery_low" => settings.raven_alert.monitor_low_battery,
                    "unlock" => settings.raven_alert.monitor_unlock,
                    "capslock_on" | "capslock_off" => settings.raven_alert.monitor_keys,
                    _ => false,
                };
                if !monitor_enabled {
                    return;
                }
            }
            _ => {}
        }
    }

    // Check if custom path is set for this sound, and play that instead
    let custom_path = match name {
        "timer_complete" => &settings.sounds.custom_timer_complete_path,
        "stopwatch" => &settings.sounds.custom_stopwatch_path,
        "battery_low" => &settings.sounds.custom_battery_low_path,
        "charger_connected" => &settings.sounds.custom_charger_connected_path,
        "charger_disconnected" => &settings.sounds.custom_charger_disconnected_path,
        "capslock_on" => &settings.sounds.custom_capslock_on_path,
        "capslock_off" => &settings.sounds.custom_capslock_off_path,
        "unlock" => &settings.sounds.custom_unlock_path,
        _ => "",
    };

    if !custom_path.is_empty() && std::path::Path::new(custom_path).exists() {
        println!("[SOUND] Playing custom {} sound: {}", name, custom_path);
        play_audio_file(custom_path);
        return;
    } else if !custom_path.is_empty() {
        println!("[SOUND] Custom sound path set but file not found (falling back): {}", custom_path);
    }

    // Default built-in sounds mapping
    let filename = match name {
        "timer_complete" => "confirmation_002.wav",
        "stopwatch" => "click_003.wav",
        "battery_low" => "error_003.wav",
        "charger_connected" => "maximize_004.wav",
        "charger_disconnected" => "minimize_004.wav",
        "capslock_on" => "open_003.wav",
        "capslock_off" => "close_003.wav",
        "unlock" => "confirmation_004.wav",
        _ => return,
    };

    // Locate the built-in sound file
    let paths_to_try = vec![
        std::path::PathBuf::from("ui").join("assets").join("sounds").join(filename),
        std::path::PathBuf::from("assets").join("sounds").join(filename),
    ];

    let mut wav_path = None;
    for path in paths_to_try {
        if path.exists() {
            wav_path = Some(path);
            break;
        }
    }

    if wav_path.is_none() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let paths_to_try_exe = vec![
                    exe_dir.join("ui").join("assets").join("sounds").join(filename),
                    exe_dir.join("assets").join("sounds").join(filename),
                    exe_dir.parent().and_then(|p| p.parent()).map(|p| p.join("ui").join("assets").join("sounds").join(filename)).unwrap_or_default(),
                ];
                for path in paths_to_try_exe {
                    if path.exists() {
                        wav_path = Some(path);
                        break;
                    }
                }
            }
        }
    }

    if let Some(path) = wav_path {
        // Dynamic canonicalization to absolute path to guarantee async Win32 thread success
        if let Ok(abs_path) = path.canonicalize() {
            if let Some(path_str) = abs_path.to_str() {
                let clean_path = path_str.trim_start_matches(r#"\\?\"#);
                play_audio_file(clean_path);
            }
        } else if let Some(path_str) = path.to_str() {
            play_audio_file(path_str);
        }
    } else {
        println!("[SOUND] Built-in sound file {} not found!", filename);
    }
}


pub fn pick_mp3_file(title: &str) -> Option<String> {
    use std::process::Command;
    // Powershell script to open an OpenFileDialog completely hidden, visual-styled for high-DPI, and return the path
    let script = format!(
        r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms
$dpiApi = Add-Type -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();' -Name "Win32Dpi" -Namespace Win32Functions -PassThru
[void]$dpiApi::SetProcessDPIAware()
[System.Windows.Forms.Application]::EnableVisualStyles()
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Filter = "Audio Files (*.mp3;*.wav)|*.mp3;*.wav|MP3 Files (*.mp3)|*.mp3|WAV Files (*.wav)|*.wav"
$dialog.Title = "{}"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
    Write-Output $dialog.FileName
}}
"#,
        title
    );
    let output = Command::new("powershell")
        .args(&["-ExecutionPolicy", "Bypass", "-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .output()
        .ok()?;
    
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    } else {
        println!("[SOUND] pick_mp3_file PowerShell failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    None
}

use crate::motion::{MotionState, NotchPhase, HOME_HEIGHT, HOME_WIDTH};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub static HOVER_ENABLED: AtomicBool = AtomicBool::new(true);
pub static HOVER_OPEN_DELAY_MS: AtomicU32 = AtomicU32::new(0);
pub static HOVER_CLOSE_DELAY_MS: AtomicU32 = AtomicU32::new(60);
pub static APPEARANCE_AUTO_HIDE: AtomicBool = AtomicBool::new(false);
pub static APPEARANCE_AUTO_HIDE_ON_FULLSCREEN: AtomicBool = AtomicBool::new(false);
pub static SETTINGS_WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

struct FocusTimerRuntime {
    duration_secs: std::cell::Cell<u64>,
    remaining_secs: std::cell::Cell<u64>,
    running: std::cell::Cell<bool>,
    last_tick: std::cell::Cell<std::time::Instant>,
}

impl FocusTimerRuntime {
    fn new(minutes: i32) -> Self {
        let duration_secs = minutes.clamp(1, 180) as u64 * 60;
        Self {
            duration_secs: std::cell::Cell::new(duration_secs),
            remaining_secs: std::cell::Cell::new(duration_secs),
            running: std::cell::Cell::new(false),
            last_tick: std::cell::Cell::new(std::time::Instant::now()),
        }
    }

    fn set_minutes(&self, minutes: i32) {
        let duration_secs = minutes.clamp(1, 180) as u64 * 60;
        self.duration_secs.set(duration_secs);
        self.remaining_secs.set(duration_secs);
        self.running.set(false);
        self.last_tick.set(std::time::Instant::now());
    }

    fn toggle(&self) {
        if self.remaining_secs.get() == 0 {
            self.remaining_secs.set(self.duration_secs.get());
        }
        self.running.set(!self.running.get());
        self.last_tick.set(std::time::Instant::now());
    }

    fn reset(&self) {
        self.remaining_secs.set(self.duration_secs.get());
        self.running.set(false);
        self.last_tick.set(std::time::Instant::now());
    }

    fn tick(&self) -> bool {
        if !self.running.get() {
            return false;
        }
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_tick.get()).as_secs();
        if elapsed == 0 {
            return false;
        }
        self.last_tick.set(now);
        let remaining = self.remaining_secs.get();
        if elapsed >= remaining {
            self.remaining_secs.set(0);
            self.running.set(false);
            true
        } else {
            self.remaining_secs.set(remaining - elapsed);
            false
        }
    }
}

fn widget_instance_opacity(settings: &settings::RavenSettings, instance: &settings::WidgetInstanceSettings) -> f32 {
    if instance.opacity > 0 {
        (instance.opacity.clamp(10, 100) as f32) / 100.0
    } else {
        settings.widgets.opacity
    }
}

fn instance_data_str<'a>(
    instance: &'a settings::WidgetInstanceSettings,
    key: &str,
    fallback: &'a str,
) -> &'a str {
    instance
        .data
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or(fallback)
}

fn instance_data_todo_items(
    settings: &settings::RavenSettings,
    instance: &settings::WidgetInstanceSettings,
) -> Vec<TodoItem> {
    let data_items = instance
        .data
        .get("todo_items")
        .and_then(|value| value.as_array());
    if let Some(items) = data_items {
        items
            .iter()
            .filter_map(|item| {
                let id = item.get("id").and_then(|v| v.as_i64())? as i32;
                let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let completed = item.get("completed").and_then(|v| v.as_bool()).unwrap_or(false);
                if settings.widgets.todo_hide_completed && completed {
                    None
                } else {
                    Some(TodoItem { id, text: text.into(), completed })
                }
            })
            .collect()
    } else {
        settings.widgets.todo_items.iter()
            .filter(|item| !(settings.widgets.todo_hide_completed && item.completed))
            .map(|item| TodoItem {
                id: item.id,
                text: item.text.clone().into(),
                completed: item.completed,
            })
            .collect()
    }
}

fn instance_data_app_items(
    settings: &settings::RavenSettings,
    instance: &settings::WidgetInstanceSettings,
) -> Vec<SlintAppShortcut> {
    let data_items = instance
        .data
        .get("apps_container_items")
        .and_then(|value| value.as_array());
    let from_json = |item: &serde_json::Value| {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let icon = crate::window::get_file_icon(path).unwrap_or_default();
        let has_icon = icon.size().width > 0 && icon.size().height > 0;
        SlintAppShortcut {
            name: name.into(),
            path: path.into(),
            icon,
            has_icon,
        }
    };

    if let Some(items) = data_items {
        items.iter().map(from_json).collect()
    } else {
        settings.widgets.apps_container_items.iter().map(|item| {
            let icon = crate::window::get_file_icon(&item.path).unwrap_or_default();
            let has_icon = icon.size().width > 0 && icon.size().height > 0;
            SlintAppShortcut {
                name: item.name.as_str().into(),
                path: item.path.as_str().into(),
                icon,
                has_icon,
            }
        }).collect()
    }
}

fn sync_extra_widget_window(
    window: &ExtraWidgetWindow,
    settings: &settings::RavenSettings,
    instance: &settings::WidgetInstanceSettings,
    focus_runtime: &Rc<FocusTimerRuntime>,
) {
    let opacity = widget_instance_opacity(settings, instance);
    let locked = instance.locked || settings.widgets.locked;
    let radius = settings.widgets.stats_border_radius as i32;

    match window {
        ExtraWidgetWindow::Year(w) => {
            update_year_progress_widget_properties(w);
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Day(w) => {
            update_day_progress_widget_properties(w);
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Month(w) => {
            update_month_progress_widget_properties(w);
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Media(w) => {
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Notes(w) => {
            w.set_notes_text(instance_data_str(instance, "notes_text", &settings.widgets.notes_text).into());
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Todo(w) => {
            w.set_todo_items(Rc::new(slint::VecModel::from(instance_data_todo_items(settings, instance))).into());
            w.set_accent_color(parse_hex_color(&settings.widgets.todo_accent_color));
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Quotes(w) => {
            if w.get_quote_text().is_empty() {
                let mut all_quotes: Vec<(String, String)> = DEFAULT_QUOTES.iter()
                    .map(|(q, a)| (q.to_string(), a.to_string()))
                    .collect();
                for custom in &settings.widgets.quotes_custom_quotes {
                    let parts: Vec<&str> = custom.split('|').collect();
                    if parts.len() == 2 {
                        all_quotes.push((parts[0].to_string(), parts[1].to_string()));
                    } else if parts.len() == 1 {
                        all_quotes.push((parts[0].to_string(), "Unknown".to_string()));
                    }
                }
                if let Some((quote, author)) = all_quotes.first() {
                    w.set_quote_text(quote.clone().into());
                    w.set_quote_author(author.clone().into());
                }
            }
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Picture(w) => {
            w.set_show_camera_overlay(false);
            let path = instance_data_str(instance, "picture_path", &settings.widgets.picture_path).to_string();
            if w.get_picture_path().as_str() != path {
                w.set_picture_path(path.clone().into());
                if !path.is_empty() {
                    if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&path)) {
                        w.set_picture_img(img);
                        w.set_has_picture(true);
                    } else {
                        w.set_has_picture(false);
                    }
                } else {
                    w.set_has_picture(false);
                }
            }
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Video(w) => {
            w.set_show_video_overlay(false);
            let path = instance_data_str(instance, "video_path", &settings.widgets.video_path).to_string();
            w.set_video_path(path.clone().into());
            w.set_has_video(!path.is_empty());
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Battery(w) => {
            if let Some((pct, charging)) = read_live_battery_status() {
                w.set_battery_pct(pct);
                w.set_is_charging(charging);
                w.set_progress_ring_img(render_battery_progress_ring(pct));
            }
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::CalendarFocus(w) => {
            update_calendar_focus_widget_properties(w, focus_runtime);
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Apps(w) => {
            w.set_app_items(Rc::new(slint::VecModel::from(instance_data_app_items(settings, instance))).into());
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::FocusScore(w) => {
            let (total_label, score, rows) = focus_score_widget_data(settings);
            w.set_total_focus_label(total_label.into());
            w.set_focus_score(score);
            w.set_score_ring_img(render_focus_score_ring(score));
            w.set_preset_rows(Rc::new(slint::VecModel::from(rows)).into());
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
        ExtraWidgetWindow::Streak(w) => {
            w.set_streak_name(instance_data_str(instance, "streak_name", &settings.widgets.streak_name).into());
            sync_calendar_widget_date(w);
            w.set_is_locked(locked);
            w.set_bg_opacity(opacity);
            w.set_border_radius_val(radius);
        }
    }
}

fn position_extra_widget_window(
    window: &ExtraWidgetWindow,
    settings: &settings::RavenSettings,
    instance: &settings::WidgetInstanceSettings,
) {
    if let Some(hwnd) = window.hwnd() {
        let width = instance.width.max(120);
        let height = instance.height.max(80);
        println!("[WIDGET-DEBUG] position_extra_widget_window: HWND {:?} | id='{}' | target (x={}, y={}, w={}, h={})",
                 hwnd, instance.id, instance.x, instance.y, width, height);
        unsafe {
            widgets::setup_widget_window(hwnd, instance.locked || settings.widgets.click_through, false);
            widgets::set_widget_click_through(hwnd, instance.locked || settings.widgets.click_through);
            widgets::apply_widget_topmost_state(
                hwnd,
                settings::is_widget_always_on_top(settings, &instance.id),
            );
            widgets::position_widget_window_from_left(hwnd, instance.x, instance.y, width, height);
        }
    } else {
        println!("[WIDGET-DEBUG] position_extra_widget_window: hwnd is None for id='{}'", instance.id);
    }
}

use std::sync::Mutex;
use windows::Win32::UI::WindowsAndMessaging::{HHOOK, KBDLLHOOKSTRUCT, CallNextHookEx, UnhookWindowsHookEx, SetWindowsHookExW, WH_KEYBOARD, WM_KEYDOWN, WM_SYSKEYDOWN};
use windows::Win32::Foundation::LRESULT;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN};

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> u32;
}

struct ShortcutHookState {
    h_hook: Option<HHOOK>,
    shortcut_id: Option<String>,
    settings_ui: Option<slint::Weak<SettingsWindow>>,
    is_low_level: bool,
}

static SHORTCUT_HOOK: Mutex<ShortcutHookState> = Mutex::new(ShortcutHookState {
    h_hook: None,
    shortcut_id: None,
    settings_ui: None,
    is_low_level: false,
});

fn is_modifier_key(vk: u32) -> bool {
    matches!(vk, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
}

fn vk_to_string(vk: u32) -> Option<String> {
    let s = match vk {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x1B => "Escape",
        0x20 => "Space",
        0x21 => "PageUp",
        0x22 => "PageDown",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2D => "Insert",
        0x2E => "Delete",
        0x30..=0x39 => {
            let c = (b'0' + (vk - 0x30) as u8) as char;
            return Some(c.to_string());
        }
        0x41..=0x5A => {
            let c = (b'A' + (vk - 0x41) as u8) as char;
            return Some(c.to_string());
        }
        0x70..=0x87 => {
            let f_num = vk - 0x70 + 1;
            return Some(format!("F{}", f_num));
        }
        0xBA => ";",
        0xBB => "Plus",
        0xBC => "Comma",
        0xBD => "Minus",
        0xBE => "Period",
        0xBF => "Slash",
        0xC0 => "Backtick",
        0xDB => "BracketLeft",
        0xDC => "Backslash",
        0xDD => "BracketRight",
        0xDE => "Quote",
        _ => return None,
    };
    Some(s.to_string())
}

unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if code >= 0 {
        let is_low_level = {
            let state = SHORTCUT_HOOK.lock().unwrap();
            state.is_low_level
        };

        let (msg_is_down, vk) = if is_low_level {
            let msg = wparam.0 as u32;
            let hook_struct = *(lparam.0 as *const KBDLLHOOKSTRUCT);
            (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN, hook_struct.vkCode)
        } else {
            // WH_KEYBOARD: transition state is bit 31 of lparam (0 for key down, 1 for key up)
            let is_down = (lparam.0 & (1 << 31)) == 0;
            (is_down, wparam.0 as u32)
        };

        if msg_is_down {

            if !is_modifier_key(vk) {
                let mut mods = Vec::new();
                
                let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
                let alt_down = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
                let shift_down = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
                let win_down = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000) != 0 
                    || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000) != 0;

                if ctrl_down {
                    mods.push("Control");
                }
                if alt_down {
                    mods.push("Alt");
                }
                if shift_down {
                    mods.push("Shift");
                }
                if win_down {
                    mods.push("Win");
                }

                if let Some(key_str) = vk_to_string(vk) {
                    mods.push(&key_str);
                    let shortcut_str = mods.join("+");
                    println!("[HOOK] Captured shortcut: {}", shortcut_str);

                    let (shortcut_id, weak_ui, h_hook) = {
                        let mut state = SHORTCUT_HOOK.lock().unwrap();
                        let id = state.shortcut_id.clone();
                        let ui = state.settings_ui.clone();
                        let hook = state.h_hook.take();
                        state.shortcut_id = None;
                        state.settings_ui = None;
                        (id, ui, hook)
                    };

                    if let (Some(shortcut_id), Some(weak_ui)) = (shortcut_id, weak_ui) {
                        if let Some(h) = h_hook {
                            unsafe { let _ = UnhookWindowsHookEx(h); }
                        }

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = weak_ui.upgrade() {
                                let json_key = match shortcut_id.as_str() {
                                    "toggle_raven"        => Some(("shortcuts", "toggle_raven")),
                                    "tab_home"            => Some(("shortcuts", "tab_home")),
                                    "tab_media"           => Some(("shortcuts", "tab_media")),
                                    "tab_calendar"        => Some(("shortcuts", "tab_calendar")),
                                    "tab_clock"           => Some(("shortcuts", "tab_clock")),
                                    "tab_drop"            => Some(("shortcuts", "tab_drop")),
                                    "tab_capture"         => Some(("shortcuts", "tab_capture")),
                                    "tab_stats"           => Some(("shortcuts", "tab_stats")),
                                    "media_play"          => Some(("shortcuts", "media_play")),
                                    "media_next"          => Some(("shortcuts", "media_next")),
                                    "media_prev"          => Some(("shortcuts", "media_prev")),
                                    "toggle_freeze"       => Some(("shortcuts", "toggle_freeze")),
                                    "quick_screenshot"    => Some(("shortcuts", "quick_screenshot")),
                                    "quick_record_toggle" => Some(("shortcuts", "quick_record_toggle")),
                                    "open_settings"       => Some(("shortcuts", "open_settings")),
                                    "restart_raven"       => Some(("shortcuts", "restart_raven")),
                                    "quit_raven"          => Some(("shortcuts", "quit_raven")),
                                    _ => None,
                                };

                                if let Some((sec, key)) = json_key {
                                    let s_settings = crate::settings::set_string(&[sec, key], &shortcut_str);
                                    let main_hwnd_val = crate::window::PILL_HWND.load(std::sync::atomic::Ordering::SeqCst);
                                    if main_hwnd_val != 0 {
                                        let main_hwnd = windows::Win32::Foundation::HWND(main_hwnd_val as _);
                                        unsafe {
                                            crate::window::register_raven_hotkeys(main_hwnd, &s_settings);
                                        }
                                    }
                                }

                                let shared_str: slint::SharedString = shortcut_str.into();
                                match shortcut_id.as_str() {
                                    "toggle_raven"        => ui.set_shortcut_toggle_raven(shared_str),
                                    "tab_home"            => ui.set_shortcut_tab_home(shared_str),
                                    "tab_media"           => ui.set_shortcut_tab_media(shared_str),
                                    "tab_calendar"        => ui.set_shortcut_tab_calendar(shared_str),
                                    "tab_clock"           => ui.set_shortcut_tab_clock(shared_str),
                                    "tab_drop"            => ui.set_shortcut_tab_drop(shared_str),
                                    "tab_capture"         => ui.set_shortcut_tab_capture(shared_str),
                                    "tab_stats"           => ui.set_shortcut_tab_stats(shared_str),
                                    "media_play"          => ui.set_shortcut_media_play(shared_str),
                                    "media_next"          => ui.set_shortcut_media_next(shared_str),
                                    "media_prev"          => ui.set_shortcut_media_prev(shared_str),
                                    "toggle_freeze"       => ui.set_shortcut_toggle_freeze(shared_str),
                                    "quick_screenshot"    => ui.set_shortcut_quick_screenshot(shared_str),
                                    "quick_record_toggle" => ui.set_shortcut_quick_record_toggle(shared_str),
                                    "open_settings"       => ui.set_shortcut_open_settings(shared_str),
                                    "restart_raven"       => ui.set_shortcut_restart_raven(shared_str),
                                    "quit_raven"          => ui.set_shortcut_quit_raven(shared_str),
                                    _ => {}
                                }
                                ui.set_recording_shortcut_key("".into());
                            }
                        });
                    }
                    return LRESULT(1);
                }
            }
            if is_modifier_key(vk) {
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Position the settings window to the center of the primary screen.
/// Must be called BEFORE show() so the window never appears at (0,0) —
/// which is right where the notch lives — causing the white flash.
fn center_settings_window(win: &SettingsWindow) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let sw = GetSystemMetrics(SM_CXSCREEN) as f32;
        let sh = GetSystemMetrics(SM_CYSCREEN) as f32;
        let scale = win.window().scale_factor();
        // Settings window is 1000 x 700 logical pixels
        let pw = (1000.0 * scale) as i32;
        let ph = (700.0 * scale) as i32;
        let cx = ((sw - pw as f32) / 2.0) as i32;
        // Keep it a bit below center vertically so it's clearly away from the notch
        let cy = ((sh - ph as f32) / 2.0) as i32;
        win.window().set_position(slint::PhysicalPosition::new(cx, cy));
    }
}

fn notch_phase_name(phase: NotchPhase) -> &'static str {
    match phase {
        NotchPhase::Closed => "closed",
        NotchPhase::Opening => "opening",
        NotchPhase::OpenContentStaging => "open-content-staging",
        NotchPhase::Open => "open",
        NotchPhase::ClosingContent => "closing-content",
        NotchPhase::Closing => "closing",
    }
}

fn tab_index(tab: &str) -> i32 {
    match tab {
        "home" => 0,
        "media" => 1,
        "clock" => 2,
        "drop" => 3,
        "capture" => 4,
        "notifications" => 5,
        "preview" => 6,
        "stats" => 7,
        _ => 0,
    }
}

fn configure_motion_targets(ui: &Pill, state: &mut MotionState) {
    let idle_width = ui.get_idle_width().max(10.0);
    let idle_height = ui.get_idle_height().max(10.0);
    let active_tab = ui.get_active_tab().to_string();
    
    let is_currently_hud_size = (state.open.width - 720.0).abs() < 1.0;
    let is_hud_active = !ui.get_system_hud_kind().is_empty();
    
    let (open_width, open_height) = if is_hud_active || (state.is_openish() && is_currently_hud_size) {
        (720.0, 244.0)
    } else {
        match active_tab.as_str() {
            "calendar" | "cal" => (motion::EXPANDED_WIDTH, motion::EXPANDED_HEIGHT),
            _ => (HOME_WIDTH, HOME_HEIGHT),
        }
    };
    
    let closed_radius = ui.get_idle_border_radius().min(idle_height / 2.0).max(0.0);
    let open_radius = ui.get_notch_border_radius().min(open_height / 2.0).max(0.0);
    state.set_closed_geometry(idle_width, idle_height, closed_radius);
    state.set_open_geometry(open_width, open_height, open_radius);
}

fn update_clock_display(ui: &Pill) {
    let now = chrono::Local::now();
    let settings = settings::RavenSettings::load();
    
    // Format Time dynamically
    let mut time_fmt = if settings.clock.mode_24h {
        "%H:%M"
    } else {
        "%I:%M"
    }.to_string();
    
    if settings.clock.show_seconds {
        time_fmt.push_str(":%S");
    }
    
    let mut time_str = now.format(&time_fmt).to_string();
    
    // Remove leading zero in 12-hour format for premium appearance
    if !settings.clock.mode_24h && time_str.starts_with('0') {
        time_str.remove(0);
    }
    
    if !settings.clock.mode_24h && settings.clock.show_ampm {
        time_str.push_str(&format!(" {}", now.format("%p")));
    }
    
    // Format Date dynamically
    let mut date_parts = Vec::new();
    if settings.clock.show_weekday {
        date_parts.push(now.format("%a").to_string()); // e.g. "Sat"
    }
    if settings.clock.show_date {
        date_parts.push(now.format("%b %-d").to_string()); // e.g. "May 24"
    }
    
    let date_str = if date_parts.is_empty() {
        "".to_string()
    } else if date_parts.len() == 2 {
        format!("{}, {}", date_parts[0], date_parts[1]) // e.g. "Sat, May 24"
    } else {
        date_parts[0].clone()
    };
    
    ui.set_time(time_str.clone().into());
    ui.set_date(date_str.into());

    // --- Set Live Clock Properties ---
    use chrono::Timelike;
    let current_hour = now.hour() as i32;
    let current_minute = now.minute() as i32;
    let current_second = now.second() as i32;
    let current_ampm = now.format("%p").to_string();
    let current_second_str = now.format("%S").to_string();
    
    ui.set_current_hour(current_hour);
    ui.set_current_minute(current_minute);
    ui.set_current_second(current_second);
    ui.set_current_ampm(current_ampm.into());
    ui.set_current_time_str(time_str.into());
    ui.set_current_second_str(current_second_str.into());
}

fn update_settings_preview_clock(s_ui: &SettingsWindow, settings: &settings::RavenSettings) {
    let now = chrono::Local::now();
    
    // Format Time dynamically
    let mut time_fmt = if settings.clock.mode_24h {
        "%H:%M"
    } else {
        "%I:%M"
    }.to_string();
    
    if settings.clock.show_seconds {
        time_fmt.push_str(":%S");
    }
    
    let mut time_str = now.format(&time_fmt).to_string();
    
    // Remove leading zero in 12-hour format for premium appearance
    if !settings.clock.mode_24h && time_str.starts_with('0') {
        time_str.remove(0);
    }
    
    if !settings.clock.mode_24h && settings.clock.show_ampm {
        time_str.push_str(&format!(" {}", now.format("%p")));
    }
    
    // Format Date dynamically
    let mut date_parts = Vec::new();
    if settings.clock.show_weekday {
        date_parts.push(now.format("%a").to_string().to_uppercase()); // e.g. "MON"
    }
    if settings.clock.show_date {
        date_parts.push(now.format("%b %-d").to_string().to_uppercase()); // e.g. "JAN 1"
    }
    
    let date_str = if date_parts.is_empty() {
        "".to_string()
    } else {
        date_parts.join(" ")
    };
    
    s_ui.set_current_time(time_str.into());
    s_ui.set_current_date(date_str.into());
}

fn apply_motion_to_ui(ui: &Pill, state: &MotionState) {
    let snap = state.snapshot();
    ui.set_motion_width(snap.width);
    ui.set_motion_height(snap.height);
    ui.set_motion_radius(snap.border_radius);
    ui.set_content_opacity(snap.content_opacity);
    ui.set_panel_ready(snap.panel_ready);
    ui.set_notch_phase(notch_phase_name(snap.phase).into());
    ui.set_is_expanded(snap.is_open);
}

fn begin_notch_motion(ui: &Pill, state: &Rc<RefCell<MotionState>>, open: bool) {
    if !open {
        ui.invoke_close_transient_panels();
        crate::window::set_window_interactive_mode(false);
        crate::window::update_pill_window_layout();
    }
    let mut motion = state.borrow_mut();
    configure_motion_targets(ui, &mut motion);
    if open {
        motion.begin_open();
    } else {
        motion.begin_close();
    }
    apply_motion_to_ui(ui, &motion);
}

#[derive(Clone, Debug)]
struct ClipboardHistoryEntry {
    id: i32,
    text: String,
    title: String,
    copied_at: chrono::DateTime<chrono::Local>,
    pinned: bool,
    selected: bool,
}

#[derive(Clone, Debug, Default)]
struct WifiSnapshot {
    enabled: bool,
    connected: bool,
    ssid: String,
    signal: i32,
    networks: Vec<SlintWifiNetwork>,
}

fn run_hidden_command(program: &str, args: &[&str]) -> Option<String> {
    use std::process::{Command, Stdio};
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd.output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn read_clipboard_text() -> Option<String> {
    let text = run_hidden_command(
        "powershell",
        &["-NoProfile", "-Command", "Get-Clipboard -Raw -Format Text 2>$null"],
    )?;
    let trimmed = text.trim_matches(&['\r', '\n'][..]).to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

fn write_clipboard_text(text: &str) {
    let escaped = text.replace('`', "``").replace('\'', "''");
    let script = format!("Set-Clipboard -Value '{}'", escaped);
    let _ = run_hidden_command("powershell", &["-NoProfile", "-Command", &script]);
}

fn clipboard_title(text: &str) -> String {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Untitled")
        .trim()
        .chars()
        .take(80)
        .collect()
}

fn clipboard_section_label(entry: &ClipboardHistoryEntry) -> String {
    if entry.pinned {
        return "Pinned".to_string();
    }
    let today = chrono::Local::now().date_naive();
    let copied = entry.copied_at.date_naive();
    if copied == today {
        "Today".to_string()
    } else if copied == today - chrono::Duration::days(1) {
        "Yesterday".to_string()
    } else {
        copied.format("%d %b").to_string()
    }
}

fn visible_clipboard_entries(
    history: &[ClipboardHistoryEntry],
    query: &str,
) -> Vec<ClipboardHistoryEntry> {
    let needle = query.trim().to_lowercase();
    let mut entries: Vec<_> = history
        .iter()
        .filter(|entry| {
            needle.is_empty()
                || entry.title.to_lowercase().contains(&needle)
                || entry.text.to_lowercase().contains(&needle)
        })
        .cloned()
        .collect();
    entries.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.copied_at.cmp(&a.copied_at))
    });
    entries
}

fn refresh_clipboard_model(ui: &Pill, history: &[ClipboardHistoryEntry]) {
    let query = ui.get_clipboard_search_query().to_string();
    let entries = visible_clipboard_entries(history, &query);
    let mut rows = Vec::new();
    let mut last_section = String::new();
    let mut row_y = 28_i32;

    for entry in entries {
        let section = clipboard_section_label(&entry);
        let visible_section = if section != last_section {
            row_y += if rows.is_empty() { 0 } else { 18 };
            last_section = section.clone();
            section
        } else {
            String::new()
        };

        rows.push(SlintClipboardItem {
            id: entry.id,
            kind: "text".into(),
            title: entry.title.clone().into(),
            subtitle: format!("{} chars", entry.text.chars().count()).into(),
            preview_text: entry.text.clone().into(),
            image: slint::Image::default(),
            is_image: false,
            pinned: entry.pinned,
            section: visible_section.into(),
            row_y,
            multi_selected: entry.selected,
        });
        row_y += 46;
    }

    let selected = ui.get_selected_clipboard_index();
    if rows.is_empty() {
        ui.set_selected_clipboard_index(0);
    } else if selected < 0 || selected as usize >= rows.len() {
        ui.set_selected_clipboard_index((rows.len() - 1) as i32);
    }
    ui.set_clipboard_items(std::rc::Rc::new(slint::VecModel::from(rows)).into());
}

fn paste_text_to_target(text: String, target_hwnd: isize) {
    write_clipboard_text(&text);
    if target_hwnd == 0 {
        return;
    }

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(80));
        unsafe {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_CONTROL,
            };
            use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

            let hwnd = HWND(target_hwnd as _);
            let _ = SetForegroundWindow(hwnd);
            std::thread::sleep(std::time::Duration::from_millis(40));
            keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
            keybd_event(b'V', 0, KEYBD_EVENT_FLAGS(0), 0);
            keybd_event(b'V', 0, KEYEVENTF_KEYUP, 0);
            keybd_event(VK_CONTROL.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    });
}

fn netsh_output(args: &[&str]) -> String {
    run_hidden_command("netsh", args).unwrap_or_default()
}

fn wifi_line_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let trimmed = line.trim();
    trimmed
        .strip_prefix(key)
        .and_then(|rest| rest.split_once(':').map(|(_, value)| value.trim()))
}

fn wifi_saved_profiles() -> HashSet<String> {
    let output = netsh_output(&["wlan", "show", "profiles"]);
    output
        .lines()
        .filter_map(|line| wifi_line_value(line, "All User Profile"))
        .map(|name| name.to_string())
        .collect()
}

fn wifi_scan_snapshot() -> WifiSnapshot {
    let interfaces = netsh_output(&["wlan", "show", "interfaces"]);
    let enabled = !interfaces.to_lowercase().contains("no wireless interface");
    let mut connected = false;
    let mut current_ssid = String::new();
    let mut current_signal = 0_i32;

    for line in interfaces.lines() {
        if let Some(value) = wifi_line_value(line, "State") {
            connected = value.eq_ignore_ascii_case("connected");
        } else if let Some(value) = wifi_line_value(line, "SSID") {
            if !line.trim_start().starts_with("BSSID") && current_ssid.is_empty() {
                current_ssid = value.to_string();
            }
        } else if let Some(value) = wifi_line_value(line, "Signal") {
            current_signal = value.trim_end_matches('%').trim().parse().unwrap_or(0);
        }
    }

    let saved = wifi_saved_profiles();
    let networks_raw = netsh_output(&["wlan", "show", "networks", "mode=bssid"]);
    let mut networks = Vec::<SlintWifiNetwork>::new();
    let mut ssid = String::new();
    let mut security = String::new();
    let mut signal = 0_i32;

    let mut push_network = |ssid: &mut String, security: &mut String, signal: &mut i32| {
        let name = ssid.trim();
        if name.is_empty() {
            return;
        }
        if networks.iter().any(|n| n.ssid.as_str() == name) {
            ssid.clear();
            security.clear();
            *signal = 0;
            return;
        }
        networks.push(SlintWifiNetwork {
            ssid: name.into(),
            signal: *signal,
            security: if security.is_empty() { "Open".into() } else { security.clone().into() },
            connected: connected && current_ssid == name,
            saved: saved.contains(name),
        });
        ssid.clear();
        security.clear();
        *signal = 0;
    };

    for line in networks_raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("SSID ") && trimmed.contains(':') {
            push_network(&mut ssid, &mut security, &mut signal);
            ssid = trimmed.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default();
        } else if let Some(value) = wifi_line_value(line, "Authentication") {
            security = value.to_string();
        } else if let Some(value) = wifi_line_value(line, "Signal") {
            signal = value.trim_end_matches('%').trim().parse().unwrap_or(signal);
        }
    }
    push_network(&mut ssid, &mut security, &mut signal);

    networks.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| b.signal.cmp(&a.signal)));

    WifiSnapshot {
        enabled,
        connected,
        ssid: current_ssid,
        signal: current_signal,
        networks,
    }
}

fn apply_wifi_snapshot(ui: &Pill, snapshot: WifiSnapshot) {
    ui.set_wifi_enabled(snapshot.enabled);
    ui.set_wifi_connected(snapshot.connected);
    ui.set_wifi_ssid(snapshot.ssid.clone().into());
    ui.set_wifi_signal(snapshot.signal);
    ui.set_wifi_status_text(if snapshot.connected {
        snapshot.ssid.into()
    } else if snapshot.enabled {
        "Not connected".into()
    } else {
        "Wi-Fi unavailable".into()
    });
    ui.set_wifi_networks(std::rc::Rc::new(slint::VecModel::from(snapshot.networks)).into());
}

fn wifi_disconnect_current() {
    let _ = netsh_output(&["wlan", "disconnect"]);
}

fn wifi_connect_network(ssid: &str, password: &str) {
    if !password.trim().is_empty() {
        let profile_name = ssid.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let key = password.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let profile = format!(
            r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
  <name>{0}</name>
  <SSIDConfig><SSID><name>{0}</name></SSID></SSIDConfig>
  <connectionType>ESS</connectionType>
  <connectionMode>auto</connectionMode>
  <MSM><security><authEncryption><authentication>WPA2PSK</authentication><encryption>AES</encryption><useOneX>false</useOneX></authEncryption><sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{1}</keyMaterial></sharedKey></security></MSM>
</WLANProfile>"#,
            profile_name, key
        );
        let path = std::env::temp_dir().join("raven_wifi_profile.xml");
        if std::fs::write(&path, profile).is_ok() {
            if let Some(path_str) = path.to_str() {
                let _ = netsh_output(&["wlan", "add", "profile", &format!("filename={}", path_str), "user=current"]);
            }
            let _ = std::fs::remove_file(path);
        }
    }
    let _ = netsh_output(&["wlan", "connect", &format!("name={}", ssid), &format!("ssid={}", ssid)]);
}

fn render_topbar_stats_ring(cpu: f32, ram: f32, gpu: f32) -> slint::Image {
    // Render 4x larger than the displayed 92px slot so the ring stays sharp
    // after Slint scales it down inside the top-bar stats panel.
    let size = 368_u32;
    let scale = size as f32 / 92.0;
    let center = size as f32 / 2.0;
    let rings = [
        (40.0_f32 * scale, 3.6_f32 * scale, cpu.clamp(0.0, 100.0) / 100.0, -0.25_f32, (246, 185, 76, 255)),
        (31.0_f32 * scale, 3.4_f32 * scale, ram.clamp(0.0, 100.0) / 100.0, 0.06_f32, (246, 185, 76, 225)),
        (22.0_f32 * scale, 3.0_f32 * scale, gpu.clamp(0.0, 100.0) / 100.0, 0.36_f32, (246, 185, 76, 190)),
    ];
    let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(size, size);

    for (index, pixel) in buffer.make_mut_slice().iter_mut().enumerate() {
        let x = (index as u32 % size) as f32 + 0.5;
        let y = (index as u32 / size) as f32 + 0.5;
        let dx = x - center;
        let dy = y - center;
        let distance = (dx * dx + dy * dy).sqrt();
        let mut out = slint::Rgba8Pixel::new(0, 0, 0, 0);

        for (radius, half, progress, start_turn, color) in rings {
            let on_ring = (distance - radius).abs() <= half;
            if !on_ring {
                continue;
            }
            let start_angle = start_turn * std::f32::consts::TAU;
            let mut angle = dy.atan2(dx) - start_angle;
            if angle < 0.0 {
                angle += std::f32::consts::TAU;
            }
            let arc_progress = angle / std::f32::consts::TAU;
            let active = progress > 0.0 && arc_progress <= progress;
            let end_angle = start_angle + progress * std::f32::consts::TAU;
            let start_point = (
                center + radius * start_angle.cos(),
                center + radius * start_angle.sin(),
            );
            let end_point = (
                center + radius * end_angle.cos(),
                center + radius * end_angle.sin(),
            );
            let start_cap = progress > 0.0
                && ((x - start_point.0).powi(2) + (y - start_point.1).powi(2)).sqrt() <= half;
            let end_cap = progress > 0.0
                && ((x - end_point.0).powi(2) + (y - end_point.1).powi(2)).sqrt() <= half;

            out = if active || start_cap || end_cap {
                slint::Rgba8Pixel::new(color.0, color.1, color.2, color.3)
            } else {
                slint::Rgba8Pixel::new(215, 207, 192, 36)
            };
        }
        *pixel = out;
    }

    slint::Image::from_rgba8(buffer)
}

fn slint_component_hwnd<T: slint::ComponentHandle>(component: &T) -> Option<isize> {
    use raw_window_handle::HasWindowHandle;
    let window = component.window();
    let binding = window.window_handle();
    let handle = binding.window_handle().ok()?;
    if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
        Some(win32.hwnd.get() as isize)
    } else {
        None
    }
}

fn overlay_scale() -> f32 {
    let scale_bits = crate::window::PILL_SCALE_FACTOR.load(std::sync::atomic::Ordering::SeqCst);
    let scale = if scale_bits == 0 { 1.0 } else { f32::from_bits(scale_bits) };
    if scale.is_finite() && scale > 0.0 { scale } else { 1.0 }
}

fn scaled_px(value: f32) -> i32 {
    (value * overlay_scale()).round() as i32
}

fn configure_floating_overlay_hwnd(
    hwnd_val: isize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    click_through: bool,
) {
    if hwnd_val == 0 {
        return;
    }
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE,
            GWLP_HWNDPARENT, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, WS_CAPTION,
            WS_DLGFRAME, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP,
            WS_SYSMENU, WS_THICKFRAME,
        };

        let hwnd = HWND(hwnd_val as _);
        let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0);
        crate::window::remove_from_taskbar(hwnd);

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let mut target_ex_style = (ex_style & !WS_EX_APPWINDOW.0)
            | WS_EX_LAYERED.0
            | WS_EX_TOOLWINDOW.0
            | WS_EX_TOPMOST.0
            | WS_EX_NOACTIVATE.0;
        if click_through {
            target_ex_style |= WS_EX_TRANSPARENT.0;
        } else {
            target_ex_style &= !WS_EX_TRANSPARENT.0;
        }
        if target_ex_style != ex_style {
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex_style as isize);
        }

        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let target_style = (style | WS_POPUP.0)
            & !(WS_CAPTION.0
                | WS_THICKFRAME.0
                | WS_MINIMIZEBOX.0
                | WS_MAXIMIZEBOX.0
                | WS_SYSMENU.0
                | WS_DLGFRAME.0);
        if target_style != style {
            let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, target_style as isize);
        }

        let _ = SetWindowPos(
            hwnd,
            HWND(-1),
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct DragScaleState {
    is_dragging: bool,
    is_left_edge: bool,
    start_mouse_x: i32,
    start_scale: f32,
    start_rect: windows::Win32::Foundation::RECT,
}

#[derive(Clone, Copy, Debug)]
struct DragMoveState {
    is_dragging: bool,
    start_mouse_x: i32,
    start_mouse_y: i32,
    start_rect: windows::Win32::Foundation::RECT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusBarConfig {
    Uninitialized,
    Normal,
    Completed,
}

thread_local! {
    static FOCUS_BAR_WEAK: std::cell::RefCell<Option<slint::Weak<FocusStatusBarWindow>>> = std::cell::RefCell::new(None);
    static FOCUS_BAR_DRAGGING: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

fn focusbar_log(msg: &str) {
    use std::io::Write;
    let path = std::path::Path::new("focusbar_debug_trace.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{}] {}", chrono::Local::now().format("%H:%M:%S%.3f"), msg);
    }
    println!("[FOCUSBAR-LOG] {}", msg);
}

pub unsafe extern "system" fn focus_bar_subclass_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id_subclass: usize,
    _ref_data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_RESTORE};
    use windows::Win32::Foundation::LRESULT;

    match msg {
        0x0084 => {
            return LRESULT(1); // HTCLIENT
        }
        0x0005 => {
            if wparam.0 == 1 { // SIZE_MINIMIZED
                let _ = ShowWindow(hwnd, SW_RESTORE);
                return LRESULT(0);
            }
        }
        0x007D => {
            let ex_style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE) as u32;
            let target_ex_style = (ex_style & !windows::Win32::UI::WindowsAndMessaging::WS_EX_APPWINDOW.0 & !windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT.0)
                | windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED.0
                | windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW.0
                | windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST.0
                | windows::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE.0;
            
            if ex_style != target_ex_style {
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE, target_ex_style as isize);
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                    hwnd,
                    windows::Win32::Foundation::HWND(-1), // HWND_TOPMOST
                    0, 0, 0, 0,
                    windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE |
                    windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE |
                    windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE |
                    windows::Win32::UI::WindowsAndMessaging::SWP_FRAMECHANGED
                );
            }
        }
        // WM_MOUSEMOVE = 0x0200: mouse entered/is over window → set hovered
        0x0200 => {
            FOCUS_BAR_WEAK.with(|cell| {
                if let Some(weak) = cell.borrow().as_ref() {
                    if let Some(w) = weak.upgrade() {
                        if !w.get_is_hovered_rust() {
                            w.set_is_hovered_rust(true);
                            // Request TrackMouseEvent so we get WM_MOUSELEAVE
                            unsafe {
                                let mut tme = windows::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT {
                                    cbSize: std::mem::size_of::<windows::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT>() as u32,
                                    dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::TME_LEAVE,
                                    hwndTrack: hwnd,
                                    dwHoverTime: 0,
                                };
                                let _ = windows::Win32::UI::Input::KeyboardAndMouse::TrackMouseEvent(&mut tme);
                            }
                        }
                    }
                }
            });
        }
        // WM_MOUSELEAVE = 0x02A3: mouse left window → clear hovered
        0x02A3 => {
            FOCUS_BAR_WEAK.with(|cell| {
                if let Some(weak) = cell.borrow().as_ref() {
                    if let Some(w) = weak.upgrade() {
                        w.set_is_hovered_rust(false);
                    }
                }
            });
        }
        _ => {}
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

fn configure_focus_bar_hwnd(hwnd_val: isize, scale: f32) {
    if hwnd_val == 0 {
        focusbar_log("configure_focus_bar_hwnd called with hwnd_val=0, returning early");
        return;
    }
    focusbar_log(&format!("configure_focus_bar_hwnd START hwnd_val=0x{:X} scale={}", hwnd_val, scale));
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWLP_HWNDPARENT,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_FRAMECHANGED, SWP_SHOWWINDOW,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_EX_APPWINDOW,
        };
        let hwnd = HWND(hwnd_val as _);

        let old = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0);
        focusbar_log(&format!("Step1: Owner set to 0 (no owner), old_owner=0x{:X}", old));

        crate::window::remove_from_taskbar(hwnd);
        focusbar_log("Step2: remove_from_taskbar called synchronously");

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let target_ex_style = (ex_style & !WS_EX_TRANSPARENT.0 & !WS_EX_APPWINDOW.0)
            | WS_EX_TOOLWINDOW.0
            | WS_EX_TOPMOST.0
            | WS_EX_NOACTIVATE.0;
        focusbar_log(&format!("Step3: ex_style=0x{:08X} target=0x{:08X}", ex_style, target_ex_style));
            
        if target_ex_style != ex_style {
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex_style as isize);
            let verify = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            focusbar_log(&format!("Step3: ExStyle applied. Verify=0x{:08X}", verify));
        }

        let style = GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE) as u32;
        let target_style = (style | windows::Win32::UI::WindowsAndMessaging::WS_POPUP.0)
            & !(windows::Win32::UI::WindowsAndMessaging::WS_CAPTION.0
                | windows::Win32::UI::WindowsAndMessaging::WS_THICKFRAME.0
                | windows::Win32::UI::WindowsAndMessaging::WS_MINIMIZEBOX.0
                | windows::Win32::UI::WindowsAndMessaging::WS_MAXIMIZEBOX.0
                | windows::Win32::UI::WindowsAndMessaging::WS_SYSMENU.0
                | windows::Win32::UI::WindowsAndMessaging::WS_DLGFRAME.0);
        if style != target_style {
            let _ = SetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE, target_style as isize);
        }
        
        let _ = SetWindowPos(
            hwnd,
            HWND(0),
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );

        let _ = windows::Win32::UI::Shell::SetWindowSubclass(
            hwnd,
            Some(focus_bar_subclass_proc),
            9696,
            0,
        );
        
        use windows::Win32::Graphics::Gdi::*;
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let screen_width = info.rcMonitor.right - info.rcMonitor.left;
        
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
        let dpi_scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
        
        let safe_scale = scale.clamp(1.0, 1.8);
        let phys_w = (420.0 * safe_scale * dpi_scale) as i32;
        let phys_h = (84.0 * safe_scale * dpi_scale) as i32;
        
        let x = info.rcMonitor.left + (screen_width - phys_w) / 2;
        let y = info.rcMonitor.top + (36.0 * safe_scale * dpi_scale) as i32 + 8;
        
        let _ = SetWindowPos(
            hwnd,
            HWND(-1), // HWND_TOPMOST
            x,
            y,
            phys_w,
            phys_h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );

        // Ensure window is fully opaque — WS_EX_LAYERED without this call = invisible window
        let _ = windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
            hwnd,
            windows::Win32::Foundation::COLORREF(0),
            255,
            windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA,
        );

        let hwnd_raw = hwnd.0;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = slint::invoke_from_event_loop(move || {
                unsafe {
                    crate::window::remove_from_taskbar(HWND(hwnd_raw));
                }
            });
        });
    }
}

fn hide_overlay_hwnd(hwnd_val: isize) {
    if hwnd_val == 0 {
        return;
    }
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        let _ = ShowWindow(HWND(hwnd_val as _), SW_HIDE);
    }
}

fn parse_hex_color(hex: &str) -> slint::Color {
    let hex = hex.trim_start_matches('#');
    if let Ok(val) = u32::from_str_radix(hex, 16) {
        let r = ((val >> 16) & 0xFF) as u8;
        let g = ((val >> 8) & 0xFF) as u8;
        let b = (val & 0xFF) as u8;
        slint::Color::from_rgb_u8(r, g, b)
    } else {
        slint::Color::from_rgb_u8(255, 255, 255)
    }
}

fn sync_selected_clock_settings_to_ui(s_ui: &SettingsWindow, settings: &settings::RavenSettings, idx: usize) {
    let inst = settings.widgets.get_clock_instance(idx);
    s_ui.set_widgets_stats_show_cpu(inst.show_cpu);
    s_ui.set_widgets_stats_show_ram(inst.show_ram);
    s_ui.set_widgets_stats_show_battery(inst.show_battery);
    s_ui.set_widgets_stats_show_percentage(inst.show_percentage);
    s_ui.set_widgets_stats_cpu_color(parse_hex_color(&inst.cpu_color));
    s_ui.set_widgets_stats_ram_color(parse_hex_color(&inst.ram_color));
    s_ui.set_widgets_stats_battery_color(parse_hex_color(&inst.battery_color));
    s_ui.set_widgets_stats_border_radius(inst.border_radius as i32);
    s_ui.set_widgets_opacity(inst.opacity as f32);
    s_ui.set_widget_size_select(inst.size.clone().into());
}

fn sync_all_clocks_to_ui(s_ui: &SettingsWindow, settings: &settings::RavenSettings) {
    // Clock 0
    let inst0 = settings.widgets.get_clock_instance(0);
    s_ui.set_clock_0_border_radius(inst0.border_radius as i32);
    s_ui.set_clock_0_cpu_color(parse_hex_color(&inst0.cpu_color));
    s_ui.set_clock_0_show_battery(inst0.show_battery);
    s_ui.set_clock_0_size(inst0.size.clone().into());

    // Clock 1
    let inst1 = settings.widgets.get_clock_instance(1);
    s_ui.set_clock_1_border_radius(inst1.border_radius as i32);
    s_ui.set_clock_1_cpu_color(parse_hex_color(&inst1.cpu_color));
    s_ui.set_clock_1_show_battery(inst1.show_battery);
    s_ui.set_clock_1_size(inst1.size.clone().into());

    // Clock 2
    let inst2 = settings.widgets.get_clock_instance(2);
    s_ui.set_clock_2_border_radius(inst2.border_radius as i32);
    s_ui.set_clock_2_cpu_color(parse_hex_color(&inst2.cpu_color));
    s_ui.set_clock_2_show_battery(inst2.show_battery);
    s_ui.set_clock_2_size(inst2.size.clone().into());

    // Clock 3
    let inst3 = settings.widgets.get_clock_instance(3);
    s_ui.set_clock_3_border_radius(inst3.border_radius as i32);
    s_ui.set_clock_3_cpu_color(parse_hex_color(&inst3.cpu_color));
    s_ui.set_clock_3_show_battery(inst3.show_battery);
    s_ui.set_clock_3_size(inst3.size.clone().into());

    // Clock 4
    let inst4 = settings.widgets.get_clock_instance(4);
    s_ui.set_clock_4_border_radius(inst4.border_radius as i32);
    s_ui.set_clock_4_cpu_color(parse_hex_color(&inst4.cpu_color));
    s_ui.set_clock_4_show_battery(inst4.show_battery);
    s_ui.set_clock_4_size(inst4.size.clone().into());
}

fn builtin_widget_enabled(settings: &settings::RavenSettings, widget_id: &str) -> bool {
    match widget_id {
        "year_progress" => settings.widgets.year_journey_enabled,
        "day_progress" => settings.widgets.day_journey_enabled,
        "month_progress" => settings.widgets.month_journey_enabled,
        "media" => settings.widgets.media_enabled,
        "notes" => settings.widgets.notes_enabled,
        "todo" => settings.widgets.todo_enabled,
        "quotes" => settings.widgets.quotes_enabled,
        "picture" => settings.widgets.picture_enabled,
        "video" => settings.widgets.video_enabled,
        "battery" => settings.widgets.battery_widget_enabled,
        "calendar_focus" => settings.widgets.calendar_focus_enabled,
        "apps_container" => settings.widgets.apps_container_enabled,
        "focus_score" => settings.widgets.focus_score_widget_enabled,
        "streak" => settings.widgets.streak_widget_enabled,
        _ => false,
    }
}

fn set_or_copy_builtin_widget(path: &[&str], widget_id: &str, value: bool) {
    let was_enabled = builtin_widget_enabled(&settings::RavenSettings::load(), widget_id);
    if value && was_enabled {
        settings::add_widget_instance_copy(widget_id);
    } else {
        settings::set_bool(path, value);
    }
}

fn active_widget_title(kind: &str) -> &'static str {
    match kind {
        "year_progress" => "Year Progress",
        "day_progress" => "Day Progress",
        "month_progress" => "Month Progress",
        "media" => "Media Player",
        "clock_0" | "clock_1" | "clock_2" | "clock_3" | "clock_4" => "Clock Widget",
        "notes" => "Quick Notes",
        "todo" => "To-Do List",
        "quotes" => "Daily Quotes",
        "picture" => "Picture Frame",
        "video" => "Video Frame",
        "battery" => "Battery Status",
        "calendar_focus" => "Calendar Focus",
        "apps_container" => "Apps Container",
        "streak" => "Calendar Widget",
        "focus_score" => "Focus Score",
        _ => "Widget",
    }
}

fn reconcile_widget_order(s_ui: &SettingsWindow) {
    let settings = settings::RavenSettings::load();
    let mut current_order = settings.widgets.widget_order.clone();

    // 1. Determine which widget IDs should be active
    let mut active_ids = std::collections::HashSet::new();
    if settings.widgets.year_journey_enabled { active_ids.insert("year_progress".to_string()); }
    if settings.widgets.day_journey_enabled { active_ids.insert("day_progress".to_string()); }
    if settings.widgets.month_journey_enabled { active_ids.insert("month_progress".to_string()); }
    if settings.widgets.media_enabled { active_ids.insert("media".to_string()); }
    if settings.widgets.notes_enabled { active_ids.insert("notes".to_string()); }
    if settings.widgets.todo_enabled { active_ids.insert("todo".to_string()); }
    if settings.widgets.quotes_enabled { active_ids.insert("quotes".to_string()); }
    if settings.widgets.picture_enabled { active_ids.insert("picture".to_string()); }
    if settings.widgets.video_enabled { active_ids.insert("video".to_string()); }
    if settings.widgets.battery_widget_enabled { active_ids.insert("battery".to_string()); }
    if settings.widgets.calendar_focus_enabled { active_ids.insert("calendar_focus".to_string()); }
    if settings.widgets.apps_container_enabled { active_ids.insert("apps_container".to_string()); }
    if settings.widgets.focus_score_widget_enabled { active_ids.insert("focus_score".to_string()); }
    if settings.widgets.streak_widget_enabled { active_ids.insert("streak".to_string()); }
    for instance in settings.widgets.instances.iter().filter(|instance| instance.visible && !instance.id.is_empty()) {
        active_ids.insert(instance.id.clone());
    }

    // Clocks
    if settings.widgets.clock_enabled && settings.widgets.stats_enabled {
        let count = (settings.widgets.clock_count as usize).min(5);
        for i in 0..count {
            active_ids.insert(format!("clock_{}", i));
        }
    }

    // 2. Filter existing order to keep only currently active ones
    current_order.retain(|id| active_ids.contains(id));

    // 3. For any active ID not in the order, append it sequentially
    let mut all_ids = vec![
        "year_progress".to_string(), "day_progress".to_string(), "month_progress".to_string(),
        "media".to_string(), "notes".to_string(), "todo".to_string(), "quotes".to_string(),
        "picture".to_string(), "video".to_string(), "battery".to_string(), "calendar_focus".to_string(),
        "apps_container".to_string(), "focus_score".to_string(), "streak".to_string()
    ];
    for i in 0..5 {
        all_ids.push(format!("clock_{}", i));
    }
    for instance in settings.widgets.instances.iter().filter(|instance| instance.visible && !instance.id.is_empty()) {
        all_ids.push(instance.id.clone());
    }

    for id in &all_ids {
        if active_ids.contains(id) && !current_order.contains(id) {
            current_order.push(id.clone());
        }
    }

    // 4. Save to settings if changed
    if current_order != settings.widgets.widget_order {
        settings::set_widget_order(current_order.clone());
    }

    // 5. Update Slint UI
    let slint_order: Vec<slint::SharedString> = current_order
        .iter()
        .map(|s| slint::SharedString::from(s.as_str()))
        .collect();
    s_ui.set_widgets_active_order(std::rc::Rc::new(slint::VecModel::from(slint_order)).into());

    let rows: Vec<ActiveWidgetRow> = current_order
        .iter()
        .map(|id| {
            let kind = settings
                .widgets
                .instances
                .iter()
                .find(|instance| instance.id == id.as_str())
                .map(|instance| instance.widget_type.clone())
                .unwrap_or_else(|| id.clone());
            let title = settings
                .widgets
                .instances
                .iter()
                .find(|instance| instance.id == id.as_str() && !instance.title.trim().is_empty())
                .map(|instance| instance.title.clone())
                .unwrap_or_else(|| active_widget_title(&kind).to_string());
            ActiveWidgetRow {
                id: id.as_str().into(),
                kind: kind.into(),
                title: title.into(),
            }
        })
        .collect();
    s_ui.set_widgets_active_rows(std::rc::Rc::new(slint::VecModel::from(rows)).into());
}

fn main() {
    diagnostics::log_startup();
    let startup_account_token = account_token_from_args();

    // Check if another instance of the app is already running
    let existing_hwnd = unsafe {
        let class_name = wide("RavenNativeHidden");
        let title = wide("Raven Native Hidden");
        windows::Win32::UI::WindowsAndMessaging::FindWindowW(
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
        )
    };

    if existing_hwnd.0 != 0 {
        if let Some(token) = startup_account_token {
            println!("[SETUP] Forwarding sign-in token to already running instance...");
            unsafe {
                let bytes = token.as_bytes();
                let cds = windows::Win32::System::DataExchange::COPYDATASTRUCT {
                    dwData: 0x1337,
                    cbData: bytes.len() as u32,
                    lpData: bytes.as_ptr() as *mut _,
                };
                let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                    existing_hwnd,
                    windows::Win32::UI::WindowsAndMessaging::WM_COPYDATA,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(&cds as *const _ as isize),
                );
            }
        } else {
            // Forward ShowSettings command to the existing instance
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    existing_hwnd,
                    0x8000 + 1337,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
        }
        std::process::exit(0);
    }

    // Set process-wide per-monitor DPI awareness context first
    unsafe {
        let set_dpi_result = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
        let context = windows::Win32::UI::HiDpi::GetThreadDpiAwarenessContext();
        let awareness = windows::Win32::UI::HiDpi::GetAwarenessFromDpiAwarenessContext(context);
        diagnostics::log(
            "DPI",
            &format!("runtime_set={:?} awareness={:?}", set_dpi_result, awareness),
        );
        let _ = windows::Win32::System::Ole::OleInitialize(None);
    }

    // Load settings — widgets will be shown/hidden based on what the user saved.
    // Do NOT force-enable widgets here; respect the saved (or default) state.
    let settings = settings::RavenSettings::load();
    if settings.advanced.run_on_startup {
        if let Err(e) = crate::window::set_run_on_startup(true) {
            eprintln!("[STARTUP-ERROR] Failed to sync startup path on boot: {:?}", e);
        }
    }
    HOVER_ENABLED.store(settings.hover.enabled, Ordering::SeqCst);
    HOVER_OPEN_DELAY_MS.store(settings.hover.open_delay, Ordering::SeqCst);
    HOVER_CLOSE_DELAY_MS.store(settings.hover.close_delay, Ordering::SeqCst);
    APPEARANCE_AUTO_HIDE.store(settings.appearance.auto_hide, Ordering::SeqCst);
    APPEARANCE_AUTO_HIDE_ON_FULLSCREEN.store(settings.appearance.auto_hide_on_fullscreen, Ordering::SeqCst);

    let events = events::EventBus::new();
    let services = services::ServiceRegistry::new(settings.clone(), events.clone());

    // Instantiate Notch Pill
    let ui = Pill::new().unwrap();
    sync_focus_data_to_ui(&ui, &settings);

    window::PILL_UI_WEAK.set(ui.as_weak()).ok();

    window::set_active_tab("home".to_string());
    window::set_tab_visibility("home".to_string(), settings.tabs.home);
    window::set_tab_visibility("media".to_string(), settings.tabs.media);
    window::set_tab_visibility("clock".to_string(), settings.tabs.clock);
    window::set_tab_visibility("drop".to_string(), settings.tabs.drop);
    window::set_tab_visibility("stats".to_string(), settings.tabs.stats);

    window::TAB_SWITCH_CALLBACK.set(Box::new({
        let ui_weak = ui.as_weak();
        move |forward| {
            let ui_weak_cloned = ui_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak_cloned.upgrade() {
                    let current = ui.get_active_tab().to_string();
                    
                    let mut active_tabs = Vec::new();
                    if ui.get_tab_home() { active_tabs.push("home"); }
                    if ui.get_tab_media() { active_tabs.push("media"); }
                    if ui.get_tab_clock() { active_tabs.push("clock"); }
                    if ui.get_tab_drop() { active_tabs.push("drop"); }
                    if ui.get_tab_stats() { active_tabs.push("stats"); }
                    
                    if active_tabs.is_empty() { return; }
                    
                    let current_idx = active_tabs.iter().position(|&t| t == current).unwrap_or(0);
                    let next_idx = if forward {
                        (current_idx + 1) % active_tabs.len()
                    } else {
                        (current_idx + active_tabs.len() - 1) % active_tabs.len()
                    };
                    
                    let next_tab = active_tabs[next_idx];
                    ui.invoke_switch_tab(next_tab.into());
                }
            });
        }
    })).ok();

    let ui_weak = ui.as_weak();
    let shelf_cloned = services.shelf.clone();
    window::DROP_CALLBACK.set(Box::new(move |paths, cx_logical| {
        let ui_weak = ui_weak.clone();
        let shelf = shelf_cloned.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let settings = settings::RavenSettings::load();
            if !settings.drop.enabled {
                return;
            }
            if let Some(ui) = ui_weak.upgrade() {
                if settings.drop.auto_expand {
                    ui.invoke_switch_tab("drop".into());
                    ui.invoke_request_notch_open();
                }

                if cx_logical <= 176.0 {
                    if let Some(first_path) = paths.first() {
                        let provider_id = ui.get_share_provider_id().to_string();
                        println!("[DROP] Share file: {} with {}", first_path, provider_id);
                        ui.invoke_shelf_share_file(first_path.clone().into(), provider_id.into());
                    }
                } else {
                    println!("[DROP] Add files to shelf: {:?}", paths);
                    shelf.add_paths(paths);

                    // Refresh shelf items in the UI so they appear immediately
                    let shelf_items_raw = shelf.items();
                    let shelf_items: Vec<crate::SlintShelfItem> = shelf_items_raw.into_iter().map(|item| {
                        let thumbnail = if item.is_image {
                            slint::Image::load_from_path(std::path::Path::new(&item.path)).unwrap_or_default()
                        } else {
                            slint::Image::default()
                        };
                        
                        let size_str = if item.size >= 1_048_576 {
                            format!("{:.1} MB", item.size as f64 / 1_048_576.0)
                        } else if item.size >= 1024 {
                            format!("{:.1} KB", item.size as f64 / 1024.0)
                        } else {
                            format!("{} B", item.size)
                        };

                        crate::SlintShelfItem {
                            id: item.id.into(),
                            name: item.name.into(),
                            path: item.path.into(),
                            size_str: size_str.into(),
                            is_image: item.is_image,
                            is_video: item.is_video,
                            thumbnail,
                        }
                    }).collect();
                    ui.set_shelf_items(std::rc::Rc::new(slint::VecModel::from(shelf_items)).into());
                }
            }
        });
    })).ok();

    ui.set_idle_width(settings.appearance.idle_width);
    {
        let (logical_screen_w, _) = crate::window::get_primary_screen_logical_width();
        ui.set_screen_width(logical_screen_w);
    }
    ui.set_full_width_bar(settings.advanced.full_width_bar);
    ui.set_top_bar_widgets(settings.advanced.full_width_bar && settings.advanced.top_bar_widgets);
    ui.set_top_bar_widget_raven(settings.advanced.top_bar_widget_raven);
    ui.set_top_bar_widget_media(settings.advanced.top_bar_widget_media);
    ui.set_top_bar_widget_apps(settings.advanced.top_bar_widget_apps);
    ui.set_top_bar_widget_stats(settings.advanced.top_bar_widget_stats);
    ui.set_top_bar_widget_clipboard(settings.advanced.top_bar_widget_clipboard);
    ui.set_top_bar_widget_volume(settings.advanced.top_bar_widget_volume);
    ui.set_top_bar_widget_wifi(settings.advanced.top_bar_widget_wifi);
    ui.set_top_bar_widget_battery(settings.advanced.top_bar_widget_battery);
    ui.set_top_bar_widget_timer(settings.advanced.top_bar_widget_timer);
    ui.set_top_bar_widget_calendar(settings.advanced.top_bar_widget_calendar);
    ui.set_alert_enabled(settings.raven_alert.enabled);
    ui.set_alert_monitor_charger_in(settings.raven_alert.monitor_charger_in);
    ui.set_alert_monitor_charger_out(settings.raven_alert.monitor_charger_out);
    ui.set_alert_monitor_low_battery(settings.raven_alert.monitor_low_battery);
    ui.set_alert_monitor_unlock(settings.raven_alert.monitor_unlock);
    ui.set_alert_monitor_bluetooth(settings.raven_alert.monitor_bluetooth);
    ui.set_alert_monitor_keys(settings.raven_alert.monitor_keys);
    ui.set_alert_volume_hud(settings.raven_alert.monitor_volume_hud);
    ui.set_alert_brightness_hud(settings.raven_alert.monitor_brightness_hud);
    ui.set_alert_monitor_camera(settings.raven_alert.monitor_camera);
    ui.set_alert_monitor_caffeine(settings.raven_alert.monitor_caffeine);
    ui.set_camera_active(false);

    ui.set_always_on_charging_mode(settings.intelligence.always_on_charging_mode.clone().into());
    ui.set_always_on_charging(settings.intelligence.always_on_charging);
    ui.set_always_on_low_battery(settings.intelligence.always_on_low_battery);

    ui.set_auto_hide(settings.appearance.auto_hide);
    ui.set_idle_height(settings.appearance.idle_height);
    ui.set_notch_border_radius(settings.appearance.border_radius);
    ui.set_idle_border_radius(settings.appearance.idle_border_radius);
    ui.set_appearance_shape(settings.appearance.shape.clone().into());
    ui.set_idle_pill_mode(settings.appearance.idle_pill_mode.clone().into());
    ui.set_idle_custom_name(settings.appearance.idle_custom_name.clone().into());
    ui.set_appearance_mode(settings.appearance.appearance_mode.clone().into());
    ui.set_appearance_opacity(settings.appearance.notch_opacity / 100.0);
    ui.set_bezel_opacity(settings.advanced.bezel_opacity / 100.0);
    ui.set_show_pill_waveform(settings.media.pill_waveform);
    ui.set_media_show_waveform(settings.media.show_waveform);
    ui.set_media_show_source(settings.media.show_source);
    ui.set_media_auto_expand(settings.media.auto_expand);
    ui.set_media_adaptive_accent(settings.media.adaptive_accent);
    ui.set_media_full_calendar_on_no_media(settings.media.full_calendar_on_no_media);
    ui.set_hover_enabled(settings.hover.enabled);
    ui.set_tab_home(settings.tabs.home);
    ui.set_tab_media(settings.tabs.media);
    ui.set_tab_calendar(settings.tabs.calendar);
    ui.set_tab_clock(settings.tabs.clock);
    ui.set_tab_drop(settings.tabs.drop);
    let provider_id = settings.drop.default_provider.clone();
    let provider_name = match provider_id.as_str() {
        "quickshare" => "Quick Share",
        "kdeconnect" => "KDE Connect",
        _ => "LocalSend",
    };
    ui.set_share_provider_id(provider_id.into());
    ui.set_share_provider_name(provider_name.into());

    ui.set_tab_stats(settings.tabs.stats);
    ui.set_tab_caffeine(settings.tabs.caffeine);
    ui.set_tab_settings(settings.tabs.settings);
    ui.set_tab_battery(settings.tabs.battery);
    ui.set_motion_width(settings.appearance.idle_width);
    ui.set_motion_height(settings.appearance.idle_height);
    ui.set_motion_radius(settings.appearance.idle_border_radius);
    ui.set_content_opacity(0.0);
    ui.set_panel_ready(false);
    ui.set_notch_phase("closed".into());

    let ui_motion_state = Rc::new(RefCell::new(MotionState::closed(
        settings.appearance.idle_width,
        settings.appearance.idle_height,
        settings.appearance.idle_border_radius,
    )));

    let motion_tick_ui = ui.as_weak();
    let motion_tick_state = ui_motion_state.clone();
    let motion_tick_timer = slint::Timer::default();
    motion_tick_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(16),
        move || {
            if let Some(ui) = motion_tick_ui.upgrade() {
                let mut motion = motion_tick_state.borrow_mut();
                if motion.is_animating() {
                    configure_motion_targets(&ui, &mut motion);
                    motion.advance_frame();
                    apply_motion_to_ui(&ui, &motion);
                }
            }
        },
    );
    std::mem::forget(motion_tick_timer);

    struct DropTracker {
        id: usize,
        name: String,
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            println!("[LIFECYCLE-LOG] destruction: {} ID = {} has been dropped (destruction/Slint component drop)", self.name, self.id);
        }
    }

    static NEXT_LIFECYCLE_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    
    // Instantiate Settings Window
    let settings_ui = SettingsWindow::new().unwrap();

    {
        let ui_weak = ui.as_weak();
        let settings_ui_weak = settings_ui.as_weak();
        std::thread::spawn(move || {
            let status = license::get_license_status().unwrap_or_else(|error| license::LicenseStatus {
                status: "unknown".to_string(),
                device_id: String::new(),
                license_key: None,
                account_email: None,
                account_name: None,
                account_username: None,
                account_picture: None,
                trial_expires_at: None,
                message: Some(error),
                force_trial_expired_preview: false,
            });
            let _ = slint::invoke_from_event_loop(move || {
                if let (Some(ui), Some(settings_ui)) = (ui_weak.upgrade(), settings_ui_weak.upgrade()) {
                    apply_license_status(&ui, &settings_ui, &status);
                }
            });
        });
    }

    if let Some(token) = startup_account_token {
        let ui_weak = ui.as_weak();
        let settings_ui_weak = settings_ui.as_weak();
        std::thread::spawn(move || {
            let result = license::connect_account_token(token);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(settings_ui) = settings_ui_weak.upgrade() {
                    match result {
                        Ok(status) => {
                            if let Some(ui) = ui_weak.upgrade() {
                                apply_license_status(&ui, &settings_ui, &status);
                            }
                        }
                        Err(error) => {
                            settings_ui.set_license_action_message(
                                format!("Account sign-in failed: {error}").into(),
                            );
                        }
                    }
                }
            });
        });
    }

    // Desktop Widget Engine Instances and Lifecycles manager
    // Multiple clock widget instances (Vec instead of Option)
    let stats_widget: std::rc::Rc<std::cell::RefCell<Vec<StatsWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let instance_widgets: std::rc::Rc<std::cell::RefCell<HashMap<String, ExtraWidgetWindow>>> =
        std::rc::Rc::new(std::cell::RefCell::new(HashMap::new()));
    let year_progress_widget: std::rc::Rc<std::cell::RefCell<Option<YearProgressWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let day_progress_widget: std::rc::Rc<std::cell::RefCell<Option<DayProgressWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let month_progress_widget: std::rc::Rc<std::cell::RefCell<Option<MonthProgressWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let media_widget: std::rc::Rc<std::cell::RefCell<Option<MediaWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let notes_widget: std::rc::Rc<std::cell::RefCell<Option<NotesWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let todo_widget: std::rc::Rc<std::cell::RefCell<Option<TodoWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let quotes_widget: std::rc::Rc<std::cell::RefCell<Option<QuotesWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let picture_widget: std::rc::Rc<std::cell::RefCell<Option<PictureWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let video_widget: std::rc::Rc<std::cell::RefCell<Option<VideoFrameWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let battery_widget: std::rc::Rc<std::cell::RefCell<Option<BatteryPercentageWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let calendar_focus_widget: std::rc::Rc<std::cell::RefCell<Option<CalendarFocusWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let apps_container_widget: std::rc::Rc<std::cell::RefCell<Option<AppsContainerWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let focus_score_widget: std::rc::Rc<std::cell::RefCell<Option<FocusScoreWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let streak_widget: std::rc::Rc<std::cell::RefCell<Option<StreakWidgetWindow>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let focus_timer_runtime = std::rc::Rc::new(FocusTimerRuntime::new(
        settings.widgets.focus_timer_minutes.round() as i32,
    ));
    let focus_completed_shown = std::rc::Rc::new(std::cell::RefCell::new(false));
    let focus_bar_hidden_by_user = std::rc::Rc::new(std::cell::RefCell::new(false));
    let focus_bar_window: std::rc::Rc<std::cell::RefCell<Option<(FocusStatusBarWindow, FocusBarConfig)>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let focus_completion_overlay_window: std::rc::Rc<std::cell::RefCell<Option<FocusCompletionOverlayWindow>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let video_gif_timer: std::rc::Rc<std::cell::RefCell<Option<slint::Timer>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let video_playback_stop: std::rc::Rc<std::cell::RefCell<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let video_mci_alias: std::rc::Rc<std::cell::RefCell<String>> = std::rc::Rc::new(std::cell::RefCell::new(String::new()));

    let update_widget_lifecycles_cell: std::rc::Rc<std::cell::RefCell<Option<std::rc::Rc<dyn Fn()>>>> = std::rc::Rc::new(std::cell::RefCell::new(None));

    let update_widget_lifecycles = {
        let stats_w = stats_widget.clone();
        let instance_w = instance_widgets.clone();
        let year_progress_w = year_progress_widget.clone();
        let day_progress_w = day_progress_widget.clone();
        let month_progress_w = month_progress_widget.clone();
        let media_w = media_widget.clone();
        let notes_w = notes_widget.clone();
        let todo_w = todo_widget.clone();
        let quotes_w = quotes_widget.clone();
        let picture_w = picture_widget.clone();
        let video_w = video_widget.clone();
        let battery_w = battery_widget.clone();
        let calendar_focus_w = calendar_focus_widget.clone();
        let apps_container_w = apps_container_widget.clone();
        let focus_score_w = focus_score_widget.clone();
        let streak_w = streak_widget.clone();
        let focus_runtime = focus_timer_runtime.clone();
        let video_gif_timer_cell = video_gif_timer.clone();
        let video_playback_stop_cell = video_playback_stop.clone();
        let video_mci_alias_cell = video_mci_alias.clone();
        let services_for_lifecycle = services.clone();
        let s_ui_weak = settings_ui.as_weak();
        let update_cell = update_widget_lifecycles_cell.clone();
        move || {
            unsafe {
                save_current_widget_position_if_active(&stats_w);
                save_current_extra_widget_positions(&instance_w);
            }
            let settings = settings::RavenSettings::load();
            println!("[WIDGET-DEBUG] update_widget_lifecycles: enabled={}, stats_enabled={}, clock_enabled={} => should_stats={}", 
                     settings.widgets.enabled, settings.widgets.stats_enabled, settings.widgets.clock_enabled,
                     settings.widgets.enabled && (settings.widgets.stats_enabled || settings.widgets.clock_enabled));
            
            // ── STATS WIDGET Lifecycle (multi-instance) ──
            let should_stats = settings.widgets.enabled && (settings.widgets.stats_enabled || settings.widgets.clock_enabled);
            let desired_count: usize = if should_stats && settings.widgets.clock_count > 0.0 {
                settings.widgets.clock_count as usize
            } else {
                0
            };
            
            let mut stats_guard = stats_w.borrow_mut();
            let current_count = stats_guard.len();
            
            // Trim excess instances from the end
            while stats_guard.len() > desired_count {
                let w = stats_guard.pop().unwrap();
                println!("[LIFECYCLE-LOG] remove/hide: popping Clock Widget instance from manager");
                let _ = w.hide();
                // Clear global HWND for index 0 if we removed all
                if stats_guard.is_empty() {
                    crate::window::STATS_WIDGET_HWND.store(0, std::sync::atomic::Ordering::SeqCst);
                }
            }
            
            // Create new instances
            for idx in current_count..desired_count {
                let w = StatsWidgetWindow::new().unwrap();
                
                let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                println!("[LIFECYCLE-LOG] creation: StatsWidgetWindow (Clock Widget {}) created with ID = {}", idx, lifecycle_id);
                
                let tracker = std::rc::Rc::new(DropTracker {
                    id: lifecycle_id,
                    name: format!("Clock Widget {}", idx),
                });
                
                let tracker_dummy = tracker.clone();
                w.on_dummy_tracker(move || {
                    let _keep_alive = &tracker_dummy;
                });

                // Set unique instance title so FindWindowW can resolve each one
                let instance_title = format!("Raven Clock Widget {}", idx);
                w.set_instance_title(instance_title.clone().into());
                
                let inst = settings.widgets.get_clock_instance(idx);
                w.set_widget_size(inst.size.clone().into());
                w.set_bg_opacity(inst.opacity as f32);
                w.set_is_locked(settings.widgets.locked);
                w.set_text_color(parse_hex_color(&inst.cpu_color));
                w.set_show_seconds(inst.show_ram);
                w.set_border_radius_val(inst.border_radius as i32);
                
                let time_now = chrono::Local::now();
                let mut time_fmt = if inst.show_cpu { "%H:%M" } else { "%I:%M" }.to_string();
                if inst.show_ram { time_fmt.push_str(":%S"); }
                let mut time_str = time_now.format(&time_fmt).to_string();
                if !inst.show_cpu && time_str.starts_with('0') { time_str.remove(0); }
                let ampm_str = if !inst.show_cpu && inst.show_battery {
                    time_now.format("%p").to_string().to_lowercase()
                } else { "".to_string() };
                w.set_time_str(time_str.into());
                w.set_ampm_str(ampm_str.into());
                w.set_date_str(time_now.format("%A, %e %B").to_string().into());
                
                // Drag callback
                let w_weak = w.as_weak();
                let tracker_drag = tracker.clone();
                w.on_drag_window(move || {
                    let _keep_alive = &tracker_drag;
                    if let Some(w) = w_weak.upgrade() {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                unsafe {
                                    let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                        hwnd, 161,
                                        windows::Win32::Foundation::WPARAM(2),
                                        windows::Win32::Foundation::LPARAM(0),
                                    );
                                }
                            }
                        }
                    }
                });

                // Context Menu callback
                let w_weak2 = w.as_weak();
                let s_ui_weak2 = s_ui_weak.clone();
                let update_cell_c = update_cell.clone();
                let stats_w_cloned = stats_w.clone();
                let tracker_menu = tracker.clone();
                w.on_show_context_menu(move || {
                    let _keep_alive = &tracker_menu;
                    if let Some(w) = w_weak2.upgrade() {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                let update_fn = { let guard = update_cell_c.borrow(); guard.as_ref().cloned().unwrap() };
                                show_native_context_menu(
                                    hwnd,
                                    "stats",
                                    Some(idx),
                                    s_ui_weak2.clone(),
                                    update_fn,
                                    stats_w_cloned.clone(),
                                );
                            }
                        }
                    }
                });

                println!("[LIFECYCLE-LOG] show: calling show() on StatsWidgetWindow, ID = {}", lifecycle_id);
                w.show().unwrap();

                // Resolve HWND asynchronously via a timer to ensure winit has mapped the window
                let click_through = settings.widgets.click_through;
                let (w_px, h_px) = get_widget_dimensions(&inst.size);
                let mut stagger_x = 0;
                for i in 0..idx {
                    let prev_inst = settings.widgets.get_clock_instance(i);
                    let (w_dim, _) = get_widget_dimensions(&prev_inst.size);
                    stagger_x += w_dim + 20; // width + 20px gap
                }
                let initial_x = if inst.pos_x > 0.0 {
                    inst.pos_x as i32
                } else {
                    40 + stagger_x
                };
                let initial_y = if inst.pos_y > 0.0 {
                    inst.pos_y as i32
                } else {
                    120 + (idx as i32 * 20)
                };

                let w_weak = w.as_weak();
                fn resolve_clock_hwnd(
                    w_weak: slint::Weak<StatsWidgetWindow>,
                    idx: usize,
                    click_through: bool,
                    initial_x: i32,
                    initial_y: i32,
                    w_px: i32,
                    h_px: i32,
                ) {
                    slint::Timer::single_shot(
                        std::time::Duration::from_millis(50),
                        move || {
                            if let Some(w) = w_weak.upgrade() {
                                if let Some(hwnd) = get_window_hwnd(w.window()) {
                                    unsafe {
                                        println!("[WIDGET-DEBUG] Clock Widget {} HWND resolved: {:?}", idx, hwnd);
                                        widgets::log_window_info(&format!("WIDGET CREATION (Clock Widget {})", idx), hwnd, None);
                                        if idx == 0 {
                                            crate::window::STATS_WIDGET_HWND.store(hwnd.0, std::sync::atomic::Ordering::SeqCst);
                                        }
                                        widgets::setup_widget_window(hwnd, click_through, false);
                                        widgets::log_window_info(&format!("AFTER setup_widget_window (Clock Widget {})", idx), hwnd, None);
                                        widgets::position_widget_window(hwnd, initial_x, initial_y, w_px, h_px);
                                    }
                                } else {
                                    resolve_clock_hwnd(w_weak, idx, click_through, initial_x, initial_y, w_px, h_px);
                                }
                            }
                        }
                    );
                }
                resolve_clock_hwnd(w_weak, idx, click_through, initial_x, initial_y, w_px, h_px);
                
                println!("[LIFECYCLE-LOG] manager insertion: StatsWidgetWindow, ID = {} pushed into stats_guard", lifecycle_id);
                stats_guard.push(w);
            }
            
            // Update settings on existing instances (visual props + size)
            for (idx, w) in stats_guard.iter().enumerate() {
                let inst = settings.widgets.get_clock_instance(idx);
                w.set_instance_title(format!("Raven Clock Widget {}", idx).into());
                w.set_widget_size(inst.size.clone().into());
                w.set_bg_opacity(inst.opacity as f32);
                w.set_is_locked(settings.widgets.locked);
                w.set_text_color(parse_hex_color(&inst.cpu_color));
                w.set_show_seconds(inst.show_ram);
                w.set_border_radius_val(inst.border_radius as i32);
                
                unsafe {
                    let title_str = format!("Raven Clock Widget {}", idx);
                    let title_wide = wide(&title_str);
                    let hwnd = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                        None,
                        windows::core::PCWSTR(title_wide.as_ptr()),
                    );
                    if hwnd.0 != 0 {
                        widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                    }
                }
                
                // Re-register context menu callback with the CORRECT, updated index!
                let w_weak2 = w.as_weak();
                let s_ui_weak2 = s_ui_weak.clone();
                let update_cell_c = update_cell.clone();
                let stats_w_cloned = stats_w.clone();
                w.on_show_context_menu(move || {
                    if let Some(w) = w_weak2.upgrade() {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                let update_fn = { let guard = update_cell_c.borrow(); guard.as_ref().cloned().unwrap() };
                                show_native_context_menu(
                                    hwnd,
                                    "stats",
                                    Some(idx),
                                    s_ui_weak2.clone(),
                                    update_fn,
                                    stats_w_cloned.clone(),
                                );
                            }
                        }
                    }
                });

                // Resize and reposition the actual OS window dynamically
                unsafe {
                    use raw_window_handle::HasWindowHandle;
                    if let Ok(handle) = w.window().window_handle().window_handle() {
                        if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                            let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                            if hwnd.0 != 0 {
                                let (w_px, h_px) = get_widget_dimensions(&inst.size);
                                let mut stagger_x = 0;
                                for i in 0..idx {
                                    let prev_inst = settings.widgets.get_clock_instance(i);
                                    let (w_dim, _) = get_widget_dimensions(&prev_inst.size);
                                    stagger_x += w_dim + 20; // width + 20px gap
                                }
                                let initial_x = if inst.pos_x > 0.0 {
                                    inst.pos_x as i32
                                } else {
                                    40 + stagger_x
                                };
                                let initial_y = if inst.pos_y > 0.0 {
                                    inst.pos_y as i32
                                } else {
                                    120 + (idx as i32 * 20)
                                };
                                widgets::position_widget_window(hwnd, initial_x, initial_y, w_px, h_px);
                            }
                        }
                    }
                }
            }

            // ── YEAR PROGRESS WIDGET Lifecycle ──
            let should_year_progress = settings.widgets.enabled && settings.widgets.year_journey_enabled;
            let mut yp_guard = year_progress_w.borrow_mut();
            if should_year_progress {
                if yp_guard.is_none() {
                    let w = YearProgressWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: YearProgressWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Year Progress Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    // Set unique instance title so FindWindowW can resolve it
                    w.set_instance_title(format!("Raven Year Progress Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_yp = update_cell.clone();
                    let stats_w_yp = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_yp.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "year_progress",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_yp.clone(),
                                    );
                                }
                            }
                        }
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on YearProgressWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();

                    // Calculate initial values
                    update_year_progress_widget_properties(&w);
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    // Setup window subclassing and positioning via a timer to let it resolve
                    let click_through = settings.widgets.click_through;
                    let yp_x = settings.widgets.year_journey_pos_x as i32;
                    let yp_y = settings.widgets.year_journey_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_yp_x_hwnd(
                        w_weak: slint::Weak<YearProgressWidgetWindow>,
                        click_through: bool,
                        yp_x: i32,
                        yp_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] YearProgressWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, yp_x, yp_y, 320, 150);
                                        }
                                    } else {
                                        resolve_yp_x_hwnd(w_weak, click_through, yp_x, yp_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_yp_x_hwnd(w_weak, click_through, yp_x, yp_y);

                    *yp_guard = Some(w);
                } else if let Some(w) = yp_guard.as_ref() {
                    // Update dynamic properties
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    
                    // Update positions/click-through if changed
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.year_journey_pos_x as i32,
                                        settings.widgets.year_journey_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(w) = yp_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding YearProgressWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── DAY PROGRESS WIDGET Lifecycle ──
            let should_day_progress = settings.widgets.enabled && settings.widgets.day_journey_enabled;
            let mut dp_guard = day_progress_w.borrow_mut();
            if should_day_progress {
                if dp_guard.is_none() {
                    let w = DayProgressWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: DayProgressWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Day Progress Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    // Set unique instance title so FindWindowW can resolve it
                    w.set_instance_title(format!("Raven Day Progress Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_dp = update_cell.clone();
                    let stats_w_dp = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_dp.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "day_progress",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_dp.clone(),
                                    );
                                }
                            }
                        }
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on DayProgressWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();

                    // Calculate initial values
                    update_day_progress_widget_properties(&w);
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    // Setup window subclassing and positioning via a timer
                    let click_through = settings.widgets.click_through;
                    let dp_x = settings.widgets.day_journey_pos_x as i32;
                    let dp_y = settings.widgets.day_journey_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_dp_x_hwnd(
                        w_weak: slint::Weak<DayProgressWidgetWindow>,
                        click_through: bool,
                        dp_x: i32,
                        dp_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] DayProgressWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, dp_x, dp_y, 320, 150);
                                        }
                                    } else {
                                        resolve_dp_x_hwnd(w_weak, click_through, dp_x, dp_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_dp_x_hwnd(w_weak, click_through, dp_x, dp_y);

                    *dp_guard = Some(w);
                } else if let Some(w) = dp_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.day_journey_pos_x as i32,
                                        settings.widgets.day_journey_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(w) = dp_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding DayProgressWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── MONTH PROGRESS WIDGET Lifecycle ──
            let should_month_progress = settings.widgets.enabled && settings.widgets.month_journey_enabled;
            let mut mp_guard = month_progress_w.borrow_mut();
            if should_month_progress {
                if mp_guard.is_none() {
                    let w = MonthProgressWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: MonthProgressWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Month Progress Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    // Set unique instance title so FindWindowW can resolve it
                    w.set_instance_title(format!("Raven Month Progress Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_mp = update_cell.clone();
                    let stats_w_mp = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_mp.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "month_progress",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_mp.clone(),
                                    );
                                }
                            }
                        }
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on MonthProgressWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();

                    // Calculate initial values
                    update_month_progress_widget_properties(&w);
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    // Setup window subclassing and positioning via a timer
                    let click_through = settings.widgets.click_through;
                    let mp_x = settings.widgets.month_journey_pos_x as i32;
                    let mp_y = settings.widgets.month_journey_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_mp_x_hwnd(
                        w_weak: slint::Weak<MonthProgressWidgetWindow>,
                        click_through: bool,
                        mp_x: i32,
                        mp_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] MonthProgressWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, mp_x, mp_y, 320, 150);
                                        }
                                    } else {
                                        resolve_mp_x_hwnd(w_weak, click_through, mp_x, mp_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_mp_x_hwnd(w_weak, click_through, mp_x, mp_y);

                    *mp_guard = Some(w);
                } else if let Some(w) = mp_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.month_journey_pos_x as i32,
                                        settings.widgets.month_journey_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(w) = mp_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding MonthProgressWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── MEDIA WIDGET Lifecycle ──
            let should_media = settings.widgets.enabled && settings.widgets.media_enabled;
            let mut m_guard = media_w.borrow_mut();
            if should_media {
                if m_guard.is_none() {
                    let w = MediaWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: MediaWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Media Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    // Set unique instance title so FindWindowW can resolve it
                    w.set_instance_title(format!("Raven Media Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_m = update_cell.clone();
                    let stats_w_m = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_m.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "media",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_m.clone(),
                                    );
                                }
                            }
                        }
                    });

                    // Playback controls callback listeners
                    let s_cloned = services_for_lifecycle.clone();
                    w.on_skip_backward(move || {
                        s_cloned.media.seek(false);
                    });

                    let s_cloned = services_for_lifecycle.clone();
                    w.on_prev_track(move || {
                        s_cloned.media.previous();
                    });

                    let s_cloned = services_for_lifecycle.clone();
                    w.on_toggle_play(move || {
                        s_cloned.media.play_pause();
                    });

                    let s_cloned = services_for_lifecycle.clone();
                    w.on_next_track(move || {
                        s_cloned.media.next();
                    });

                    let s_cloned = services_for_lifecycle.clone();
                    w.on_skip_forward(move || {
                        s_cloned.media.seek(true);
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on MediaWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    // Setup window subclassing and positioning via a timer
                    let click_through = settings.widgets.click_through;
                    let m_x = settings.widgets.media_pos_x as i32;
                    let m_y = settings.widgets.media_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_m_x_hwnd(
                        w_weak: slint::Weak<MediaWidgetWindow>,
                        click_through: bool,
                        m_x: i32,
                        m_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] MediaWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, m_x, m_y, 320, 150);
                                        }
                                    } else {
                                        resolve_m_x_hwnd(w_weak, click_through, m_x, m_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_m_x_hwnd(w_weak, click_through, m_x, m_y);

                    *m_guard = Some(w);
                } else if let Some(w) = m_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.media_pos_x as i32,
                                        settings.widgets.media_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(w) = m_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding MediaWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── NOTES WIDGET Lifecycle ──
            let should_notes = settings.widgets.enabled && settings.widgets.notes_enabled;
            let mut notes_guard = notes_w.borrow_mut();
            if should_notes {
                if notes_guard.is_none() {
                    let w = NotesWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: NotesWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Notes Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    w.set_instance_title(format!("Raven Notes Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_n = update_cell.clone();
                    let stats_w_n = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_n.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "notes",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_n.clone(),
                                    );
                                }
                            }
                        }
                    });

                    // Notes changed text callback
                    let update_widget_lifecycles_notes_change = update_cell.clone();
                    w.on_notes_changed(move |text| {
                        settings::set_string(&["widgets", "notes_text"], text.as_str());
                        let update_fn = { let guard = update_widget_lifecycles_notes_change.borrow(); guard.as_ref().cloned().unwrap() };
                        update_fn();
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on NotesWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();

                    // Calculate initial values
                    w.set_notes_text(settings.widgets.notes_text.clone().into());
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    // Setup window subclassing and positioning via a timer
                    let click_through = settings.widgets.click_through;
                    let n_x = settings.widgets.notes_pos_x as i32;
                    let n_y = settings.widgets.notes_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_n_x_hwnd(
                        w_weak: slint::Weak<NotesWidgetWindow>,
                        click_through: bool,
                        n_x: i32,
                        n_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] NotesWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, true);
                                            widgets::position_widget_window_from_left(hwnd, n_x, n_y, 320, 150);
                                        }
                                    } else {
                                        resolve_n_x_hwnd(w_weak, click_through, n_x, n_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_n_x_hwnd(w_weak, click_through, n_x, n_y);

                    *notes_guard = Some(w);
                } else if let Some(w) = notes_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    w.set_notes_text(settings.widgets.notes_text.clone().into());
                    
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.notes_pos_x as i32,
                                        settings.widgets.notes_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(w) = notes_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding NotesWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── TODO WIDGET Lifecycle ──
            let should_todo = settings.widgets.enabled && settings.widgets.todo_enabled;
            let mut todo_guard = todo_w.borrow_mut();
            if should_todo {
                if todo_guard.is_none() {
                    let w = TodoWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: TodoWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Todo Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    w.set_instance_title(format!("Raven Todo Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_t = update_cell.clone();
                    let stats_w_t = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_t.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "todo",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_t.clone(),
                                    );
                                }
                            }
                        }
                    });

                    // To-Do list operations callbacks
                    let update_widget_lifecycles_todo_add = update_cell.clone();
                    w.on_add_task(move |text| {
                        settings::todo_add_item(text.as_str());
                        let update_fn = { let guard = update_widget_lifecycles_todo_add.borrow(); guard.as_ref().cloned().unwrap() };
                        update_fn();
                    });

                    let update_widget_lifecycles_todo_toggle = update_cell.clone();
                    w.on_toggle_task(move |id| {
                        settings::todo_toggle_item(id);
                        let update_fn = { let guard = update_widget_lifecycles_todo_toggle.borrow(); guard.as_ref().cloned().unwrap() };
                        update_fn();
                    });

                    let update_widget_lifecycles_todo_delete = update_cell.clone();
                    w.on_delete_task(move |id| {
                        settings::todo_delete_item(id);
                        let update_fn = { let guard = update_widget_lifecycles_todo_delete.borrow(); guard.as_ref().cloned().unwrap() };
                        update_fn();
                    });

                    let update_widget_lifecycles_todo_move = update_cell.clone();
                    w.on_move_task(move |id, is_up| {
                        settings::todo_move_item(id, is_up);
                        let update_fn = { let guard = update_widget_lifecycles_todo_move.borrow(); guard.as_ref().cloned().unwrap() };
                        update_fn();
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on TodoWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();

                    // Calculate initial values
                    w.set_accent_color(parse_hex_color(&settings.widgets.todo_accent_color));
                    
                    let slint_items: Vec<TodoItem> = settings.widgets.todo_items.iter()
                        .filter(|item| !(settings.widgets.todo_hide_completed && item.completed))
                        .map(|item| TodoItem {
                            id: item.id,
                            text: item.text.clone().into(),
                            completed: item.completed,
                        })
                        .collect();
                    w.set_todo_items(std::rc::Rc::new(slint::VecModel::from(slint_items)).into());

                    // Setup window subclassing and positioning via a timer
                    let click_through = settings.widgets.click_through;
                    let t_x = settings.widgets.todo_pos_x as i32;
                    let t_y = settings.widgets.todo_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_t_x_hwnd(
                        w_weak: slint::Weak<TodoWidgetWindow>,
                        click_through: bool,
                        t_x: i32,
                        t_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] TodoWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, true);
                                            widgets::position_widget_window_from_left(hwnd, t_x, t_y, 320, 150);
                                        }
                                    } else {
                                        resolve_t_x_hwnd(w_weak, click_through, t_x, t_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_t_x_hwnd(w_weak, click_through, t_x, t_y);

                    *todo_guard = Some(w);
                } else if let Some(w) = todo_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    w.set_accent_color(parse_hex_color(&settings.widgets.todo_accent_color));
                    
                    let slint_items: Vec<TodoItem> = settings.widgets.todo_items.iter()
                        .filter(|item| !(settings.widgets.todo_hide_completed && item.completed))
                        .map(|item| TodoItem {
                            id: item.id,
                            text: item.text.clone().into(),
                            completed: item.completed,
                        })
                        .collect();
                    w.set_todo_items(std::rc::Rc::new(slint::VecModel::from(slint_items)).into());
                    
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.todo_pos_x as i32,
                                        settings.widgets.todo_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(w) = todo_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding TodoWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── QUOTES WIDGET Lifecycle ──
            let should_quotes = settings.widgets.enabled && settings.widgets.quotes_enabled;
            let mut quotes_guard = quotes_w.borrow_mut();
            if should_quotes {
                if quotes_guard.is_none() {
                    let w = QuotesWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: QuotesWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Quotes Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    w.set_instance_title(format!("Raven Quotes Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_q = update_cell.clone();
                    let stats_w_q = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_q.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "quotes",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_q.clone(),
                                    );
                                }
                            }
                        }
                    });

                    // Refresh quote callback (manual click)
                    let w_weak_refresh = w.as_weak();
                    w.on_next_quote(move || {
                        if let Some(w) = w_weak_refresh.upgrade() {
                            let settings = settings::RavenSettings::load();
                            let mut all_quotes: Vec<(String, String)> = DEFAULT_QUOTES.iter()
                                .map(|(q, a)| (q.to_string(), a.to_string()))
                                .collect();
                            for custom in &settings.widgets.quotes_custom_quotes {
                                let parts: Vec<&str> = custom.split('|').collect();
                                if parts.len() == 2 {
                                    all_quotes.push((parts[0].to_string(), parts[1].to_string()));
                                } else if parts.len() == 1 {
                                    all_quotes.push((parts[0].to_string(), "Unknown".to_string()));
                                }
                            }
                            use rand::seq::SliceRandom;
                            let mut rng = rand::thread_rng();
                            if let Some((quote, author)) = all_quotes.choose(&mut rng) {
                                w.set_quote_text(quote.clone().into());
                                w.set_quote_author(author.clone().into());
                            }
                        }
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on QuotesWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();

                    // Initial quote
                    let mut all_quotes: Vec<(String, String)> = DEFAULT_QUOTES.iter()
                        .map(|(q, a)| (q.to_string(), a.to_string()))
                        .collect();
                    for custom in &settings.widgets.quotes_custom_quotes {
                        let parts: Vec<&str> = custom.split('|').collect();
                        if parts.len() == 2 {
                            all_quotes.push((parts[0].to_string(), parts[1].to_string()));
                        } else if parts.len() == 1 {
                            all_quotes.push((parts[0].to_string(), "Unknown".to_string()));
                        }
                    }
                    use rand::seq::SliceRandom;
                    let mut rng = rand::thread_rng();
                    if let Some((quote, author)) = all_quotes.choose(&mut rng) {
                        w.set_quote_text(quote.clone().into());
                        w.set_quote_author(author.clone().into());
                    }
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    // Setup window subclassing and positioning via a timer
                    let click_through = settings.widgets.click_through;
                    let q_x = settings.widgets.quotes_pos_x as i32;
                    let q_y = settings.widgets.quotes_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_q_x_hwnd(
                        w_weak: slint::Weak<QuotesWidgetWindow>,
                        click_through: bool,
                        q_x: i32,
                        q_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] QuotesWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, q_x, q_y, 320, 150);
                                        }
                                    } else {
                                        resolve_q_x_hwnd(w_weak, click_through, q_x, q_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_q_x_hwnd(w_weak, click_through, q_x, q_y);

                    *quotes_guard = Some(w);
                } else if let Some(w) = quotes_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.quotes_pos_x as i32,
                                        settings.widgets.quotes_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(w) = quotes_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding QuotesWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── PICTURE WIDGET Lifecycle ──
            let should_picture = settings.widgets.enabled && settings.widgets.picture_enabled;
            let mut picture_guard = picture_w.borrow_mut();
            if should_picture {
                if picture_guard.is_none() {
                    let w = PictureWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: PictureWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Picture Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    w.set_instance_title(format!("Raven Picture Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_p = update_cell.clone();
                    let stats_w_p = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            w.set_show_camera_overlay(false);
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_p.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "picture",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_p.clone(),
                                    );
                                }
                            }
                        }
                    });

                    // Select picture callback
                    let w_weak_pic = w.as_weak();
                    let s_ui_weak_pic = s_ui_weak.clone();
                    w.on_select_picture(move || {
                        if let Some(w_win) = w_weak_pic.upgrade() {
                            w_win.set_show_camera_overlay(false);
                        }
                        let w_weak_c = w_weak_pic.clone();
                        let s_ui_c = s_ui_weak_pic.clone();
                        std::thread::spawn(move || {
                            if let Some(path) = select_image_file() {
                                let _ = slint::invoke_from_event_loop(move || {
                                    settings::set_string(&["widgets", "picture_path"], &path);
                                    if let Some(w_win) = w_weak_c.upgrade() {
                                        w_win.set_picture_path(path.clone().into());
                                        if !path.is_empty() {
                                            if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&path)) {
                                                w_win.set_picture_img(img);
                                                w_win.set_has_picture(true);
                                            } else {
                                                w_win.set_has_picture(false);
                                            }
                                        } else {
                                            w_win.set_has_picture(false);
                                        }
                                    }
                                    if let Some(s_win) = s_ui_c.upgrade() {
                                        s_win.set_picture_selected_path(path.into());
                                    }
                                    GLOBAL_UPDATE_LIFECYCLES.with(|cell| {
                                        if let Some(f) = cell.borrow().as_ref() {
                                            f();
                                        }
                                    });
                                });
                            }
                        });
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on PictureWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    w.set_show_camera_overlay(false);

                    // Initial values
                    let path = settings.widgets.picture_path.clone();
                    w.set_picture_path(path.clone().into());
                    if !path.is_empty() {
                        if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&path)) {
                            w.set_picture_img(img);
                            w.set_has_picture(true);
                        } else {
                            w.set_has_picture(false);
                        }
                    } else {
                        w.set_has_picture(false);
                    }

                    // Setup window subclassing and positioning via a timer
                    let click_through = settings.widgets.click_through;
                    let p_x = settings.widgets.picture_pos_x as i32;
                    let p_y = settings.widgets.picture_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_p_x_hwnd(
                        w_weak: slint::Weak<PictureWidgetWindow>,
                        click_through: bool,
                        p_x: i32,
                        p_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] PictureWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, p_x, p_y, 320, 150);
                                        }
                                    } else {
                                        resolve_p_x_hwnd(w_weak, click_through, p_x, p_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_p_x_hwnd(w_weak, click_through, p_x, p_y);

                    *picture_guard = Some(w);
                } else if let Some(w) = picture_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    let path = settings.widgets.picture_path.clone();
                    let old_path: String = w.get_picture_path().into();
                    if path != old_path {
                        w.set_picture_path(path.clone().into());
                        if !path.is_empty() {
                            if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&path)) {
                                w.set_picture_img(img);
                                w.set_has_picture(true);
                            } else {
                                w.set_has_picture(false);
                            }
                        } else {
                            w.set_has_picture(false);
                        }
                    }
                    
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.picture_pos_x as i32,
                                        settings.widgets.picture_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(w) = picture_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding PictureWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── VIDEO FRAME WIDGET Lifecycle ──
            let should_video = settings.widgets.enabled && settings.widgets.video_enabled;
            let mut video_guard = video_w.borrow_mut();
            if should_video {
                if video_guard.is_none() {
                    let w = VideoFrameWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: VideoFrameWidgetWindow created with ID = {}", lifecycle_id);
                    
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Video Frame Widget".to_string(),
                    });
                    
                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    let alias = format!("raven_video_widget_{}", lifecycle_id);
                    *video_mci_alias_cell.borrow_mut() = alias.clone();
                    w.set_instance_title(format!("Raven Video Frame Widget {}", lifecycle_id).into());

                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_v = update_cell.clone();
                    let stats_w_v = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            w.set_show_video_overlay(false);
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_v.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "video",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_v.clone(),
                                    );
                                }
                            }
                        }
                    });

                    let w_weak_vid = w.as_weak();
                    let s_ui_weak_vid = s_ui_weak.clone();
                    w.on_select_video(move || {
                        if let Some(w_win) = w_weak_vid.upgrade() {
                            w_win.set_show_video_overlay(false);
                        }
                        let s_ui_c = s_ui_weak_vid.clone();
                        std::thread::spawn(move || {
                            if let Some(path) = select_video_file() {
                                let _ = slint::invoke_from_event_loop(move || {
                                    settings::set_string(&["widgets", "video_path"], &path);
                                    if let Some(s_win) = s_ui_c.upgrade() {
                                        s_win.set_video_selected_path(path.into());
                                    }
                                    GLOBAL_UPDATE_LIFECYCLES.with(|cell| {
                                        if let Some(f) = cell.borrow().as_ref() {
                                            f();
                                        }
                                    });
                                });
                            }
                        });
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on VideoFrameWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    w.set_show_video_overlay(false);

                    let path = settings.widgets.video_path.clone();
                    w.set_video_path(path.clone().into());
                    configure_video_widget_media(&w, &path, &alias, &video_gif_timer_cell, &video_playback_stop_cell);

                    let click_through = settings.widgets.click_through;
                    let v_x = settings.widgets.video_pos_x as i32;
                    let v_y = settings.widgets.video_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_v_x_hwnd(
                        w_weak: slint::Weak<VideoFrameWidgetWindow>,
                        click_through: bool,
                        v_x: i32,
                        v_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] VideoFrameWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, v_x, v_y, 320, 150);
                                        }
                                    } else {
                                        resolve_v_x_hwnd(w_weak, click_through, v_x, v_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_v_x_hwnd(w_weak, click_through, v_x, v_y);

                    *video_guard = Some(w);
                } else if let Some(w) = video_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    let path = settings.widgets.video_path.clone();
                    let old_path: String = w.get_video_path().into();
                    let alias = video_mci_alias_cell.borrow().clone();
                    if path != old_path {
                        configure_video_widget_media(w, &path, &alias, &video_gif_timer_cell, &video_playback_stop_cell);
                    }
                    
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.video_pos_x as i32,
                                        settings.widgets.video_pos_y as i32,
                                        320,
                                        150
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(timer) = video_gif_timer_cell.borrow_mut().take() {
                    timer.stop();
                }
                if let Some(stop_flag) = video_playback_stop_cell.borrow_mut().take() {
                    stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                let alias = video_mci_alias_cell.borrow().clone();
                if !alias.is_empty() {
                    stop_mci_video(&alias);
                }
                if let Some(w) = video_guard.take() {
                    println!("[LIFECYCLE-LOG] remove/hide: hiding VideoFrameWidgetWindow");
                    let _ = w.hide();
                }
            }

            // ── BATTERY PERCENTAGE WIDGET Lifecycle ──
            let should_battery_widget = settings.widgets.enabled && settings.widgets.battery_widget_enabled;
            let mut battery_guard = battery_w.borrow_mut();
            if should_battery_widget {
                if battery_guard.is_none() {
                    let w = BatteryPercentageWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("[LIFECYCLE-LOG] creation: BatteryPercentageWidgetWindow created with ID = {}", lifecycle_id);

                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Battery Percentage Widget".to_string(),
                    });

                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    w.set_instance_title(format!("Raven Battery Percentage Widget {}", lifecycle_id).into());

                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_b = update_cell.clone();
                    let stats_w_b = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = { let guard = update_cell_b.borrow(); guard.as_ref().cloned().unwrap() };
                                    show_native_context_menu(
                                        hwnd,
                                        "battery_widget",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_b.clone(),
                                    );
                                }
                            }
                        }
                    });

                    println!("[LIFECYCLE-LOG] show: calling show() on BatteryPercentageWidgetWindow, ID = {}", lifecycle_id);
                    w.show().unwrap();
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    if let Some((pct, charging)) = read_live_battery_status() {
                        w.set_battery_pct(pct);
                        w.set_is_charging(charging);
                        w.set_progress_ring_img(render_battery_progress_ring(pct));
                    }

                    let click_through = settings.widgets.click_through;
                    let b_x = settings.widgets.battery_widget_pos_x as i32;
                    let b_y = settings.widgets.battery_widget_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_b_x_hwnd(
                        w_weak: slint::Weak<BatteryPercentageWidgetWindow>,
                        click_through: bool,
                        b_x: i32,
                        b_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] BatteryPercentageWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, b_x, b_y, 190, 190);
                                        }
                                    } else {
                                        resolve_b_x_hwnd(w_weak, click_through, b_x, b_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_b_x_hwnd(w_weak, click_through, b_x, b_y);

                    *battery_guard = Some(w);
                } else if let Some(w) = battery_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    if let Some((pct, charging)) = read_live_battery_status() {
                        w.set_battery_pct(pct);
                        w.set_is_charging(charging);
                        w.set_progress_ring_img(render_battery_progress_ring(pct));
                    }

                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                if hwnd.0 != 0 {
                                    widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                    widgets::position_widget_window_from_left(
                                        hwnd,
                                        settings.widgets.battery_widget_pos_x as i32,
                                        settings.widgets.battery_widget_pos_y as i32,
                                        190,
                                        190
                                    );
                                }
                            }
                        }
                    }
                }
            } else if let Some(w) = battery_guard.take() {
                println!("[LIFECYCLE-LOG] remove/hide: hiding BatteryPercentageWidgetWindow");
                let _ = w.hide();
            }

            // ── CALENDAR FOCUS WIDGET Lifecycle ──
            let should_calendar_focus =
                settings.widgets.enabled && settings.widgets.calendar_focus_enabled;
            let mut focus_guard = calendar_focus_w.borrow_mut();
            if should_calendar_focus {
                if focus_guard.is_none() {
                    let w = CalendarFocusWidgetWindow::new().unwrap();
                    let lifecycle_id =
                        NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Calendar Focus Widget".to_string(),
                    });

                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });
                    w.set_instance_title(
                        format!("Raven Calendar Focus Widget {}", lifecycle_id).into(),
                    );

                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) =
                                    handle.as_raw()
                                {
                                    let hwnd = windows::Win32::Foundation::HWND(
                                        win32.hwnd.get() as _,
                                    );
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd,
                                            161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    let install_timer_menu = |widget: &CalendarFocusWidgetWindow,
                                              configure_only: bool| {
                        let weak = widget.as_weak();
                        let runtime = focus_runtime.clone();
                        let s_ui = s_ui_weak.clone();
                        let update_cell = update_cell.clone();
                        let stats_widgets = stats_w.clone();
                        move || {
                            if let Some(widget) = weak.upgrade() {
                                use raw_window_handle::HasWindowHandle;
                                if let Ok(handle) =
                                    widget.window().window_handle().window_handle()
                                {
                                    if let raw_window_handle::RawWindowHandle::Win32(win32) =
                                        handle.as_raw()
                                    {
                                        let hwnd = windows::Win32::Foundation::HWND(
                                            win32.hwnd.get() as _,
                                        );
                                        let update_fn = {
                                            let guard = update_cell.borrow();
                                            guard.as_ref().cloned().unwrap()
                                        };
                                        show_focus_timer_context_menu(
                                            hwnd,
                                            widget.as_weak(),
                                            runtime.clone(),
                                            s_ui.clone(),
                                            update_fn,
                                            stats_widgets.clone(),
                                            configure_only,
                                        );
                                    }
                                }
                            }
                        }
                    };
                    w.on_show_context_menu(install_timer_menu(&w, false));
                    w.on_configure_timer(install_timer_menu(&w, true));
                    let toggle_weak = w.as_weak();
                    let toggle_runtime = focus_runtime.clone();
                    w.on_toggle_timer(move || {
                        toggle_runtime.toggle();
                        if let Some(widget) = toggle_weak.upgrade() {
                            update_calendar_focus_widget_properties(
                                &widget,
                                &toggle_runtime,
                            );
                        }
                    });

                    w.show().unwrap();
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    update_calendar_focus_widget_properties(&w, &focus_runtime);

                    let click_through = settings.widgets.click_through;
                    let focus_x = settings.widgets.calendar_focus_pos_x as i32;
                    let focus_y = settings.widgets.calendar_focus_pos_y as i32;
                    let w_weak = w.as_weak();
                    fn resolve_focus_x_hwnd(
                        w_weak: slint::Weak<CalendarFocusWidgetWindow>,
                        click_through: bool,
                        focus_x: i32,
                        focus_y: i32,
                    ) {
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(50),
                            move || {
                                if let Some(w) = w_weak.upgrade() {
                                    if let Some(hwnd) = get_window_hwnd(w.window()) {
                                        unsafe {
                                            println!("[WIDGET-DEBUG] GenericWidgetWindow HWND successfully resolved: {:?}", hwnd);
                                            widgets::setup_widget_window(hwnd, click_through, false);
                                            widgets::position_widget_window_from_left(hwnd, focus_x, focus_y, 320, 150);
                                        }
                                    } else {
                                        resolve_focus_x_hwnd(w_weak, click_through, focus_x, focus_y);
                                    }
                                }
                            }
                        );
                    }
                    resolve_focus_x_hwnd(w_weak, click_through, focus_x, focus_y);

                    *focus_guard = Some(w);
                } else if let Some(w) = focus_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    update_calendar_focus_widget_properties(w, &focus_runtime);
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) =
                                handle.as_raw()
                            {
                                let hwnd = windows::Win32::Foundation::HWND(
                                    win32.hwnd.get() as _,
                                );
                                widgets::set_widget_click_through(
                                    hwnd,
                                    settings.widgets.click_through,
                                );
                                widgets::position_widget_window_from_left(
                                    hwnd,
                                    settings.widgets.calendar_focus_pos_x as i32,
                                    settings.widgets.calendar_focus_pos_y as i32,
                                    356,
                                    168,
                                );
                            }
                        }
                    }
                }
            } else if let Some(w) = focus_guard.take() {
                let _ = w.hide();
            }

            // ── STREAK WIDGET Lifecycle ──
            let should_streak = settings.widgets.enabled && settings.widgets.streak_widget_enabled;
            let mut streak_guard = streak_w.borrow_mut();
            if should_streak {
                if streak_guard.is_none() {
                    let w = StreakWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Calendar Widget".to_string(),
                    });

                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });
                    w.set_instance_title(format!("Raven Calendar Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_c = update_cell.clone();
                    let stats_w_cloned = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(widget) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = widget.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = {
                                        let guard = update_cell_c.borrow();
                                        guard.as_ref().cloned().unwrap()
                                    };
                                    show_native_context_menu(
                                        hwnd,
                                        "streak",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_cloned.clone(),
                                    );
                                }
                            }
                        }
                    });

                    // Rename callback
                    w.on_rename_streak(move |new_name| {
                        settings::set_streak_name(&new_name);
                    });

                    sync_calendar_widget_date(&w);

                    w.show().unwrap();
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    let click_through = settings.widgets.click_through;
                    let st_x = settings.widgets.streak_widget_pos_x as i32;
                    let st_y = settings.widgets.streak_widget_pos_y as i32;
                    let w_weak_show = w.as_weak();

                    slint::Timer::single_shot(
                        std::time::Duration::from_millis(50),
                        move || {
                            if let Some(w) = w_weak_show.upgrade() {
                                if let Some(hwnd) = get_window_hwnd(w.window()) {
                                    unsafe {
                                        println!("[WIDGET-DEBUG] Streak Widget HWND successfully resolved: {:?}", hwnd);
                                        widgets::setup_widget_window(hwnd, click_through, false);
                                        widgets::position_widget_window_from_left(hwnd, st_x, st_y, 320, 150);
                                    }
                                }
                            }
                        }
                    );

                    *streak_guard = Some(w);
                } else if let Some(w) = streak_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    sync_calendar_widget_date(w);
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                widgets::position_widget_window_from_left(
                                    hwnd,
                                    settings.widgets.streak_widget_pos_x as i32,
                                    settings.widgets.streak_widget_pos_y as i32,
                                    320,
                                    150,
                                );
                            }
                        }
                    }
                }
            } else if let Some(w) = streak_guard.take() {
                let _ = w.hide();
            }

            // ── FOCUS SCORE WIDGET Lifecycle ──
            let should_focus_score = settings.widgets.enabled && settings.widgets.focus_score_widget_enabled;
            let mut score_guard = focus_score_w.borrow_mut();
            if should_focus_score {
                if score_guard.is_none() {
                    let w = FocusScoreWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Focus Score Widget".to_string(),
                    });

                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });
                    w.set_instance_title(format!("Raven Focus Score Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_c = update_cell.clone();
                    let stats_w_cloned = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(widget) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = widget.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = {
                                        let guard = update_cell_c.borrow();
                                        guard.as_ref().cloned().unwrap()
                                    };
                                    show_native_context_menu(
                                        hwnd,
                                        "focus_score",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_cloned.clone(),
                                    );
                                }
                            }
                        }
                    });

                    let (total_label, score, preset_rows) = focus_score_widget_data(&settings);
                    w.set_preset_rows(std::rc::Rc::new(slint::VecModel::from(preset_rows)).into());
                    w.set_total_focus_label(total_label.into());
                    w.set_focus_score(score);
                    w.set_score_ring_img(render_focus_score_ring(score));

                    w.show().unwrap();
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    let click_through = settings.widgets.click_through;
                    let fs_x = settings.widgets.focus_score_widget_pos_x as i32;
                    let fs_y = settings.widgets.focus_score_widget_pos_y as i32;
                    let w_weak_show = w.as_weak();

                    slint::Timer::single_shot(
                        std::time::Duration::from_millis(50),
                        move || {
                            if let Some(w) = w_weak_show.upgrade() {
                                if let Some(hwnd) = get_window_hwnd(w.window()) {
                                    unsafe {
                                        println!("[WIDGET-DEBUG] Focus Score Widget HWND successfully resolved: {:?}", hwnd);
                                        widgets::setup_widget_window(hwnd, click_through, false);
                                        widgets::position_widget_window_from_left(hwnd, fs_x, fs_y, 380, 190);
                                    }
                                }
                            }
                        }
                    );

                    *score_guard = Some(w);
                } else if let Some(w) = score_guard.as_ref() {
                    let (total_label, score, preset_rows) = focus_score_widget_data(&settings);
                    w.set_preset_rows(std::rc::Rc::new(slint::VecModel::from(preset_rows)).into());
                    w.set_total_focus_label(total_label.into());
                    w.set_focus_score(score);
                    w.set_score_ring_img(render_focus_score_ring(score));
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                widgets::position_widget_window_from_left(
                                    hwnd,
                                    settings.widgets.focus_score_widget_pos_x as i32,
                                    settings.widgets.focus_score_widget_pos_y as i32,
                                    380,
                                    190,
                                );
                            }
                        }
                    }
                }
            } else if let Some(w) = score_guard.take() {
                let _ = w.hide();
            }

            // ── APPS CONTAINER WIDGET Lifecycle ──
            let should_apps_container = settings.widgets.enabled && settings.widgets.apps_container_enabled;
            let mut apps_guard = apps_container_w.borrow_mut();
            if should_apps_container {
                if apps_guard.is_none() {
                    let w = AppsContainerWidgetWindow::new().unwrap();
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let tracker = std::rc::Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: "Apps Container Widget".to_string(),
                    });

                    let tracker_dummy = tracker.clone();
                    w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });
                    w.set_instance_title(format!("Raven Apps Container Widget {}", lifecycle_id).into());

                    // Drag callback
                    let w_weak = w.as_weak();
                    let tracker_drag = tracker.clone();
                    w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                            hwnd, 161,
                                            windows::Win32::Foundation::WPARAM(2),
                                            windows::Win32::Foundation::LPARAM(0),
                                        );
                                    }
                                }
                            }
                        }
                    });

                    // Context Menu callback
                    let w_weak2 = w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_c = update_cell.clone();
                    let stats_w_cloned = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(widget) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = widget.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = {
                                        let guard = update_cell_c.borrow();
                                        guard.as_ref().cloned().unwrap()
                                    };
                                    show_native_context_menu(
                                        hwnd,
                                        "apps_container",
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_cloned.clone(),
                                    );
                                }
                            }
                        }
                    });

                    // Launch app callback
                    w.on_launch_app(move |idx| {
                        let settings = settings::RavenSettings::load();
                        if (idx as usize) < settings.widgets.apps_container_items.len() {
                            let path = &settings.widgets.apps_container_items[idx as usize].path;
                            let _ = std::process::Command::new("cmd")
                                .args(["/c", "start", "", path])
                                .spawn();
                        }
                    });

                    // Remove app callback
                    let w_weak_rm = w.as_weak();
                    w.on_remove_app(move |idx| {
                        let new_settings = settings::remove_apps_container_item(idx as usize);
                        if let Some(w) = w_weak_rm.upgrade() {
                            let items: Vec<SlintAppShortcut> = new_settings.widgets.apps_container_items.iter().map(|item| {
                                let icon_img = crate::window::get_file_icon(&item.path).unwrap_or_else(|| slint::Image::default());
                                let has_icon = icon_img.size().width > 0 && icon_img.size().height > 0;
                                SlintAppShortcut {
                                    name: slint::SharedString::from(item.name.as_str()),
                                    path: slint::SharedString::from(item.path.as_str()),
                                    icon: icon_img,
                                    has_icon,
                                }
                            }).collect();
                            w.set_app_items(std::rc::Rc::new(slint::VecModel::from(items)).into());
                        }
                    });

                    // Populate initial items
                    let items: Vec<SlintAppShortcut> = settings.widgets.apps_container_items.iter().map(|item| {
                        let icon_img = crate::window::get_file_icon(&item.path).unwrap_or_else(|| slint::Image::default());
                        let has_icon = icon_img.size().width > 0 && icon_img.size().height > 0;
                        SlintAppShortcut {
                            name: slint::SharedString::from(item.name.as_str()),
                            path: slint::SharedString::from(item.path.as_str()),
                            icon: icon_img,
                            has_icon,
                        }
                    }).collect();
                    w.set_app_items(std::rc::Rc::new(slint::VecModel::from(items)).into());

                    w.show().unwrap();
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);

                    let click_through = settings.widgets.click_through;
                    let ac_x = settings.widgets.apps_container_pos_x as i32;
                    let ac_y = settings.widgets.apps_container_pos_y as i32;
                    let w_weak_show = w.as_weak();

                    slint::Timer::single_shot(
                        std::time::Duration::from_millis(50),
                        move || {
                            if let Some(w) = w_weak_show.upgrade() {
                                if let Some(hwnd) = get_window_hwnd(w.window()) {
                                    unsafe {
                                        println!("[WIDGET-DEBUG] Apps Container Widget HWND successfully resolved: {:?}", hwnd);
                                        widgets::setup_widget_window(hwnd, click_through, false);
                                        crate::window::register_apps_container_drop_target(hwnd);
                                        widgets::position_widget_window_from_left(hwnd, ac_x, ac_y, 320, 190);
                                    }
                                }
                            }
                        }
                    );

                    *apps_guard = Some(w);
                } else if let Some(w) = apps_guard.as_ref() {
                    w.set_is_locked(settings.widgets.locked);
                    w.set_bg_opacity(settings.widgets.opacity);
                    w.set_border_radius_val(settings.widgets.stats_border_radius as i32);
                    
                    // Periodically refresh items
                    let items: Vec<SlintAppShortcut> = settings.widgets.apps_container_items.iter().map(|item| {
                        let icon_img = crate::window::get_file_icon(&item.path).unwrap_or_else(|| slint::Image::default());
                        let has_icon = icon_img.size().width > 0 && icon_img.size().height > 0;
                        SlintAppShortcut {
                            name: slint::SharedString::from(item.name.as_str()),
                            path: slint::SharedString::from(item.path.as_str()),
                            icon: icon_img,
                            has_icon,
                        }
                    }).collect();
                    w.set_app_items(std::rc::Rc::new(slint::VecModel::from(items)).into());

                    unsafe {
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                widgets::set_widget_click_through(hwnd, settings.widgets.click_through);
                                widgets::position_widget_window_from_left(
                                    hwnd,
                                    settings.widgets.apps_container_pos_x as i32,
                                    settings.widgets.apps_container_pos_y as i32,
                                    320,
                                    190,
                                );
                            }
                        }
                    }
                }
            } else if let Some(w) = apps_guard.take() {
                let _ = w.hide();
            }

            macro_rules! wire_extra_common {
                ($w:expr, $instance_id:expr, $tracker_name:expr) => {{
                    let lifecycle_id = NEXT_LIFECYCLE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let tracker = Rc::new(DropTracker {
                        id: lifecycle_id,
                        name: $tracker_name.to_string(),
                    });
                    let tracker_dummy = tracker.clone();
                    $w.on_dummy_tracker(move || {
                        let _keep_alive = &tracker_dummy;
                    });

                    let w_weak = $w.as_weak();
                    let tracker_drag = tracker.clone();
                    $w.on_drag_window(move || {
                        let _keep_alive = &tracker_drag;
                        if let Some(w) = w_weak.upgrade() {
                            drag_widget_window(w.window());
                        }
                    });

                    let w_weak2 = $w.as_weak();
                    let s_ui_weak2 = s_ui_weak.clone();
                    let update_cell_c = update_cell.clone();
                    let stats_w_c = stats_w.clone();
                    let tracker_menu = tracker.clone();
                    let instance_id_str = $instance_id.to_string();
                    $w.on_show_context_menu(move || {
                        let _keep_alive = &tracker_menu;
                        if let Some(w) = w_weak2.upgrade() {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = w.window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                    let update_fn = {
                                        let guard = update_cell_c.borrow();
                                        guard.as_ref().cloned().unwrap()
                                    };
                                    show_native_context_menu(
                                        hwnd,
                                        &instance_id_str,
                                        None,
                                        s_ui_weak2.clone(),
                                        update_fn,
                                        stats_w_c.clone(),
                                    );
                                }
                            }
                        }
                    });
                }};
            }

            let desired_instance_ids: HashSet<String> = if settings.widgets.enabled {
                settings
                    .widgets
                    .instances
                    .iter()
                    .filter(|instance| instance.visible && !instance.id.is_empty())
                    .map(|instance| instance.id.clone())
                    .collect()
            } else {
                HashSet::new()
            };

            {
                let mut widgets_guard = instance_w.borrow_mut();
                widgets_guard.retain(|id, widget| {
                    if desired_instance_ids.contains(id) {
                        true
                    } else {
                        println!("[LIFECYCLE-LOG] remove/hide: removing extra widget instance (ID = {}) from manager", id);
                        let _ = widget.hide();
                        false
                    }
                });

                if settings.widgets.enabled {
                    for instance in settings.widgets.instances.iter().filter(|instance| instance.visible && !instance.id.is_empty()) {
                        let kind = if instance.widget_type.trim().is_empty() {
                            "widget"
                        } else {
                            instance.widget_type.as_str()
                        };

                        if let Some(w) = widgets_guard.get(&instance.id) {
                            sync_extra_widget_window(w, &settings, instance, &focus_runtime);
                            position_extra_widget_window(w, &settings, instance);
                        } else {
                            let new_window = match kind {
                                "year_progress" => {
                                    let w = YearProgressWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Year Progress Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Year Progress Widget Copy");
                                    Some(ExtraWidgetWindow::Year(w))
                                }
                                "day_progress" => {
                                    let w = DayProgressWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Day Progress Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Day Progress Widget Copy");
                                    Some(ExtraWidgetWindow::Day(w))
                                }
                                "month_progress" => {
                                    let w = MonthProgressWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Month Progress Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Month Progress Widget Copy");
                                    Some(ExtraWidgetWindow::Month(w))
                                }
                                "media" => {
                                    let w = MediaWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Media Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Media Widget Copy");
                                    let s = services_for_lifecycle.clone();
                                    w.on_skip_backward(move || s.media.seek(false));
                                    let s = services_for_lifecycle.clone();
                                    w.on_prev_track(move || s.media.previous());
                                    let s = services_for_lifecycle.clone();
                                    w.on_toggle_play(move || s.media.play_pause());
                                    let s = services_for_lifecycle.clone();
                                    w.on_next_track(move || s.media.next());
                                    let s = services_for_lifecycle.clone();
                                    w.on_skip_forward(move || s.media.seek(true));
                                    Some(ExtraWidgetWindow::Media(w))
                                }
                                "notes" => {
                                    let w = NotesWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Notes Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Notes Widget Copy");
                                    let update_cell_notes = update_cell.clone();
                                    let instance_id = instance.id.clone();
                                    w.on_notes_changed(move |text| {
                                        settings::set_widget_instance_data_value(
                                            &instance_id,
                                            "notes_text",
                                            serde_json::json!(text.as_str()),
                                        );
                                        if let Some(update_fn) = update_cell_notes.borrow().as_ref().cloned() {
                                            update_fn();
                                        }
                                    });
                                    Some(ExtraWidgetWindow::Notes(w))
                                }
                                "todo" => {
                                    let w = TodoWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Todo Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Todo Widget Copy");
                                    let update_cell_add = update_cell.clone();
                                    let instance_id_add = instance.id.clone();
                                    w.on_add_task(move |text| {
                                        settings::instance_todo_add_item(&instance_id_add, text.as_str());
                                        if let Some(update_fn) = update_cell_add.borrow().as_ref().cloned() { update_fn(); }
                                    });
                                    let update_cell_toggle = update_cell.clone();
                                    let instance_id_toggle = instance.id.clone();
                                    w.on_toggle_task(move |id| {
                                        settings::instance_todo_toggle_item(&instance_id_toggle, id);
                                        if let Some(update_fn) = update_cell_toggle.borrow().as_ref().cloned() { update_fn(); }
                                    });
                                    let update_cell_delete = update_cell.clone();
                                    let instance_id_delete = instance.id.clone();
                                    w.on_delete_task(move |id| {
                                        settings::instance_todo_delete_item(&instance_id_delete, id);
                                        if let Some(update_fn) = update_cell_delete.borrow().as_ref().cloned() { update_fn(); }
                                    });
                                    let update_cell_move = update_cell.clone();
                                    let instance_id_move = instance.id.clone();
                                    w.on_move_task(move |id, is_up| {
                                        settings::instance_todo_move_item(&instance_id_move, id, is_up);
                                        if let Some(update_fn) = update_cell_move.borrow().as_ref().cloned() { update_fn(); }
                                    });
                                    Some(ExtraWidgetWindow::Todo(w))
                                }
                                "quotes" => {
                                    let w = QuotesWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Quotes Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Quotes Widget Copy");
                                    let w_weak_refresh = w.as_weak();
                                    w.on_next_quote(move || {
                                        if let Some(w) = w_weak_refresh.upgrade() {
                                            let settings = settings::RavenSettings::load();
                                            let mut all_quotes: Vec<(String, String)> = DEFAULT_QUOTES.iter()
                                                .map(|(q, a)| (q.to_string(), a.to_string()))
                                                .collect();
                                            for custom in &settings.widgets.quotes_custom_quotes {
                                                let parts: Vec<&str> = custom.split('|').collect();
                                                if parts.len() == 2 {
                                                    all_quotes.push((parts[0].to_string(), parts[1].to_string()));
                                                } else if parts.len() == 1 {
                                                    all_quotes.push((parts[0].to_string(), "Unknown".to_string()));
                                                }
                                            }
                                            use rand::seq::SliceRandom;
                                            if let Some((quote, author)) = all_quotes.choose(&mut rand::thread_rng()) {
                                                w.set_quote_text(quote.clone().into());
                                                w.set_quote_author(author.clone().into());
                                            }
                                        }
                                    });
                                    Some(ExtraWidgetWindow::Quotes(w))
                                }
                                "picture" => {
                                    let w = PictureWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Picture Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Picture Widget Copy");
                                    let w_weak_pic = w.as_weak();
                                    let s_ui_weak_pic = s_ui_weak.clone();
                                    let instance_id_pic = instance.id.clone();
                                    w.on_select_picture(move || {
                                        let w_weak_c = w_weak_pic.clone();
                                        let s_ui_c = s_ui_weak_pic.clone();
                                        let instance_id = instance_id_pic.clone();
                                        std::thread::spawn(move || {
                                            if let Some(path) = select_image_file() {
                                                let _ = slint::invoke_from_event_loop(move || {
                                                    settings::set_widget_instance_data_value(
                                                        &instance_id,
                                                        "picture_path",
                                                        serde_json::json!(path),
                                                    );
                                                    if let Some(w) = w_weak_c.upgrade() {
                                                        w.set_picture_path(path.clone().into());
                                                    }
                                                    if let Some(s_ui) = s_ui_c.upgrade() {
                                                        s_ui.set_picture_selected_path(path.into());
                                                    }
                                                    GLOBAL_UPDATE_LIFECYCLES.with(|cell| {
                                                        if let Some(update_fn) = cell.borrow().as_ref() {
                                                            update_fn();
                                                        }
                                                    });
                                                });
                                            }
                                        });
                                    });
                                    Some(ExtraWidgetWindow::Picture(w))
                                }
                                "video" => {
                                    let w = VideoFrameWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Video Frame Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Video Frame Widget Copy");
                                    let s_ui_weak_vid = s_ui_weak.clone();
                                    let instance_id_vid = instance.id.clone();
                                    w.on_select_video(move || {
                                        let s_ui_c = s_ui_weak_vid.clone();
                                        let instance_id = instance_id_vid.clone();
                                        std::thread::spawn(move || {
                                            if let Some(path) = select_video_file() {
                                                let _ = slint::invoke_from_event_loop(move || {
                                                    settings::set_widget_instance_data_value(
                                                        &instance_id,
                                                        "video_path",
                                                        serde_json::json!(path),
                                                    );
                                                    if let Some(s_ui) = s_ui_c.upgrade() {
                                                        s_ui.set_video_selected_path(path.into());
                                                    }
                                                    GLOBAL_UPDATE_LIFECYCLES.with(|cell| {
                                                        if let Some(update_fn) = cell.borrow().as_ref() {
                                                            update_fn();
                                                        }
                                                    });
                                                });
                                            }
                                        });
                                    });
                                    Some(ExtraWidgetWindow::Video(w))
                                }
                                "battery" | "battery_widget" => {
                                    let w = BatteryPercentageWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Battery Percentage Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Battery Widget Copy");
                                    Some(ExtraWidgetWindow::Battery(w))
                                }
                                "calendar_focus" => {
                                    let w = CalendarFocusWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Calendar Focus Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Calendar Focus Widget Copy");
                                    let toggle_weak = w.as_weak();
                                    let toggle_runtime = focus_runtime.clone();
                                    w.on_toggle_timer(move || {
                                        toggle_runtime.toggle();
                                        if let Some(widget) = toggle_weak.upgrade() {
                                            update_calendar_focus_widget_properties(&widget, &toggle_runtime);
                                        }
                                    });
                                    let menu_weak = w.as_weak();
                                    let runtime = focus_runtime.clone();
                                    let s_ui = s_ui_weak.clone();
                                    let update_cell_menu = update_cell.clone();
                                    let stats_widgets = stats_w.clone();
                                    w.on_configure_timer(move || {
                                        if let Some(widget) = menu_weak.upgrade() {
                                            use raw_window_handle::HasWindowHandle;
                                            if let Ok(handle) = widget.window().window_handle().window_handle() {
                                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                                                    if let Some(update_fn) = update_cell_menu.borrow().as_ref().cloned() {
                                                        show_focus_timer_context_menu(
                                                            hwnd,
                                                            widget.as_weak(),
                                                            runtime.clone(),
                                                            s_ui.clone(),
                                                            update_fn,
                                                            stats_widgets.clone(),
                                                            true,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    });
                                    Some(ExtraWidgetWindow::CalendarFocus(w))
                                }
                                "apps_container" => {
                                    let w = AppsContainerWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Apps Container Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Apps Container Widget Copy");
                                    let instance_id_launch = instance.id.clone();
                                    w.on_launch_app(move |idx| {
                                        let settings = settings::RavenSettings::load();
                                        if let Some(instance) = settings.widgets.instances.iter().find(|item| item.id == instance_id_launch) {
                                            let items = instance_data_app_items(&settings, instance);
                                            if let Some(item) = items.get(idx as usize) {
                                                let _ = std::process::Command::new("cmd")
                                                    .args(["/c", "start", "", item.path.as_str()])
                                                    .spawn();
                                            }
                                        }
                                    });
                                    let update_cell_remove = update_cell.clone();
                                    let instance_id_remove = instance.id.clone();
                                    w.on_remove_app(move |idx| {
                                        settings::remove_instance_apps_container_item(&instance_id_remove, idx as usize);
                                        if let Some(update_fn) = update_cell_remove.borrow().as_ref().cloned() {
                                            update_fn();
                                        }
                                    });
                                    Some(ExtraWidgetWindow::Apps(w))
                                }
                                "focus_score" => {
                                    let w = FocusScoreWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Focus Score Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Focus Score Widget Copy");
                                    Some(ExtraWidgetWindow::FocusScore(w))
                                }
                                "streak" => {
                                    let w = StreakWidgetWindow::new().unwrap();
                                    w.set_instance_title(format!("Raven Calendar Widget {}", instance.id).into());
                                    wire_extra_common!(w, instance.id.as_str(), "Calendar Widget Copy");
                                    let update_cell_rename = update_cell.clone();
                                    let instance_id = instance.id.clone();
                                    w.on_rename_streak(move |new_name| {
                                        settings::set_widget_instance_data_value(
                                            &instance_id,
                                            "streak_name",
                                            serde_json::json!(new_name.as_str()),
                                        );
                                        if let Some(update_fn) = update_cell_rename.borrow().as_ref().cloned() {
                                            update_fn();
                                        }
                                    });
                                    Some(ExtraWidgetWindow::Streak(w))
                                }
                                _ => None,
                            };

                            if let Some(w) = new_window {
                                sync_extra_widget_window(&w, &settings, instance, &focus_runtime);
                                w.hide().ok();
                                match &w {
                                    ExtraWidgetWindow::Year(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Day(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Month(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Media(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Notes(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Todo(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Quotes(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Picture(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Video(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Battery(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::CalendarFocus(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Apps(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::FocusScore(inner) => inner.show().unwrap(),
                                    ExtraWidgetWindow::Streak(inner) => inner.show().unwrap(),
                                }
                                let instance_for_timer = instance.clone();
                                let settings_for_timer = settings.clone();
                                let id_for_timer = instance.id.clone();
                                let instance_w_for_timer = instance_w.clone();
                                fn resolve_extra_hwnd(
                                    instance_w: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, ExtraWidgetWindow>>>,
                                    id: String,
                                    settings: settings::RavenSettings,
                                    instance: settings::WidgetInstanceSettings,
                                ) {
                                    let instance_w_c = instance_w.clone();
                                    let id_c = id.clone();
                                    let settings_c = settings.clone();
                                    let instance_c = instance.clone();
                                    slint::Timer::single_shot(std::time::Duration::from_millis(50), move || {
                                        let resolved = {
                                            if let Some(widget) = instance_w.borrow().get(&id) {
                                                if let Some(hwnd) = widget.hwnd() {
                                                    println!("[WIDGET-DEBUG] resolve_extra_hwnd: resolved HWND {:?} for extra widget ID '{}', placing at x={}, y={}, size={}x{}", 
                                                             hwnd, id, instance.x, instance.y, instance.width, instance.height);
                                                    position_extra_widget_window(widget, &settings, &instance);
                                                    true
                                                } else {
                                                    false
                                                }
                                            } else {
                                                println!("[WIDGET-DEBUG] resolve_extra_hwnd: widget ID '{}' not found in manager", id);
                                                true
                                            }
                                        };
                                        if !resolved {
                                            resolve_extra_hwnd(instance_w_c, id_c, settings_c, instance_c);
                                        }
                                    });
                                }
                                resolve_extra_hwnd(instance_w_for_timer, id_for_timer, settings_for_timer, instance_for_timer);
                                widgets_guard.insert(instance.id.clone(), w);
                            }
                        }
                    }
                }
            }
        }
    };
    let update_widget_lifecycles = std::rc::Rc::new(update_widget_lifecycles);
    *update_widget_lifecycles_cell.borrow_mut() = Some(update_widget_lifecycles.clone() as std::rc::Rc<dyn Fn()>);
    GLOBAL_UPDATE_LIFECYCLES.with(|cell| {
        *cell.borrow_mut() = Some(update_widget_lifecycles.clone() as std::rc::Rc<dyn Fn()>);
    });

    {
        let ui_weak = ui.as_weak();
        let update_widget_lifecycles_save = update_widget_lifecycles.clone();
        ui.on_save_focus_goal_preset(move |goal| {
            let updated_settings = settings::add_focus_goal_preset(goal.as_str());
            if let Some(ui) = ui_weak.upgrade() {
                sync_focus_data_to_ui(&ui, &updated_settings);
            }
            update_widget_lifecycles_save();
        });
    }

    let _ = crate::widgets::APPS_CONTAINER_DROP_CALLBACK.set(Box::new(move |paths| {
        for path in paths {
            settings::add_apps_container_item(&path);
        }
        let _ = slint::invoke_from_event_loop(move || {
            GLOBAL_UPDATE_LIFECYCLES.with(|cell| {
                if let Some(f) = cell.borrow().as_ref() {
                    f();
                }
            });
        });
    }));
    
    // Initial Spawn of Active Widgets
    update_widget_lifecycles();


    // Initialize UI state with loaded settings
    settings_ui.set_app_version_string(format!("v{}", env!("CARGO_PKG_VERSION")).into());

    // Spawn background thread to check for updates periodically (every 4 hours)
    let settings_ui_weak_period = settings_ui.as_weak();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        loop {
            println!("[UPDATER] Running periodic update check...");
            match fetch_latest_version() {
                Ok(manifest) => {
                    let current_version = env!("CARGO_PKG_VERSION");
                    let is_newer = is_newer_version(current_version, &manifest.version);
                    
                    if is_newer {
                        println!("[UPDATER] New update found: v{}", manifest.version);
                        let ui_clone = settings_ui_weak_period.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_clone.upgrade() {
                                ui.set_update_available(true);
                                ui.set_update_url(manifest.url.into());
                                ui.set_about_update_status(format!("New version available: v{}", manifest.version).into());
                            }
                        });
                    } else {
                        println!("[UPDATER] App is up to date (v{}).", current_version);
                    }
                }
                Err(err) => {
                    println!("[UPDATER-ERROR] Periodic update check failed: {}", err);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(4 * 3600));
        }
    });

    settings_ui.set_auto_hide(settings.appearance.auto_hide);
    settings_ui.set_notch_color(settings.appearance.notch_color.clone().into());
    settings_ui.set_accent_color(settings.appearance.accent_color.clone().into());
    settings_ui.set_shape(settings.appearance.shape.clone().into());
    settings_ui.set_idle_pill_mode(settings.appearance.idle_pill_mode.clone().into());
    settings_ui.set_idle_custom_name(settings.appearance.idle_custom_name.clone().into());
    settings_ui.set_hover_enabled(settings.hover.enabled);
    settings_ui.set_idle_width(settings.appearance.idle_width as i32);
    settings_ui.set_idle_height(settings.appearance.idle_height as i32);
    settings_ui.set_notch_border_radius(settings.appearance.border_radius as i32);
    settings_ui.set_idle_border_radius(settings.appearance.idle_border_radius as i32);
    settings_ui.set_auto_hide_on_fullscreen(settings.appearance.auto_hide_on_fullscreen);
    settings_ui.set_appearance_opacity(settings.appearance.notch_opacity as f32 / 100.0);
    settings_ui.set_appearance_mode(settings.appearance.appearance_mode.clone().into());
    update_settings_preview_clock(&settings_ui, &settings);
    settings_ui.set_reserve_top_area(settings.advanced.reserve_top_area);
    settings_ui.set_full_width_bar(settings.advanced.full_width_bar);
    settings_ui.set_top_bar_widgets(settings.advanced.top_bar_widgets);
    settings_ui.set_advanced_run_on_startup(settings.advanced.run_on_startup);
    settings_ui.set_capture_enabled(settings.capture.enabled);
    settings_ui.set_include_cursor(settings.capture.include_cursor);
    settings_ui.set_show_recording_indicator(settings.capture.show_recording_indicator);
    settings_ui.set_mic_enabled(settings.capture.mic_enabled);
    settings_ui.set_system_audio_enabled(settings.capture.system_audio_enabled);

    settings_ui.set_tab_home(settings.tabs.home);
    settings_ui.set_tab_media(settings.tabs.media);
    settings_ui.set_tab_calendar(settings.tabs.calendar);
    settings_ui.set_tab_clock(settings.tabs.clock);
    settings_ui.set_tab_drop(settings.tabs.drop);
    settings_ui.set_tab_capture(settings.tabs.capture);
    settings_ui.set_tab_notifications(settings.tabs.notifications);
    settings_ui.set_tab_stats(settings.tabs.stats);
    settings_ui.set_tab_caffeine(settings.tabs.caffeine);
    settings_ui.set_tab_settings(settings.tabs.settings);
    settings_ui.set_tab_battery(settings.tabs.battery);

    settings_ui.set_media_show_waveform(settings.media.show_waveform);
    settings_ui.set_media_pill_waveform(settings.media.pill_waveform);
    settings_ui.set_media_show_source(settings.media.show_source);
    settings_ui.set_media_auto_expand(settings.media.auto_expand);
    settings_ui.set_media_adaptive_accent(settings.media.adaptive_accent);
    settings_ui.set_media_full_calendar_on_no_media(settings.media.full_calendar_on_no_media);

    settings_ui.set_clock_24h(settings.clock.mode_24h);
    settings_ui.set_clock_show_seconds(settings.clock.show_seconds);
    settings_ui.set_clock_show_ampm(settings.clock.show_ampm);
    settings_ui.set_clock_blink_colon(settings.clock.blink_colon);
    settings_ui.set_clock_show_weekday(settings.clock.show_weekday);
    settings_ui.set_clock_show_date(settings.clock.show_date);
    settings_ui.set_clock_show_utc(settings.clock.show_utc);
    settings_ui.set_clock_show_timer(settings.clock.show_timer);
    settings_ui.set_clock_show_stopwatch(settings.clock.show_stopwatch);

    // Sounds
    settings_ui.set_sounds_enabled(settings.sounds.enabled);
    settings_ui.set_sound_timer_complete(settings.sounds.timer_complete);
    settings_ui.set_sound_stopwatch(settings.sounds.stopwatch);
    settings_ui.set_sound_battery_low(settings.sounds.battery_low);
    settings_ui.set_sound_charger_connected(settings.sounds.charger_connected);
    settings_ui.set_sound_charger_disconnected(settings.sounds.charger_disconnected);
    settings_ui.set_sound_capslock_on(settings.sounds.capslock_on);
    settings_ui.set_sound_capslock_off(settings.sounds.capslock_off);
    settings_ui.set_sound_unlock(settings.sounds.unlock);
    settings_ui.set_sound_timer_complete_custom_path(settings.sounds.custom_timer_complete_path.clone().into());
    settings_ui.set_sound_stopwatch_custom_path(settings.sounds.custom_stopwatch_path.clone().into());
    settings_ui.set_sound_battery_low_custom_path(settings.sounds.custom_battery_low_path.clone().into());
    settings_ui.set_sound_charger_connected_custom_path(settings.sounds.custom_charger_connected_path.clone().into());
    settings_ui.set_sound_charger_disconnected_custom_path(settings.sounds.custom_charger_disconnected_path.clone().into());
    settings_ui.set_sound_capslock_on_custom_path(settings.sounds.custom_capslock_on_path.clone().into());
    settings_ui.set_sound_capslock_off_custom_path(settings.sounds.custom_capslock_off_path.clone().into());
    settings_ui.set_sound_unlock_custom_path(settings.sounds.custom_unlock_path.clone().into());

    // Live Calendar
    settings_ui.set_cal_width(settings.cal.width as i32);
    settings_ui.set_cal_height(settings.cal.height as i32);
    settings_ui.set_media_calendar_url(settings.media.calendar_url.clone().into());
    {
        let snap = services.snapshot();
        let selected_ids = settings.media.google_calendar_ids.clone();
        let slint_cals = map_google_calendars(&snap.calendar.google_calendars, &selected_ids);
        let selected_count = if selected_ids.is_empty() && snap.calendar.google_connected {
            1
        } else {
            selected_ids.len()
        };
        settings_ui.set_google_calendars(std::rc::Rc::new(slint::VecModel::from(slint_cals)).into());
        settings_ui.set_google_selected_calendars_count(selected_count as i32);
    }

    // Drop Shelf Settings
    settings_ui.set_drop_enabled(settings.drop.enabled);
    settings_ui.set_drop_auto_expand(settings.drop.auto_expand);
    settings_ui.set_drop_open_after_drop(settings.drop.open_after_drop);
    settings_ui.set_drop_keep_max(settings.drop.keep_max as i32);
    settings_ui.set_drop_default_provider(settings.drop.default_provider.clone().into());

    // Keyboard Shortcuts
    settings_ui.set_shortcut_toggle_raven(settings.shortcuts.toggle_raven.clone().into());
    settings_ui.set_shortcut_tab_home(settings.shortcuts.tab_home.clone().into());
    settings_ui.set_shortcut_tab_media(settings.shortcuts.tab_media.clone().into());
    settings_ui.set_shortcut_tab_calendar(settings.shortcuts.tab_calendar.clone().into());
    settings_ui.set_shortcut_tab_clock(settings.shortcuts.tab_clock.clone().into());
    settings_ui.set_shortcut_tab_drop(settings.shortcuts.tab_drop.clone().into());
    settings_ui.set_shortcut_tab_capture(settings.shortcuts.tab_capture.clone().into());
    settings_ui.set_shortcut_tab_stats(settings.shortcuts.tab_stats.clone().into());
    settings_ui.set_shortcut_media_play(settings.shortcuts.media_play.clone().into());
    settings_ui.set_shortcut_media_next(settings.shortcuts.media_next.clone().into());
    settings_ui.set_shortcut_media_prev(settings.shortcuts.media_prev.clone().into());
    settings_ui.set_shortcut_toggle_freeze(settings.shortcuts.toggle_freeze.clone().into());
    settings_ui.set_shortcut_quick_screenshot(settings.shortcuts.quick_screenshot.clone().into());
    settings_ui.set_shortcut_quick_record_toggle(settings.shortcuts.quick_record_toggle.clone().into());
    settings_ui.set_shortcut_open_settings(settings.shortcuts.open_settings.clone().into());
    settings_ui.set_shortcut_restart_raven(settings.shortcuts.restart_raven.clone().into());
    settings_ui.set_shortcut_quit_raven(settings.shortcuts.quit_raven.clone().into());

    // Intelligence & Alerts
    settings_ui.set_alert_enabled(settings.raven_alert.enabled);
    settings_ui.set_alert_monitor_charger_in(settings.raven_alert.monitor_charger_in);
    settings_ui.set_alert_monitor_charger_out(settings.raven_alert.monitor_charger_out);
    settings_ui.set_alert_monitor_low_battery(settings.raven_alert.monitor_low_battery);
    settings_ui.set_alert_monitor_unlock(settings.raven_alert.monitor_unlock);
    settings_ui.set_alert_monitor_bluetooth(settings.raven_alert.monitor_bluetooth);
    settings_ui.set_alert_monitor_keys(settings.raven_alert.monitor_keys);
    settings_ui.set_alert_volume_hud(settings.raven_alert.monitor_volume_hud);
    settings_ui.set_alert_brightness_hud(settings.raven_alert.monitor_brightness_hud);
    settings_ui.set_alert_monitor_camera(settings.raven_alert.monitor_camera);
    settings_ui.set_alert_monitor_caffeine(settings.raven_alert.monitor_caffeine);
    settings_ui.set_intelligence_always_on_charging(settings.intelligence.always_on_charging);
    settings_ui.set_intelligence_always_on_low_battery(settings.intelligence.always_on_low_battery);
    settings_ui.set_intelligence_always_on_charging_mode(settings.intelligence.always_on_charging_mode.clone().into());

    // Advanced panel (new granular fields)
    settings_ui.set_advanced_reserve_top_area(settings.advanced.reserve_top_area);
    settings_ui.set_advanced_full_width_bar(settings.advanced.full_width_bar);
    settings_ui.set_advanced_top_bar_widgets(settings.advanced.top_bar_widgets);
    settings_ui.set_advanced_top_bar_widget_raven(settings.advanced.top_bar_widget_raven);
    settings_ui.set_advanced_top_bar_widget_media(settings.advanced.top_bar_widget_media);
    settings_ui.set_advanced_top_bar_widget_apps(settings.advanced.top_bar_widget_apps);
    settings_ui.set_advanced_top_bar_widget_stats(settings.advanced.top_bar_widget_stats);
    settings_ui.set_advanced_top_bar_widget_clipboard(settings.advanced.top_bar_widget_clipboard);
    settings_ui.set_advanced_top_bar_widget_volume(settings.advanced.top_bar_widget_volume);
    settings_ui.set_advanced_top_bar_widget_wifi(settings.advanced.top_bar_widget_wifi);
    settings_ui.set_advanced_top_bar_widget_battery(settings.advanced.top_bar_widget_battery);
    settings_ui.set_advanced_top_bar_widget_timer(settings.advanced.top_bar_widget_timer);
    settings_ui.set_advanced_top_bar_widget_calendar(settings.advanced.top_bar_widget_calendar);
    settings_ui.set_advanced_reserve_top_height(settings.advanced.reserve_top_height as i32);
    settings_ui.set_advanced_bezel_opacity(settings.advanced.bezel_opacity as f32 / 100.0);
    settings_ui.set_advanced_run_on_startup(settings.advanced.run_on_startup);
    settings_ui.set_appearance_mode(settings.appearance.appearance_mode.clone().into());
    settings_ui.set_appearance_opacity(settings.appearance.notch_opacity / 100.0);

    // Initialize Desktop Widget Engine Settings
    settings_ui.set_widgets_enabled(settings.widgets.enabled);
    settings_ui.set_widgets_click_through(settings.widgets.click_through);
    settings_ui.set_widgets_locked(settings.widgets.locked);
    settings_ui.set_widgets_opacity(settings.widgets.opacity);
    settings_ui.set_widgets_clock_enabled(settings.widgets.clock_enabled);
    settings_ui.set_widgets_year_progress_enabled(settings.widgets.year_journey_enabled);
    settings_ui.set_widgets_day_progress_enabled(settings.widgets.day_journey_enabled);
    settings_ui.set_widgets_month_progress_enabled(settings.widgets.month_journey_enabled);
    settings_ui.set_widgets_media_enabled(settings.widgets.media_enabled);
    settings_ui.set_widgets_stats_enabled(settings.widgets.stats_enabled);
    settings_ui.set_widgets_actions_enabled(settings.widgets.actions_enabled);
    settings_ui.set_widgets_notes_enabled(settings.widgets.notes_enabled);
    settings_ui.set_widgets_todo_enabled(settings.widgets.todo_enabled);
    settings_ui.set_widgets_quotes_enabled(settings.widgets.quotes_enabled);
    settings_ui.set_widgets_picture_enabled(settings.widgets.picture_enabled);
    settings_ui.set_widgets_video_enabled(settings.widgets.video_enabled);
    settings_ui.set_widgets_battery_enabled(settings.widgets.battery_widget_enabled);
    settings_ui.set_widgets_calendar_focus_enabled(settings.widgets.calendar_focus_enabled);
    settings_ui.set_widgets_apps_container_enabled(settings.widgets.apps_container_enabled);
    settings_ui.set_widgets_focus_score_enabled(settings.widgets.focus_score_widget_enabled);
    settings_ui.set_widgets_streak_enabled(settings.widgets.streak_widget_enabled);
    settings_ui.set_focus_timer_minutes(settings.widgets.focus_timer_minutes.round() as i32);
    settings_ui.set_todo_hide_completed(settings.widgets.todo_hide_completed);
    settings_ui.set_todo_accent_color_str(settings.widgets.todo_accent_color.clone().into());
    settings_ui.set_quotes_cycle_enabled(settings.widgets.quotes_cycle_enabled);
    settings_ui.set_quotes_interval_mins(settings.widgets.quotes_change_interval_mins);
    settings_ui.set_picture_selected_path(settings.widgets.picture_path.clone().into());
    settings_ui.set_video_selected_path(settings.widgets.video_path.clone().into());
    settings_ui.set_widgets_clock_count(settings.widgets.clock_count as i32);
    settings_ui.set_selected_clock_index(0);
    sync_selected_clock_settings_to_ui(&settings_ui, &settings, 0);
    sync_all_clocks_to_ui(&settings_ui, &settings);
    reconcile_widget_order(&settings_ui);

    let settings_ui_weak_clock_changed = settings_ui.as_weak();
    settings_ui.on_selected_clock_changed(move |idx| {
        if let Some(s_ui) = settings_ui_weak_clock_changed.upgrade() {
            let settings = settings::RavenSettings::load();
            sync_selected_clock_settings_to_ui(&s_ui, &settings, idx as usize);
            sync_all_clocks_to_ui(&s_ui, &settings);
        }
    });

    // Handle per-index widget removal: close targeted window smoothly and shift remaining ones
    let settings_ui_weak_remove = settings_ui.as_weak();
    let stats_widget_remove = stats_widget.clone();
    let update_widget_lifecycles_remove = update_widget_lifecycles.clone();
    settings_ui.on_remove_clock_at(move |remove_idx| {
        let remove_idx = remove_idx as usize;
        println!("[WIDGET] remove_clock_at({})", remove_idx);

        // Step 1: Smoothly hide and remove the specific window at remove_idx
        {
            let mut stats_guard = stats_widget_remove.borrow_mut();
            if remove_idx < stats_guard.len() {
                println!("[LIFECYCLE-LOG] remove/hide: removing Clock Widget at index {} via remove_clock_at", remove_idx);
                let w = stats_guard.remove(remove_idx);
                let _ = w.hide();
            }
            if stats_guard.is_empty() {
                crate::window::STATS_WIDGET_HWND.store(0, std::sync::atomic::Ordering::SeqCst);
            } else if remove_idx == 0 {
                use raw_window_handle::HasWindowHandle;
                if let Ok(handle) = stats_guard[0].window().window_handle().window_handle() {
                    if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                        let hwnd = win32.hwnd.get() as isize;
                        crate::window::STATS_WIDGET_HWND.store(hwnd, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
        }

        // Step 2: Remove instance at remove_idx and decrement count atomically in settings
        let new_settings = settings::remove_clock_instance(remove_idx);
        let new_count = new_settings.widgets.clock_count as i32;

        // Step 3: Sync UI state
        if let Some(s_ui) = settings_ui_weak_remove.upgrade() {
            s_ui.set_widgets_clock_count(new_count);
            if new_count == 0 {
                s_ui.set_widgets_stats_enabled(false);
                s_ui.set_selected_clock_index(0);
                sync_selected_clock_settings_to_ui(&s_ui, &new_settings, 0);
            } else {
                sync_all_clocks_to_ui(&s_ui, &new_settings);
                let sel = s_ui.get_selected_clock_index().min(new_count - 1).max(0);
                s_ui.set_selected_clock_index(sel);
                sync_selected_clock_settings_to_ui(&s_ui, &new_settings, sel as usize);
            }
            reconcile_widget_order(&s_ui);
        }

        // Step 4: Recreate/update remaining windows via the lifecycle manager
        update_widget_lifecycles_remove();
    });

    // Handle remove_active_widget callback from the chip X button
    let settings_ui_weak_rmw = settings_ui.as_weak();
    let update_widget_lifecycles_rmw = update_widget_lifecycles.clone();
    let stats_widget_rmw = stats_widget.clone();
    settings_ui.on_remove_active_widget(move |widget_id| {
        let wid = widget_id.to_string();
        println!("[WIDGET] remove_active_widget: {}", wid);
        
        if wid.starts_with("clock_") {
            if let Some(idx_str) = wid.strip_prefix("clock_") {
                if let Ok(remove_idx) = idx_str.parse::<usize>() {
                    // Step 1: Smoothly hide and remove the specific window at remove_idx
                    {
                        let mut stats_guard = stats_widget_rmw.borrow_mut();
                        if remove_idx < stats_guard.len() {
                            println!("[LIFECYCLE-LOG] remove/hide: removing Clock Widget at index {} via remove_active_widget", remove_idx);
                            let w = stats_guard.remove(remove_idx);
                            let _ = w.hide();
                        }
                        if stats_guard.is_empty() {
                            crate::window::STATS_WIDGET_HWND.store(0, std::sync::atomic::Ordering::SeqCst);
                        } else if remove_idx == 0 {
                            use raw_window_handle::HasWindowHandle;
                            if let Ok(handle) = stats_guard[0].window().window_handle().window_handle() {
                                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = win32.hwnd.get() as isize;
                                    crate::window::STATS_WIDGET_HWND.store(hwnd, std::sync::atomic::Ordering::SeqCst);
                                }
                            }
                        }
                    }
                    
                    // Step 2: Remove instance at remove_idx and decrement count atomically in settings
                    let new_settings = settings::remove_clock_instance(remove_idx);
                    let new_count = new_settings.widgets.clock_count as i32;
                    
                    // Step 3: Sync UI state
                    if let Some(s_ui) = settings_ui_weak_rmw.upgrade() {
                        s_ui.set_widgets_clock_count(new_count);
                        if new_count == 0 {
                            s_ui.set_widgets_stats_enabled(false);
                            s_ui.set_selected_clock_index(0);
                            sync_selected_clock_settings_to_ui(&s_ui, &new_settings, 0);
                        } else {
                            sync_all_clocks_to_ui(&s_ui, &new_settings);
                            let sel = s_ui.get_selected_clock_index().min(new_count - 1).max(0);
                            s_ui.set_selected_clock_index(sel);
                            sync_selected_clock_settings_to_ui(&s_ui, &new_settings, sel as usize);
                        }
                        reconcile_widget_order(&s_ui);
                    }
                    update_widget_lifecycles_rmw();
                }
            }
            return;
        }
        
        let (settings_path, ui_key): (&[&str], &str) = match wid.as_str() {
            "year_progress"  => (&["widgets","year_journey_enabled"],       "year_progress"),
            "day_progress"   => (&["widgets","day_journey_enabled"],        "day_progress"),
            "month_progress" => (&["widgets","month_journey_enabled"],      "month_progress"),
            "media"          => (&["widgets","media_enabled"],              "media"),
            "notes"          => (&["widgets","notes_enabled"],              "notes"),
            "todo"           => (&["widgets","todo_enabled"],               "todo"),
            "quotes"         => (&["widgets","quotes_enabled"],             "quotes"),
            "picture"        => (&["widgets","picture_enabled"],            "picture"),
            "video"          => (&["widgets","video_enabled"],              "video"),
            "battery"        => (&["widgets","battery_widget_enabled"],     "battery"),
            "calendar_focus" => (&["widgets","calendar_focus_enabled"],     "calendar_focus"),
            "apps_container" => (&["widgets","apps_container_enabled"],     "apps_container"),
            "focus_score"    => (&["widgets","focus_score_widget_enabled"], "focus_score"),
            "streak"         => (&["widgets","streak_widget_enabled"],      "streak"),
            _ => { return; }
        };
        settings::set_bool(settings_path, false);
        if let Some(s_ui) = settings_ui_weak_rmw.upgrade() {
            match ui_key {
                "year_progress"  => s_ui.set_widgets_year_progress_enabled(false),
                "day_progress"   => s_ui.set_widgets_day_progress_enabled(false),
                "month_progress" => s_ui.set_widgets_month_progress_enabled(false),
                "media"          => s_ui.set_widgets_media_enabled(false),
                "notes"          => s_ui.set_widgets_notes_enabled(false),
                "todo"           => s_ui.set_widgets_todo_enabled(false),
                "quotes"         => s_ui.set_widgets_quotes_enabled(false),
                "picture"        => s_ui.set_widgets_picture_enabled(false),
                "video"          => s_ui.set_widgets_video_enabled(false),
                "battery"        => s_ui.set_widgets_battery_enabled(false),
                "calendar_focus" => s_ui.set_widgets_calendar_focus_enabled(false),
                "apps_container" => s_ui.set_widgets_apps_container_enabled(false),
                "focus_score"    => s_ui.set_widgets_focus_score_enabled(false),
                "streak"         => s_ui.set_widgets_streak_enabled(false),
                _ => {}
            }
            reconcile_widget_order(&s_ui);
        }
        update_widget_lifecycles_rmw();
    });


    // Wire Real-Time Updates from Settings -> Notch
    let ui_weak = ui.as_weak();
    let settings_ui_weak_int = settings_ui.as_weak();
    let ui_motion_state_cloned = ui_motion_state.clone();
    let update_widget_lifecycles_int = update_widget_lifecycles.clone();
    let focus_timer_runtime_int = focus_timer_runtime.clone();
    settings_ui.on_int_setting_changed(move |setting, value| {
        let t_start = std::time::Instant::now();
        println!("[SETTINGS-LOG] on_int_setting_changed start: {} = {}", setting, value);
        if setting.as_str().starts_with("widgets")
            && settings_ui_weak_int
                .upgrade()
                .map(|ui| ui.get_premium_locked())
                .unwrap_or(false)
        {
            println!("[LICENSE] Ignored locked widget int setting: {}", setting);
            return;
        }
        // Live update the Pill
        if let Some(ui) = ui_weak.upgrade() {
            match setting.as_str() {
                "border-radius" => {
                    ui.set_notch_border_radius(value as f32);
                    ui.set_motion_radius(value as f32);
                    let mut motion = ui_motion_state_cloned.borrow_mut();
                    let open_width = motion.open.width;
                    let open_height = motion.open.height;
                    motion.set_open_geometry(open_width, open_height, value as f32);
                },
                "idle-border-radius" => {
                    ui.set_idle_border_radius(value as f32);
                    ui.set_motion_radius(value as f32);
                    let mut motion = ui_motion_state_cloned.borrow_mut();
                    let idle_width = ui.get_idle_width();
                    let idle_height = ui.get_idle_height();
                    motion.set_closed_geometry(idle_width, idle_height, value as f32);
                },
                "idle-width" => {
                    ui.set_idle_width(value as f32);
                    ui.set_motion_width(value as f32);
                    let mut motion = ui_motion_state_cloned.borrow_mut();
                    let idle_height = ui.get_idle_height();
                    let closed_radius = ui.get_idle_border_radius().min(idle_height / 2.0).max(0.0);
                    motion.set_closed_geometry(value as f32, idle_height, closed_radius);
                },
                "idle-height" => {
                    ui.set_idle_height(value as f32);
                    ui.set_motion_height(value as f32);
                    let mut motion = ui_motion_state_cloned.borrow_mut();
                    let idle_width = ui.get_idle_width();
                    let closed_radius = ui.get_idle_border_radius().min(value as f32 / 2.0).max(0.0);
                    motion.set_closed_geometry(idle_width, value as f32, closed_radius);
                },
                "appearance-opacity" => ui.set_appearance_opacity(value as f32 / 100.0),
                "advanced-bezel-opacity" => ui.set_bezel_opacity(value as f32 / 100.0),
                _ => {}
            }
        }
        
        // Persist setting
        match setting.as_str() {
            "border-radius" => {
                settings::set_number(&["appearance", "border_radius"], value as f64);
                if let Some(s_ui) = settings_ui_weak_int.upgrade() {
                    s_ui.set_notch_border_radius(value);
                }
            },
            "idle-border-radius" => {
                settings::set_number(&["appearance", "idle_border_radius"], value as f64);
                if let Some(s_ui) = settings_ui_weak_int.upgrade() {
                    s_ui.set_idle_border_radius(value);
                }
            },
            "idle-width"    => { settings::set_number(&["appearance", "idle_width"], value as f64); },
            "idle-height"   => { settings::set_number(&["appearance", "idle_height"], value as f64); },
            "appearance-opacity" => {
                settings::set_number(&["appearance", "notch_opacity"], value as f64);
                if let Some(s_ui) = settings_ui_weak_int.upgrade() {
                    s_ui.set_appearance_opacity(value as f32 / 100.0);
                }
            },
            "advanced-bezel-opacity" => {
                settings::set_number(&["advanced", "bezel_opacity"], value as f64);
                if let Some(s_ui) = settings_ui_weak_int.upgrade() {
                    s_ui.set_advanced_bezel_opacity(value as f32 / 100.0);
                }
            },
            "notch-opacity" => { settings::set_number(&["appearance", "notch_opacity"], value as f64); },
            "inactive-opacity" => { settings::set_number(&["appearance", "inactive_opacity"], value as f64); },
            "open-delay"    => {
                HOVER_OPEN_DELAY_MS.store(value as u32, Ordering::SeqCst);
                settings::set_number(&["hover", "open_delay"], value as f64);
            },
            "close-delay"   => {
                HOVER_CLOSE_DELAY_MS.store(value as u32, Ordering::SeqCst);
                settings::set_number(&["hover", "close_delay"], value as f64);
            },
            "reserve-top-height" => {
                settings::set_number(&["advanced", "reserve_top_height"], value as f64);
                crate::window::update_appbar_reservation();
            },
            "advanced-reserve-top-height" => {
                settings::set_number(&["advanced", "reserve_top_height"], value as f64);
                crate::window::update_appbar_reservation();
            },
            "cal-width"  => { settings::set_number(&["cal", "width"], value as f64); },
            "cal-height" => { settings::set_number(&["cal", "height"], value as f64); },
            "drop-keep-max" => { settings::set_number(&["drop", "keep_max"], value as f64); },
            "widgets-opacity" => {
                let s_ui = settings_ui_weak_int.upgrade();
                let is_stats = s_ui.as_ref().map(|ui| ui.get_selected_widget_id() == "stats").unwrap_or(false);
                if is_stats {
                    let idx = s_ui.as_ref().map(|ui| ui.get_selected_clock_index() as usize).unwrap_or(0);
                    settings::update_clock_instance_setting(idx, |inst| { inst.opacity = value as f64 / 100.0; });
                    update_widget_lifecycles_int();
                    let settings = settings::RavenSettings::load();
                    if let Some(ui) = s_ui {
                        sync_selected_clock_settings_to_ui(&ui, &settings, idx);
                        sync_all_clocks_to_ui(&ui, &settings);
                    }
                } else {
                    settings::set_number(&["widgets", "opacity"], value as f64 / 100.0);
                    update_widget_lifecycles_int();
                }
            },
            "widgets-stats-border-radius" => {
                let s_ui = settings_ui_weak_int.upgrade();
                let is_stats = s_ui.as_ref().map(|ui| ui.get_selected_widget_id() == "stats").unwrap_or(false);
                if is_stats {
                    let idx = s_ui.as_ref().map(|ui| ui.get_selected_clock_index() as usize).unwrap_or(0);
                    settings::update_clock_instance_setting(idx, |inst| { inst.border_radius = value as f64; });
                    update_widget_lifecycles_int();
                    let settings = settings::RavenSettings::load();
                    if let Some(ui) = s_ui {
                        sync_selected_clock_settings_to_ui(&ui, &settings, idx);
                        sync_all_clocks_to_ui(&ui, &settings);
                    }
                } else {
                    settings::set_number(&["widgets", "stats_border_radius"], value as f64);
                    update_widget_lifecycles_int();
                }
            },
            "widgets-clock-count" => {
                // Persist the count and recreate/destroy window instances
                settings::set_number(&["widgets", "clock_count"], value as f64);
                update_widget_lifecycles_int();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak_int.upgrade() {
                    sync_all_clocks_to_ui(&s_ui, &settings);
                    reconcile_widget_order(&s_ui);
                }
            },
            "quotes-interval-mins" => {
                settings::set_number(&["widgets", "quotes_change_interval_mins"], value as f64);
                update_widget_lifecycles_int();
            },
            "focus-timer-minutes" => {
                let minutes = value.clamp(1, 180);
                settings::set_number(&["widgets", "focus_timer_minutes"], minutes as f64);
                focus_timer_runtime_int.set_minutes(minutes);
                update_widget_lifecycles_int();
            },
            _ => { println!("[Settings] Unknown int setting: {} = {}", setting, value); }
        }
        println!("[SETTINGS-LOG] on_int_setting_changed completed in {:?}", t_start.elapsed());
    });

    // Phase 6: bool settings from tabs — log for now, persist later
    let ui_weak_bool = ui.as_weak();
    let update_widget_lifecycles_bool = update_widget_lifecycles.clone();
    let settings_ui_weak_bool = settings_ui.as_weak();
    settings_ui.on_bool_setting_changed(move |setting, value| {
        let t_start = std::time::Instant::now();
        println!("[SETTINGS-LOG] on_bool_setting_changed start: {} = {}", setting, value);
        if setting.as_str().starts_with("widgets")
            && settings_ui_weak_bool
                .upgrade()
                .map(|ui| ui.get_premium_locked())
                .unwrap_or(false)
        {
            println!("[LICENSE] Ignored locked widget bool setting: {}", setting);
            return;
        }
        match setting.as_str() {
            "auto-hide" => {
                APPEARANCE_AUTO_HIDE.store(value, Ordering::SeqCst);
                settings::set_bool(&["appearance", "auto_hide"], value);
                if value {
                    settings::set_bool(&["advanced", "reserve_top_area"], false);
                    if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                        s_ui.set_reserve_top_area(false);
                        s_ui.set_advanced_reserve_top_area(false);
                    }
                    crate::window::update_appbar_reservation();
                }
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_auto_hide(value);
                    if value {
                        main_ui.set_is_hovered(false);
                    }
                }
                crate::window::update_pill_window_layout();
            },
            "auto-hide-on-fullscreen" => {
                APPEARANCE_AUTO_HIDE_ON_FULLSCREEN.store(value, Ordering::SeqCst);
                settings::set_bool(&["appearance", "auto_hide_on_fullscreen"], value);
                if value {
                    settings::set_bool(&["advanced", "reserve_top_area"], false);
                    if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                        s_ui.set_reserve_top_area(false);
                        s_ui.set_advanced_reserve_top_area(false);
                    }
                    crate::window::update_appbar_reservation();
                }
                crate::window::update_pill_window_layout();
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    if value {
                        main_ui.set_is_hovered(false);
                    }
                }
            },
            "hover-enabled" => {
                HOVER_ENABLED.store(value, Ordering::SeqCst);
                settings::set_bool(&["hover", "enabled"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_hover_enabled(value);
                }
            },
            "reserve-top-area" => {
                settings::set_bool(&["advanced", "reserve_top_area"], value);
                crate::window::update_appbar_reservation();
                crate::window::update_pill_window_layout();
            },
            "full-width-bar" => {
                settings::set_bool(&["advanced", "full_width_bar"], value);
                crate::window::update_pill_window_layout();
            },
            "top-bar-widgets" => {
                settings::set_bool(&["advanced", "top_bar_widgets"], value);
                crate::window::update_pill_window_layout();
            },
            "capture-enabled" => { settings::set_bool(&["capture", "enabled"], value); },
            "include-cursor" => { settings::set_bool(&["capture", "include_cursor"], value); },
            "show-recording-indicator" => { settings::set_bool(&["capture", "show_recording_indicator"], value); },
            "mic-enabled" => { settings::set_bool(&["capture", "mic_enabled"], value); },
            "system-audio-enabled" => { settings::set_bool(&["capture", "system_audio_enabled"], value); },

            // Tabs
            "tab-home" => {
                settings::set_bool(&["tabs", "home"], value);
                window::set_tab_visibility("home".to_string(), value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_home(value);
                }
            },
            "tab-media" => {
                settings::set_bool(&["tabs", "media"], value);
                window::set_tab_visibility("media".to_string(), value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_media(value);
                }
            },
            "tab-calendar" => {
                settings::set_bool(&["tabs", "calendar"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_calendar(value);
                }
            },
            "tab-clock" => {
                settings::set_bool(&["tabs", "clock"], value);
                window::set_tab_visibility("clock".to_string(), value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_clock(value);
                }
            },
            "tab-drop" => {
                settings::set_bool(&["tabs", "drop"], value);
                window::set_tab_visibility("drop".to_string(), value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_drop(value);
                }
            },
            "tab-capture" => { settings::set_bool(&["tabs", "capture"], value); },
            "tab-notifications" => { settings::set_bool(&["tabs", "notifications"], value); },
            "tab-stats" => {
                settings::set_bool(&["tabs", "stats"], value);
                window::set_tab_visibility("stats".to_string(), value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_stats(value);
                }
            },
            "tab-caffeine" => {
                settings::set_bool(&["tabs", "caffeine"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_caffeine(value);
                }
            },
            "tab-settings" => {
                settings::set_bool(&["tabs", "settings"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_settings(value);
                }
            },
            "tab-battery" => {
                settings::set_bool(&["tabs", "battery"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_tab_battery(value);
                }
            },

            // Media
            "media-show-waveform" => {
                settings::set_bool(&["media", "show_waveform"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_media_show_waveform(value);
                }
            },
            "media-pill-waveform" => {
                settings::set_bool(&["media", "pill_waveform"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_show_pill_waveform(value);
                }
            },
            "media-show-source" => {
                settings::set_bool(&["media", "show_source"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_media_show_source(value);
                }
            },
            "media-auto-expand" => {
                settings::set_bool(&["media", "auto_expand"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_media_auto_expand(value);
                }
            },
            "media-adaptive-accent" => {
                settings::set_bool(&["media", "adaptive_accent"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_media_adaptive_accent(value);
                }
            },
            "media-full-calendar-on-no-media" => {
                settings::set_bool(&["media", "full_calendar_on_no_media"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_media_full_calendar_on_no_media(value);
                }
            },

            // Clock
            "clock-24h" => {
                settings::set_bool(&["clock", "mode_24h"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },
            "clock-show-seconds" => {
                settings::set_bool(&["clock", "show_seconds"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },
            "clock-show-ampm" => {
                settings::set_bool(&["clock", "show_ampm"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },
            "clock-blink-colon" => {
                settings::set_bool(&["clock", "blink_colon"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },
            "clock-show-weekday" => {
                settings::set_bool(&["clock", "show_weekday"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },
            "clock-show-date" => {
                settings::set_bool(&["clock", "show_date"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },
            "clock-show-utc" => {
                settings::set_bool(&["clock", "show_utc"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },
            "clock-show-timer" => {
                settings::set_bool(&["clock", "show_timer"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },
            "clock-show-stopwatch" => {
                settings::set_bool(&["clock", "show_stopwatch"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    update_clock_display(&main_ui);
                }
            },

            // Sounds
            "sounds-enabled" => { settings::set_bool(&["sounds", "enabled"], value); },
            "sound-timer-complete" => { settings::set_bool(&["sounds", "timer_complete"], value); },
            "sound-stopwatch" => { settings::set_bool(&["sounds", "stopwatch"], value); },
            "sound-battery-low" => { settings::set_bool(&["sounds", "battery_low"], value); },
            "sound-charger-connected" => { settings::set_bool(&["sounds", "charger_connected"], value); },
            "sound-charger-disconnected" => { settings::set_bool(&["sounds", "charger_disconnected"], value); },
            "sound-capslock-on" => { settings::set_bool(&["sounds", "capslock_on"], value); },
            "sound-capslock-off" => { settings::set_bool(&["sounds", "capslock_off"], value); },
            "sound-unlock" => { settings::set_bool(&["sounds", "unlock"], value); },

            // Intelligence & Alerts
            "alert-enabled"                => {
                settings::set_bool(&["raven_alert", "enabled"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_enabled(value);
                }
            },
            "alert-monitor-charger-in"     => {
                settings::set_bool(&["raven_alert", "monitor_charger_in"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_monitor_charger_in(value);
                }
            },
            "alert-monitor-charger-out"    => {
                settings::set_bool(&["raven_alert", "monitor_charger_out"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_monitor_charger_out(value);
                }
            },
            "alert-monitor-low-battery"    => {
                settings::set_bool(&["raven_alert", "monitor_low_battery"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_monitor_low_battery(value);
                }
            },
            "alert-monitor-unlock"         => {
                settings::set_bool(&["raven_alert", "monitor_unlock"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_monitor_unlock(value);
                }
            },
            "alert-monitor-bluetooth"      => {
                settings::set_bool(&["raven_alert", "monitor_bluetooth"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_monitor_bluetooth(value);
                }
            },
            "alert-monitor-keys"           => {
                settings::set_bool(&["raven_alert", "monitor_keys"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_monitor_keys(value);
                }
            },
            "alert-volume-hud"             => {
                settings::set_bool(&["raven_alert", "monitor_volume_hud"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_volume_hud(value);
                }
            },
            "alert-brightness-hud"         => {
                settings::set_bool(&["raven_alert", "monitor_brightness_hud"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_brightness_hud(value);
                }
            },
            "alert-monitor-camera"         => {
                settings::set_bool(&["raven_alert", "monitor_camera"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_monitor_camera(value);
                }
            },
            "alert-monitor-caffeine"       => {
                settings::set_bool(&["raven_alert", "monitor_caffeine"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_alert_monitor_caffeine(value);
                }
            },
            "intelligence-always-on-charging"     => {
                settings::set_bool(&["intelligence", "always_on_charging"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_always_on_charging(value);
                }
            },
            "intelligence-always-on-low-battery"  => {
                settings::set_bool(&["intelligence", "always_on_low_battery"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    main_ui.set_always_on_low_battery(value);
                }
            },

            // Advanced (new granular names from redesigned panel)
            "advanced-reserve-top-area"  => {
                settings::set_bool(&["advanced", "reserve_top_area"], value);
                crate::window::update_appbar_reservation();
                crate::window::update_pill_window_layout();
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    let settings = settings::RavenSettings::load();
                    let (logical_screen_w, _) = crate::window::get_primary_screen_logical_width();
                    main_ui.set_screen_width(logical_screen_w);
                    main_ui.set_full_width_bar(settings.advanced.full_width_bar);
                    main_ui.set_top_bar_widgets(settings.advanced.full_width_bar && settings.advanced.top_bar_widgets);
                }
            },
            "advanced-full-width-bar"    => {
                settings::set_bool(&["advanced", "full_width_bar"], value);
                crate::window::update_appbar_reservation();
                crate::window::update_pill_window_layout();
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    let settings = settings::RavenSettings::load();
                    let (logical_screen_w, _) = crate::window::get_primary_screen_logical_width();
                    main_ui.set_screen_width(logical_screen_w);
                    main_ui.set_full_width_bar(settings.advanced.full_width_bar);
                    main_ui.set_top_bar_widgets(settings.advanced.full_width_bar && settings.advanced.top_bar_widgets);
                }
            },
            "advanced-top-bar-widgets"   => {
                settings::set_bool(&["advanced", "top_bar_widgets"], value);
                crate::window::update_pill_window_layout();
                if let Some(main_ui) = ui_weak_bool.upgrade() {
                    let settings = settings::RavenSettings::load();
                    main_ui.set_top_bar_widgets(settings.advanced.full_width_bar && settings.advanced.top_bar_widgets);
                }
            },
            "advanced-run-on-startup"    => {
                settings::set_bool(&["advanced", "run_on_startup"], value);
                if let Err(e) = crate::window::set_run_on_startup(value) {
                    eprintln!("[STARTUP-ERROR] Failed to set run on startup: {:?}", e);
                }
            },
            "advanced-top-bar-widget-raven" => {
                settings::set_bool(&["advanced", "top_bar_widget_raven"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_raven(value); }
            },
            "advanced-top-bar-widget-media" => {
                settings::set_bool(&["advanced", "top_bar_widget_media"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_media(value); }
            },
            "advanced-top-bar-widget-apps" => {
                settings::set_bool(&["advanced", "top_bar_widget_apps"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_apps(value); }
            },
            "advanced-top-bar-widget-stats" => {
                settings::set_bool(&["advanced", "top_bar_widget_stats"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_stats(value); }
            },
            "advanced-top-bar-widget-clipboard" => {
                settings::set_bool(&["advanced", "top_bar_widget_clipboard"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_clipboard(value); }
            },
            "advanced-top-bar-widget-volume" => {
                settings::set_bool(&["advanced", "top_bar_widget_volume"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_volume(value); }
            },
            "advanced-top-bar-widget-wifi" => {
                settings::set_bool(&["advanced", "top_bar_widget_wifi"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_wifi(value); }
            },
            "advanced-top-bar-widget-battery" => {
                settings::set_bool(&["advanced", "top_bar_widget_battery"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_battery(value); }
            },
            "advanced-top-bar-widget-timer" => {
                settings::set_bool(&["advanced", "top_bar_widget_timer"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_timer(value); }
            },
            "advanced-top-bar-widget-calendar" => {
                settings::set_bool(&["advanced", "top_bar_widget_calendar"], value);
                if let Some(main_ui) = ui_weak_bool.upgrade() { main_ui.set_top_bar_widget_calendar(value); }
            },

            // Drop Shelf
            "drop-enabled" => { settings::set_bool(&["drop", "enabled"], value); },
            "drop-auto-expand" => { settings::set_bool(&["drop", "auto_expand"], value); },
            "drop-open-after-drop" => { settings::set_bool(&["drop", "open_after_drop"], value); },

            // Desktop Widgets
            "widgets-enabled" => {
                settings::set_bool(&["widgets", "enabled"], value);
                update_widget_lifecycles_bool();
            },
            "widgets-click-through" => {
                settings::set_bool(&["widgets", "click_through"], value);
                update_widget_lifecycles_bool();
            },
            "widgets-locked" => {
                settings::set_bool(&["widgets", "locked"], value);
                update_widget_lifecycles_bool();
            },
            "widgets-year-progress-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "year_journey_enabled"], "year_progress", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-day-progress-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "day_journey_enabled"], "day_progress", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-month-progress-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "month_journey_enabled"], "month_progress", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-media-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "media_enabled"], "media", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-notes-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "notes_enabled"], "notes", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-todo-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "todo_enabled"], "todo", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-quotes-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "quotes_enabled"], "quotes", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-picture-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "picture_enabled"], "picture", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-video-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "video_enabled"], "video", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-battery-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "battery_widget_enabled"], "battery", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-calendar-focus-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "calendar_focus_enabled"], "calendar_focus", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-apps-container-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "apps_container_enabled"], "apps_container", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-focus-score-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "focus_score_widget_enabled"], "focus_score", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-streak-enabled" => {
                set_or_copy_builtin_widget(&["widgets", "streak_widget_enabled"], "streak", value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "todo-hide-completed" => {
                settings::set_bool(&["widgets", "todo_hide_completed"], value);
                update_widget_lifecycles_bool();
            },
            "quotes-cycle-enabled" => {
                settings::set_bool(&["widgets", "quotes_cycle_enabled"], value);
                update_widget_lifecycles_bool();
            },
            "widgets-clock-enabled" => {
                settings::set_bool(&["widgets", "clock_enabled"], value);
                settings::set_bool(&["widgets", "stats_enabled"], value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    s_ui.set_widgets_stats_enabled(value);
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-stats-enabled" => {
                settings::set_bool(&["widgets", "stats_enabled"], value);
                settings::set_bool(&["widgets", "clock_enabled"], value);
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    s_ui.set_widgets_clock_enabled(value);
                    if value {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-actions-enabled" => {
                settings::set_bool(&["widgets", "actions_enabled"], value);
                if value {
                    settings::set_bool(&["widgets", "enabled"], true);
                    if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                        s_ui.set_widgets_enabled(true);
                    }
                }
                update_widget_lifecycles_bool();
            },
            "widgets-stats-show-cpu" => {
                let idx = if let Some(s_ui) = settings_ui_weak_bool.upgrade() { s_ui.get_selected_clock_index() as usize } else { 0 };
                settings::update_clock_instance_setting(idx, |inst| { inst.show_cpu = value; });
                update_widget_lifecycles_bool();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    sync_selected_clock_settings_to_ui(&s_ui, &settings, idx);
                    sync_all_clocks_to_ui(&s_ui, &settings);
                }
            },
            "widgets-stats-show-ram" => {
                let idx = if let Some(s_ui) = settings_ui_weak_bool.upgrade() { s_ui.get_selected_clock_index() as usize } else { 0 };
                settings::update_clock_instance_setting(idx, |inst| { inst.show_ram = value; });
                update_widget_lifecycles_bool();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    sync_selected_clock_settings_to_ui(&s_ui, &settings, idx);
                    sync_all_clocks_to_ui(&s_ui, &settings);
                }
            },
            "widgets-stats-show-battery" => {
                let idx = if let Some(s_ui) = settings_ui_weak_bool.upgrade() { s_ui.get_selected_clock_index() as usize } else { 0 };
                settings::update_clock_instance_setting(idx, |inst| { inst.show_battery = value; });
                update_widget_lifecycles_bool();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    sync_selected_clock_settings_to_ui(&s_ui, &settings, idx);
                    sync_all_clocks_to_ui(&s_ui, &settings);
                }
            },
            "widgets-stats-show-percentage" => {
                let idx = if let Some(s_ui) = settings_ui_weak_bool.upgrade() { s_ui.get_selected_clock_index() as usize } else { 0 };
                settings::update_clock_instance_setting(idx, |inst| { inst.show_percentage = value; });
                update_widget_lifecycles_bool();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
                    sync_selected_clock_settings_to_ui(&s_ui, &settings, idx);
                    sync_all_clocks_to_ui(&s_ui, &settings);
                }
            },

            _ => { println!("[Settings] Unknown bool setting: {} = {}", setting, value); }
        }
        if let Some(s_ui) = settings_ui_weak_bool.upgrade() {
            let settings = settings::RavenSettings::load();
            update_settings_preview_clock(&s_ui, &settings);
            reconcile_widget_order(&s_ui);
        }
        println!("[SETTINGS-LOG] on_bool_setting_changed completed in {:?}", t_start.elapsed());
    });

    let ui_weak_string = ui.as_weak();
    let settings_ui_weak = settings_ui.as_weak();
    let update_widget_lifecycles_str = update_widget_lifecycles.clone();
    settings_ui.on_string_setting_changed(move |setting, value| {
        let t_start = std::time::Instant::now();
        println!("[SETTINGS-LOG] on_string_setting_changed start: {} = {}", setting, value);
        if setting.as_str().starts_with("widgets")
            && settings_ui_weak
                .upgrade()
                .map(|ui| ui.get_premium_locked())
                .unwrap_or(false)
        {
            println!("[LICENSE] Ignored locked widget string setting: {}", setting);
            return;
        }
        match setting.as_str() {
            "appearance-mode" => {
                settings::set_string(&["appearance", "appearance_mode"], value.as_str());
                if let Some(main_ui) = ui_weak_string.upgrade() {
                    main_ui.set_appearance_mode(value.clone().into());
                }
                if let Some(s_ui) = settings_ui_weak.upgrade() {
                    s_ui.set_appearance_mode(value.clone().into());
                }
            },
            "notch-color" => { settings::set_string(&["appearance", "notch_color"], value.as_str()); },
            "accent-color" => { settings::set_string(&["appearance", "accent_color"], value.as_str()); },
            "idle-pill-mode" => {
                settings::set_string(&["appearance", "idle_pill_mode"], value.as_str());
                if let Some(main_ui) = ui_weak_string.upgrade() {
                    main_ui.set_idle_pill_mode(value.clone().into());
                }
            },
            "shape" => {
                settings::set_string(&["appearance", "shape"], value.as_str());
                if let Some(main_ui) = ui_weak_string.upgrade() {
                    main_ui.set_appearance_shape(value.clone().into());
                }
            },
            "idle-custom-name" => {
                settings::set_string(&["appearance", "idle_custom_name"], value.as_str());
                if let Some(main_ui) = ui_weak_string.upgrade() {
                    main_ui.set_idle_custom_name(value.clone().into());
                }
            },
            // Calendar
            "media-calendar-url" => { settings::set_string(&["media", "calendar_url"], value.as_str()); },
            // Drop
            "drop-default-provider" => {
                settings::set_string(&["drop", "default_provider"], value.as_str());
                if let Some(s_ui) = settings_ui_weak.upgrade() {
                    s_ui.set_drop_default_provider(value.clone().into());
                }
                if let Some(main_ui) = ui_weak_string.upgrade() {
                    let next_name = match value.as_str() {
                        "quickshare" => "Quick Share",
                        "kdeconnect" => "KDE Connect",
                        _ => "LocalSend",
                    };
                    main_ui.set_share_provider_id(value.clone().into());
                    main_ui.set_share_provider_name(next_name.into());
                }
            },
            // Intelligence
            "intelligence-always-on-charging-mode" => {
                settings::set_string(&["intelligence", "always_on_charging_mode"], value.as_str());
                if let Some(main_ui) = ui_weak_string.upgrade() {
                    main_ui.set_always_on_charging_mode(value.clone().into());
                }
                if let Some(s_ui) = settings_ui_weak.upgrade() {
                    s_ui.set_intelligence_always_on_charging_mode(value.clone().into());
                }
            },
            // Widgets stats colors
            "widgets-stats-cpu-color" => {
                let idx = if let Some(s_ui) = settings_ui_weak.upgrade() { s_ui.get_selected_clock_index() as usize } else { 0 };
                settings::update_clock_instance_setting(idx, |inst| { inst.cpu_color = value.to_string(); });
                update_widget_lifecycles_str();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak.upgrade() {
                    sync_selected_clock_settings_to_ui(&s_ui, &settings, idx);
                    sync_all_clocks_to_ui(&s_ui, &settings);
                }
            },
            "widgets-stats-ram-color" => {
                let idx = if let Some(s_ui) = settings_ui_weak.upgrade() { s_ui.get_selected_clock_index() as usize } else { 0 };
                settings::update_clock_instance_setting(idx, |inst| { inst.ram_color = value.to_string(); });
                update_widget_lifecycles_str();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak.upgrade() {
                    sync_selected_clock_settings_to_ui(&s_ui, &settings, idx);
                    sync_all_clocks_to_ui(&s_ui, &settings);
                }
            },
            "widgets-stats-battery-color" => {
                let idx = if let Some(s_ui) = settings_ui_weak.upgrade() { s_ui.get_selected_clock_index() as usize } else { 0 };
                settings::update_clock_instance_setting(idx, |inst| { inst.battery_color = value.to_string(); });
                update_widget_lifecycles_str();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak.upgrade() {
                    sync_selected_clock_settings_to_ui(&s_ui, &settings, idx);
                    sync_all_clocks_to_ui(&s_ui, &settings);
                }
            },
            "widgets-size-select" => {
                let idx = if let Some(s_ui) = settings_ui_weak.upgrade() { s_ui.get_selected_clock_index() as usize } else { 0 };
                settings::update_clock_instance_setting(idx, |inst| { inst.size = value.to_string(); });
                update_widget_lifecycles_str();
                let settings = settings::RavenSettings::load();
                if let Some(s_ui) = settings_ui_weak.upgrade() {
                    sync_selected_clock_settings_to_ui(&s_ui, &settings, idx);
                    sync_all_clocks_to_ui(&s_ui, &settings);
                }
            },
            "todo-accent-color-str" => {
                settings::set_string(&["widgets", "todo_accent_color"], value.as_str());
                update_widget_lifecycles_str();
            },
            "widgets-notes-text" => {
                settings::set_string(&["widgets", "notes_text"], value.as_str());
                update_widget_lifecycles_str();
            },
            // Keyboard Shortcuts — persist and re-register hotkeys
            "shortcut-toggle-raven"       => { settings::set_string(&["shortcuts", "toggle_raven"], value.as_str()); },
            "shortcut-tab-home"           => { settings::set_string(&["shortcuts", "tab_home"], value.as_str()); },
            "shortcut-tab-media"          => { settings::set_string(&["shortcuts", "tab_media"], value.as_str()); },
            "shortcut-tab-calendar"       => { settings::set_string(&["shortcuts", "tab_calendar"], value.as_str()); },
            "shortcut-tab-clock"          => { settings::set_string(&["shortcuts", "tab_clock"], value.as_str()); },
            "shortcut-tab-drop"           => { settings::set_string(&["shortcuts", "tab_drop"], value.as_str()); },
            "shortcut-tab-capture"        => { settings::set_string(&["shortcuts", "tab_capture"], value.as_str()); },
            "shortcut-tab-stats"          => { settings::set_string(&["shortcuts", "tab_stats"], value.as_str()); },
            "shortcut-media-play"         => { settings::set_string(&["shortcuts", "media_play"], value.as_str()); },
            "shortcut-media-next"         => { settings::set_string(&["shortcuts", "media_next"], value.as_str()); },
            "shortcut-media-prev"         => { settings::set_string(&["shortcuts", "media_prev"], value.as_str()); },
            "shortcut-toggle-freeze"      => { settings::set_string(&["shortcuts", "toggle_freeze"], value.as_str()); },
            "shortcut-quick-screenshot"   => { settings::set_string(&["shortcuts", "quick_screenshot"], value.as_str()); },
            "shortcut-quick-record-toggle"=> { settings::set_string(&["shortcuts", "quick_record_toggle"], value.as_str()); },
            "shortcut-open-settings"      => { settings::set_string(&["shortcuts", "open_settings"], value.as_str()); },
            "shortcut-restart-raven"      => { settings::set_string(&["shortcuts", "restart_raven"], value.as_str()); },
            "shortcut-quit-raven"         => { settings::set_string(&["shortcuts", "quit_raven"], value.as_str()); },
            _ => { println!("[Settings] Unknown string setting: {} = {}", setting, value); }
        }
        if setting.starts_with("shortcut-") {
            let s_settings = settings::RavenSettings::load();
            let main_hwnd_val = crate::window::PILL_HWND.load(std::sync::atomic::Ordering::SeqCst);
            if main_hwnd_val != 0 {
                let main_hwnd = windows::Win32::Foundation::HWND(main_hwnd_val as _);
                unsafe {
                    crate::window::register_raven_hotkeys(main_hwnd, &s_settings);
                }
            }
        }
        if let Some(s_ui) = settings_ui_weak.upgrade() {
            let settings = settings::RavenSettings::load();
            update_settings_preview_clock(&s_ui, &settings);
        }
        println!("[SETTINGS-LOG] on_string_setting_changed completed in {:?}", t_start.elapsed());
    });

    settings_ui.on_play_sound(move |sound_name| {
        play_sound_by_name_bypass(sound_name.as_str(), true);
    });

    let settings_ui_weak_upload = settings_ui.as_weak();
    settings_ui.on_upload_sound(move |sound_name| {
        let sound_name = sound_name.to_string();
        println!("[SOUND] on_upload_sound callback triggered for: {}", sound_name);
        let settings_ui_weak = settings_ui_weak_upload.clone();
        std::thread::spawn(move || {
            println!("[SOUND] Background file dialog thread started for: {}", sound_name);
            let title = match sound_name.as_str() {
                "timer_complete" => "Select Custom Timer Complete Sound",
                "stopwatch" => "Select Custom Stopwatch Sound",
                "battery_low" => "Select Custom Battery Low Sound",
                "charger_connected" => "Select Custom Charger Connected Sound",
                "charger_disconnected" => "Select Custom Charger Disconnected Sound",
                "capslock_on" => "Select Custom Caps Lock On Sound",
                "capslock_off" => "Select Custom Caps Lock Off Sound",
                "unlock" => "Select Custom Screen Unlock Sound",
                _ => "Select Custom Sound",
            };

            if let Some(path) = pick_mp3_file(title) {
                println!("[SOUND] User picked file for {}: {}", sound_name, path);
                let setting_key = match sound_name.as_str() {
                    "timer_complete" => "custom_timer_complete_path",
                    "stopwatch" => "custom_stopwatch_path",
                    "battery_low" => "custom_battery_low_path",
                    "charger_connected" => "custom_charger_connected_path",
                    "charger_disconnected" => "custom_charger_disconnected_path",
                    "capslock_on" => "custom_capslock_on_path",
                    "capslock_off" => "custom_capslock_off_path",
                    "unlock" => "custom_unlock_path",
                    _ => "",
                };

                if !setting_key.is_empty() {
                    println!("[SOUND] Setting json value for sound key '{}' to '{}'", setting_key, path);
                    let _updated = settings::set_string(&["sounds", setting_key], &path);
                    let path_cloned = path.clone();
                    let sound_name_cloned = sound_name.clone();
                    println!("[SOUND] Dispatching UI property update for '{}' back to main thread", sound_name_cloned);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(s_ui) = settings_ui_weak.upgrade() {
                            println!("[SOUND] Main thread upgrading SettingsWindow handle for '{}'", sound_name_cloned);
                            match sound_name_cloned.as_str() {
                                "timer_complete" => s_ui.set_sound_timer_complete_custom_path(path_cloned.into()),
                                "stopwatch" => s_ui.set_sound_stopwatch_custom_path(path_cloned.into()),
                                "battery_low" => s_ui.set_sound_battery_low_custom_path(path_cloned.into()),
                                "charger_connected" => s_ui.set_sound_charger_connected_custom_path(path_cloned.into()),
                                "charger_disconnected" => s_ui.set_sound_charger_disconnected_custom_path(path_cloned.into()),
                                "capslock_on" => s_ui.set_sound_capslock_on_custom_path(path_cloned.into()),
                                "capslock_off" => s_ui.set_sound_capslock_off_custom_path(path_cloned.into()),
                                "unlock" => s_ui.set_sound_unlock_custom_path(path_cloned.into()),
                                _ => {}
                            }
                            println!("[SOUND] UI custom path update completed for '{}'", sound_name_cloned);
                        } else {
                            println!("[SOUND] Main thread upgrade failed! SettingsWindow was already closed/dropped.");
                        }
                    });
                }
            } else {
                println!("[SOUND] Dialog was cancelled or failed for: {}", sound_name);
            }
        });
    });

    let settings_ui_weak_reset = settings_ui.as_weak();
    settings_ui.on_reset_sound(move |sound_name| {
        println!("[SOUND] Reset custom sound for {} to default", sound_name);
        let setting_key = match sound_name.as_str() {
            "timer_complete" => "custom_timer_complete_path",
            "stopwatch" => "custom_stopwatch_path",
            "battery_low" => "custom_battery_low_path",
            "charger_connected" => "custom_charger_connected_path",
            "charger_disconnected" => "custom_charger_disconnected_path",
            "capslock_on" => "custom_capslock_on_path",
            "capslock_off" => "custom_capslock_off_path",
            "unlock" => "custom_unlock_path",
            _ => "",
        };

        if !setting_key.is_empty() {
            let _updated = settings::set_string(&["sounds", setting_key], "");
            if let Some(s_ui) = settings_ui_weak_reset.upgrade() {
                match sound_name.as_str() {
                    "timer_complete" => s_ui.set_sound_timer_complete_custom_path("".into()),
                    "stopwatch" => s_ui.set_sound_stopwatch_custom_path("".into()),
                    "battery_low" => s_ui.set_sound_battery_low_custom_path("".into()),
                    "charger_connected" => s_ui.set_sound_charger_connected_custom_path("".into()),
                    "charger_disconnected" => s_ui.set_sound_charger_disconnected_custom_path("".into()),
                    "capslock_on" => s_ui.set_sound_capslock_on_custom_path("".into()),
                    "capslock_off" => s_ui.set_sound_capslock_off_custom_path("".into()),
                    "unlock" => s_ui.set_sound_unlock_custom_path("".into()),
                    _ => {}
                }
            }
        }
    });

    // Shortcut recorder callbacks
    let settings_ui_weak_record = settings_ui.as_weak();
    settings_ui.on_record_shortcut(move |shortcut_id| {
        // Toggle recording state: set the recording-shortcut-key to the ID (or clear if already recording that one)
        if let Some(s_ui) = settings_ui_weak_record.upgrade() {
            let current = s_ui.get_recording_shortcut_key().to_string();
            if current == shortcut_id.as_str() {
                // Already recording this one — cancel
                s_ui.set_recording_shortcut_key("".into());
                let h_hook = {
                    let mut state = SHORTCUT_HOOK.lock().unwrap();
                    let h = state.h_hook.take();
                    state.shortcut_id = None;
                    state.settings_ui = None;
                    h
                };
                if let Some(h) = h_hook {
                    unsafe { let _ = UnhookWindowsHookEx(h); }
                }
                println!("[SHORTCUT] Cancelled recording shortcut for: {}", shortcut_id);
            } else {
                // Start recording for this shortcut
                s_ui.set_recording_shortcut_key(shortcut_id.clone());
                println!("[SHORTCUT] Recording shortcut for: {}", shortcut_id);
                // Install hook
                unsafe {
                    let thread_id = GetCurrentThreadId();
                    let hook = SetWindowsHookExW(
                        WH_KEYBOARD,
                        Some(keyboard_hook_proc),
                        None,
                        thread_id,
                    ).ok();
                    let old_hook = {
                        let mut state = SHORTCUT_HOOK.lock().unwrap();
                        let old = state.h_hook.take();
                        state.h_hook = hook;
                        state.shortcut_id = Some(shortcut_id.to_string());
                        state.settings_ui = Some(settings_ui_weak_record.clone());
                        state.is_low_level = false;
                        old
                    };
                    if let Some(old) = old_hook {
                        let _ = UnhookWindowsHookEx(old);
                    }
                    println!("[SHORTCUT] Thread-local hook registered with status: {:?}", hook);
                }
            }
        }
    });

    let settings_ui_weak_clear = settings_ui.as_weak();
    settings_ui.on_clear_shortcut(move |shortcut_id| {
        println!("[SHORTCUT] Clearing shortcut for: {}", shortcut_id);
        // Map shortcut_id → settings JSON key (Slint uses underscored IDs)
        let json_key = match shortcut_id.as_str() {
            "toggle_raven"        => Some(("shortcuts", "toggle_raven")),
            "tab_home"            => Some(("shortcuts", "tab_home")),
            "tab_media"           => Some(("shortcuts", "tab_media")),
            "tab_calendar"        => Some(("shortcuts", "tab_calendar")),
            "tab_clock"           => Some(("shortcuts", "tab_clock")),
            "tab_drop"            => Some(("shortcuts", "tab_drop")),
            "tab_capture"         => Some(("shortcuts", "tab_capture")),
            "tab_stats"           => Some(("shortcuts", "tab_stats")),
            "media_play"          => Some(("shortcuts", "media_play")),
            "media_next"          => Some(("shortcuts", "media_next")),
            "media_prev"          => Some(("shortcuts", "media_prev")),
            "toggle_freeze"       => Some(("shortcuts", "toggle_freeze")),
            "quick_screenshot"    => Some(("shortcuts", "quick_screenshot")),
            "quick_record_toggle" => Some(("shortcuts", "quick_record_toggle")),
            "open_settings"       => Some(("shortcuts", "open_settings")),
            "restart_raven"       => Some(("shortcuts", "restart_raven")),
            "quit_raven"          => Some(("shortcuts", "quit_raven")),
            _ => None,
        };
        let mut updated_settings = None;
        if let Some((section, key)) = json_key {
            updated_settings = Some(settings::set_string(&[section, key], ""));
        }
        if let Some(s_settings) = updated_settings {
            let main_hwnd_val = crate::window::PILL_HWND.load(std::sync::atomic::Ordering::SeqCst);
            if main_hwnd_val != 0 {
                let main_hwnd = windows::Win32::Foundation::HWND(main_hwnd_val as _);
                unsafe {
                    crate::window::register_raven_hotkeys(main_hwnd, &s_settings);
                }
            }
        }
        // Update the Slint property to show "—"
        if let Some(s_ui) = settings_ui_weak_clear.upgrade() {
            let empty: slint::SharedString = "".into();
            match shortcut_id.as_str() {
                "toggle_raven"        => s_ui.set_shortcut_toggle_raven(empty),
                "tab_home"            => s_ui.set_shortcut_tab_home(empty),
                "tab_media"           => s_ui.set_shortcut_tab_media(empty),
                "tab_calendar"        => s_ui.set_shortcut_tab_calendar(empty),
                "tab_clock"           => s_ui.set_shortcut_tab_clock(empty),
                "tab_drop"            => s_ui.set_shortcut_tab_drop(empty),
                "tab_capture"         => s_ui.set_shortcut_tab_capture(empty),
                "tab_stats"           => s_ui.set_shortcut_tab_stats(empty),
                "media_play"          => s_ui.set_shortcut_media_play(empty),
                "media_next"          => s_ui.set_shortcut_media_next(empty),
                "media_prev"          => s_ui.set_shortcut_media_prev(empty),
                "toggle_freeze"       => s_ui.set_shortcut_toggle_freeze(empty),
                "quick_screenshot"    => s_ui.set_shortcut_quick_screenshot(empty),
                "quick_record_toggle" => s_ui.set_shortcut_quick_record_toggle(empty),
                "open_settings"       => s_ui.set_shortcut_open_settings(empty),
                "restart_raven"       => s_ui.set_shortcut_restart_raven(empty),
                "quit_raven"          => s_ui.set_shortcut_quit_raven(empty),
                _ => {}
            }
        }
    });

    let settings_ui_close_weak = settings_ui.as_weak();
    settings_ui.on_close_clicked(move || {
        println!("[BTN-TRACE] CLOSE → callback entered");
        SETTINGS_WINDOW_OPEN.store(false, Ordering::SeqCst);
        // Clean up shortcut hook
        let mut state = SHORTCUT_HOOK.lock().unwrap();
        if let Some(h) = state.h_hook.take() {
            unsafe { let _ = UnhookWindowsHookEx(h); }
        }
        state.shortcut_id = None;
        state.settings_ui = None;
        drop(state);
        if let Some(s_ui) = settings_ui_close_weak.upgrade() {
            s_ui.set_recording_shortcut_key("".into());
            let _ = s_ui.hide();
            println!("[BTN-TRACE] CLOSE → hidden");
        }
    });

    let settings_ui_on_close_weak = settings_ui.as_weak();
    settings_ui.window().on_close_requested(move || {
        SETTINGS_WINDOW_OPEN.store(false, Ordering::SeqCst);
        if let Some(s_ui) = settings_ui_on_close_weak.upgrade() {
            let _ = s_ui.hide();
        }
        slint::CloseRequestResponse::KeepWindowShown
    });

    let settings_ui_min_weak = settings_ui.as_weak();
    settings_ui.on_minimize_clicked(move || {
        println!("[BTN-TRACE] MINIMIZE → callback entered");
        if let Some(s_ui) = settings_ui_min_weak.upgrade() {
            let mut resolved_hwnd = None;
            use raw_window_handle::HasWindowHandle;
            if let Ok(handle) = s_ui.window().window_handle().window_handle() {
                if let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() {
                    resolved_hwnd = Some(windows::Win32::Foundation::HWND(h.hwnd.get() as _));
                }
            }
            if resolved_hwnd.is_none() {
                unsafe {
                    let title = wide("Raven Settings");
                    let hwnd = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                        None,
                        windows::core::PCWSTR(title.as_ptr()),
                    );
                    if hwnd.0 != 0 {
                        resolved_hwnd = Some(hwnd);
                    }
                }
            }
            if let Some(hwnd) = resolved_hwnd {
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::*;
                    println!("[BTN-TRACE] MINIMIZE → ShowWindow SW_MINIMIZE hwnd={:?}", hwnd);
                    let _ = ShowWindow(hwnd, SW_MINIMIZE);
                }
            } else {
                println!("[BTN-TRACE] MINIMIZE → Failed to resolve HWND!");
            }
        }
    });

    let settings_ui_max_weak = settings_ui.as_weak();
    settings_ui.on_maximize_clicked(move || {
        println!("[BTN-TRACE] MAXIMIZE → callback entered");
        if let Some(s_ui) = settings_ui_max_weak.upgrade() {
            let mut resolved_hwnd = None;
            use raw_window_handle::HasWindowHandle;
            if let Ok(handle) = s_ui.window().window_handle().window_handle() {
                if let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() {
                    resolved_hwnd = Some(windows::Win32::Foundation::HWND(h.hwnd.get() as _));
                }
            }
            if resolved_hwnd.is_none() {
                unsafe {
                    let title = wide("Raven Settings");
                    let hwnd = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                        None,
                        windows::core::PCWSTR(title.as_ptr()),
                    );
                    if hwnd.0 != 0 {
                        resolved_hwnd = Some(hwnd);
                    }
                }
            }
            if let Some(hwnd) = resolved_hwnd {
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::*;
                    let is_zoomed = IsZoomed(hwnd).as_bool();
                    let show_cmd = if is_zoomed { SW_RESTORE } else { SW_MAXIMIZE };
                    println!("[BTN-TRACE] MAXIMIZE → ShowWindow {} hwnd={:?}", if is_zoomed { "SW_RESTORE" } else { "SW_MAXIMIZE" }, hwnd);
                    let _ = ShowWindow(hwnd, show_cmd);
                }
            } else {
                println!("[BTN-TRACE] MAXIMIZE → Failed to resolve HWND!");
            }
        }
    });

    let settings_ui_drag_weak = settings_ui.as_weak();
    settings_ui.on_drag_window(move || {
        if let Some(s_ui) = settings_ui_drag_weak.upgrade() {
            let mut resolved_hwnd = None;
            use raw_window_handle::HasWindowHandle;
            if let Ok(handle) = s_ui.window().window_handle().window_handle() {
                if let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() {
                    resolved_hwnd = Some(windows::Win32::Foundation::HWND(h.hwnd.get() as _));
                }
            }
            if resolved_hwnd.is_none() {
                unsafe {
                    let title = wide("Raven Settings");
                    let hwnd = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                        None,
                        windows::core::PCWSTR(title.as_ptr()),
                    );
                    if hwnd.0 != 0 {
                        resolved_hwnd = Some(hwnd);
                    }
                }
            }
            if let Some(hwnd) = resolved_hwnd {
                unsafe {
                    let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                        hwnd,
                        161, // WM_NCLBUTTONDOWN
                        windows::Win32::Foundation::WPARAM(2), // HTCAPTION
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
        }
    });


    // Initialize settings_ui Google Calendar state from current snapshot
    {
        let snap = services.snapshot();
        settings_ui.set_google_connected(snap.calendar.google_connected);
        settings_ui.set_google_email(snap.calendar.google_email.clone().into());
        settings_ui.set_google_message(String::new().into());
    }

    // settings_ui: Connect Google button
    let s_cloned_sg = services.clone();
    let settings_ui_weak_sg = settings_ui.as_weak();
    settings_ui.on_google_sign_in(move || {
        if settings_ui_weak_sg
            .upgrade()
            .map(|ui| ui.get_premium_locked())
            .unwrap_or(false)
        {
            return;
        }
        if let Some(s_ui) = settings_ui_weak_sg.upgrade() {
            s_ui.set_google_busy(true);
            s_ui.set_google_message("".into());
        }
        let services_g = s_cloned_sg.clone();
        let ui_weak_done = settings_ui_weak_sg.clone();
        std::thread::spawn(move || {
            let snapshot = services_g.connect_google_calendar();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(s_ui) = ui_weak_done.upgrade() {
                    s_ui.set_google_connected(snapshot.calendar.google_connected);
                    s_ui.set_google_email(snapshot.calendar.google_email.clone().into());
                    s_ui.set_google_message(snapshot.calendar.message.clone().into());
                    s_ui.set_google_busy(false);

                    let settings = settings::RavenSettings::load();
                    let selected_ids = settings.media.google_calendar_ids.clone();
                    let slint_cals = map_google_calendars(&snapshot.calendar.google_calendars, &selected_ids);
                    let selected_count = if selected_ids.is_empty() && snapshot.calendar.google_connected {
                        1
                    } else {
                        selected_ids.len()
                    };
                    s_ui.set_google_calendars(std::rc::Rc::new(slint::VecModel::from(slint_cals)).into());
                    s_ui.set_google_selected_calendars_count(selected_count as i32);
                }
            });
        });
    });

    // settings_ui: Disconnect Google button
    let s_cloned_dg = services.clone();
    let settings_ui_weak_dg = settings_ui.as_weak();
    settings_ui.on_disconnect_google(move || {
        if settings_ui_weak_dg
            .upgrade()
            .map(|ui| ui.get_premium_locked())
            .unwrap_or(false)
        {
            return;
        }
        let services_d = s_cloned_dg.clone();
        let ui_weak_done = settings_ui_weak_dg.clone();
        std::thread::spawn(move || {
            let snapshot = services_d.disconnect_google_calendar();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(s_ui) = ui_weak_done.upgrade() {
                    s_ui.set_google_connected(false);
                    s_ui.set_google_email("".into());
                    s_ui.set_google_message(snapshot.calendar.message.clone().into());
                    s_ui.set_google_busy(false);
                }
            });
        });
    });

    // settings_ui: Toggle Google calendar
    let s_cloned_tc = services.clone();
    let settings_ui_weak_tc = settings_ui.as_weak();
    let main_ui_weak_tc = ui.as_weak();
    settings_ui.on_toggle_google_calendar(move |cal_id| {
        if settings_ui_weak_tc
            .upgrade()
            .map(|ui| ui.get_premium_locked())
            .unwrap_or(false)
        {
            return;
        }
        let cal_id_str = cal_id.to_string();
        println!("[SETTINGS-LOG] toggle_google_calendar: {}", cal_id_str);
        
        let settings = settings::toggle_google_calendar_id(&cal_id_str);
        let selected_ids = settings.media.google_calendar_ids.clone();
        
        // Optimistic UI update: Toggle selected state in Slint model instantly using upgraded weak reference
        if let Some(s_ui) = settings_ui_weak_tc.upgrade() {
            let current_cals = s_ui.get_google_calendars();
            let mut slint_cals = Vec::new();
            let mut selected_count = 0;
            
            for i in 0..current_cals.row_count() {
                if let Some(mut entry) = current_cals.row_data(i) {
                    if entry.id == cal_id {
                        entry.selected = !entry.selected;
                    }
                    if entry.selected {
                        selected_count += 1;
                    }
                    slint_cals.push(entry);
                }
            }
            
            if selected_count == 0 {
                for entry in &slint_cals {
                    if entry.primary {
                        selected_count = 1;
                        break;
                    }
                }
            }
            
            s_ui.set_google_calendars(std::rc::Rc::new(slint::VecModel::from(slint_cals)).into());
            s_ui.set_google_selected_calendars_count(selected_count);
        }
        
        let services_tc = s_cloned_tc.clone();
        let ui_weak_done = settings_ui_weak_tc.clone();
        let main_ui_done = main_ui_weak_tc.clone();
        std::thread::spawn(move || {
            let snapshot = services_tc.refresh_calendar();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(s_ui) = ui_weak_done.upgrade() {
                    let slint_cals = map_google_calendars(&snapshot.calendar.google_calendars, &selected_ids);
                    let selected_count = if selected_ids.is_empty() && snapshot.calendar.google_connected {
                        1
                    } else {
                        selected_ids.len()
                    };
                    s_ui.set_google_calendars(std::rc::Rc::new(slint::VecModel::from(slint_cals)).into());
                    s_ui.set_google_selected_calendars_count(selected_count as i32);
                }
                if let Some(main_ui) = main_ui_done.upgrade() {
                    let events: Vec<SlintCalendarEvent> = snapshot.calendar.items.iter().map(|item| {
                        SlintCalendarEvent {
                            title: item.title.clone().into(),
                            date_str: item.date_str.clone().into(),
                        }
                    }).collect();
                    main_ui.set_calendar_events(std::rc::Rc::new(slint::VecModel::from(events)).into());
                }
            });
        });
    });

    // settings_ui: About callbacks
    let settings_ui_weak_about = settings_ui.as_weak();
    settings_ui.on_check_updates(move || {
        if let Some(s_ui) = settings_ui_weak_about.upgrade() {
            if s_ui.get_update_available() {
                let current_btn_text = s_ui.get_update_button_text().to_string();
                if current_btn_text.starts_with("Downloading") {
                    return;
                }

                s_ui.set_update_button_text("Downloading...".into());
                s_ui.set_about_update_status("Downloading update...".into());

                let url = s_ui.get_update_url().to_string();
                let s_ui_weak = settings_ui_weak_about.clone();

                std::thread::spawn(move || {
                    let download_res = (|| -> Result<(), String> {
                        use std::io::{Read, Write};

                        // 1. Send HTTP request
                        let response = ureq::get(&url)
                            .call()
                            .map_err(|e| format!("Connection failed: {}", e))?;
                        
                        let total_size = response.header("Content-Length")
                            .and_then(|len| len.parse::<u64>().ok())
                            .unwrap_or(0);

                        // 2. Read reader chunk by chunk
                        let mut reader = response.into_reader();
                        let temp_dir = std::env::temp_dir();
                        let dest_path = temp_dir.join("Raven-Notch-Setup.exe");
                        let mut file = std::fs::File::create(&dest_path)
                            .map_err(|e| format!("Failed to create file: {}", e))?;

                        let mut buffer = [0; 16384];
                        let mut downloaded = 0;

                        loop {
                            let bytes_read = reader.read(&mut buffer)
                                .map_err(|e| format!("Read error: {}", e))?;
                            if bytes_read == 0 {
                                break;
                            }
                            file.write_all(&buffer[..bytes_read])
                                .map_err(|e| format!("Write error: {}", e))?;
                            downloaded += bytes_read as u64;

                            if total_size > 0 {
                                let pct = (downloaded * 100 / total_size) as i32;
                                let text = format!("Downloading {pct}%");
                                let s_ui_inner = s_ui_weak.clone();
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(ui) = s_ui_inner.upgrade() {
                                        ui.set_update_button_text(text.into());
                                    }
                                });
                            }
                        }

                        // Flush and close file
                        file.sync_all().map_err(|e| e.to_string())?;
                        drop(file);

                        // 3. Launch the installer silently in the background
                        #[cfg(target_os = "windows")]
                        {
                            use std::os::windows::process::CommandExt;
                            let _ = std::process::Command::new(dest_path)
                                .args(["/VERYSILENT", "/SP-", "/SUPPRESSMSGBOXES"])
                                .creation_flags(0x08000000)
                                .spawn();
                        }

                        // 4. Exit the application immediately
                        std::process::exit(0);
                    })();

                    if let Err(err) = download_res {
                        eprintln!("[UPDATER-ERROR] {}", err);
                        let s_ui_inner = s_ui_weak.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = s_ui_inner.upgrade() {
                                ui.set_update_button_text("".into());
                                ui.set_about_update_status(format!("Download failed: {}", err).into());
                            }
                        });
                    }
                });
            } else {
                s_ui.set_about_update_status("Checking...".into());
                let ui_weak = settings_ui_weak_about.clone();
                std::thread::spawn(move || {
                    match fetch_latest_version() {
                        Ok(manifest) => {
                            let current_version = env!("CARGO_PKG_VERSION");
                            let is_newer = is_newer_version(current_version, &manifest.version);
                            
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak.upgrade() {
                                    if is_newer {
                                        ui.set_update_available(true);
                                        ui.set_update_url(manifest.url.into());
                                        ui.set_about_update_status(format!("New version available: v{}", manifest.version).into());
                                    } else {
                                        ui.set_update_available(false);
                                        ui.set_about_update_status("Up to date!".into());
                                        
                                        let ui_reset = ui_weak.clone();
                                        slint::Timer::single_shot(std::time::Duration::from_secs(3), move || {
                                            if let Some(ui_r) = ui_reset.upgrade() {
                                                if ui_r.get_about_update_status() == "Up to date!" {
                                                    ui_r.set_about_update_status("".into());
                                                }
                                            }
                                        });
                                    }
                                }
                            });
                        }
                        Err(err) => {
                            println!("[UPDATER-ERROR] Failed to fetch version: {}", err);
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak.upgrade() {
                                    ui.set_about_update_status("Connection failed".into());
                                    
                                    let ui_reset = ui_weak.clone();
                                    slint::Timer::single_shot(std::time::Duration::from_secs(3), move || {
                                        if let Some(ui_r) = ui_reset.upgrade() {
                                            if ui_r.get_about_update_status() == "Connection failed" {
                                                ui_r.set_about_update_status("".into());
                                            }
                                        }
                                    });
                                }
                            });
                        }
                    }
                });
            }
        }
    });

    let ui_weak_license_toggle = ui.as_weak();
    let settings_ui_weak_license_toggle = settings_ui.as_weak();
    settings_ui.on_toggle_trial_expired_preview(move || {
        if let Some(settings_ui) = settings_ui_weak_license_toggle.upgrade() {
            let enabled = !settings_ui.get_force_trial_expired_preview();
            match license::set_force_trial_expired_preview(enabled) {
                Ok(status) => {
                    if let Some(ui) = ui_weak_license_toggle.upgrade() {
                        apply_license_status(&ui, &settings_ui, &status);
                    }
                }
                Err(error) => {
                    settings_ui.set_license_status_label(format!("License preview failed: {error}").into());
                }
            }
        }
    });

    let ui_weak_license_activate = ui.as_weak();
    let settings_ui_weak_license_activate = settings_ui.as_weak();
    settings_ui.on_activate_license(move |license_key| {
        if let Some(settings_ui) = settings_ui_weak_license_activate.upgrade() {
            let key = license_key.to_string();
            settings_ui.set_license_action_message("Activating license...".into());
            let ui_weak = ui_weak_license_activate.clone();
            let settings_ui_weak = settings_ui_weak_license_activate.clone();
            std::thread::spawn(move || {
                let result = license::activate_license(key);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(settings_ui) = settings_ui_weak.upgrade() {
                        match result {
                            Ok(status) => {
                                if status.status == "paid_active" {
                                    if let Some(ui) = ui_weak.upgrade() {
                                        apply_license_status(&ui, &settings_ui, &status);
                                    }
                                    settings_ui.set_license_key_input("".into());
                                } else {
                                    settings_ui.set_license_action_message(
                                        status
                                            .message
                                            .unwrap_or_else(|| "Invalid license key. Please check the key and try again.".to_string())
                                            .into(),
                                    );
                                }
                            }
                            Err(error) => {
                                settings_ui.set_license_action_message(format!("Activation failed: {error}").into());
                            }
                        }
                    }
                });
            });
        }
    });

    let settings_ui_weak_license_paste = settings_ui.as_weak();
    settings_ui.on_paste_license_key(move || {
        if let Some(settings_ui) = settings_ui_weak_license_paste.upgrade() {
            if let Some(text) = read_clipboard_text() {
                let clean = text.trim().to_uppercase();
                settings_ui.set_license_key_input(clean.into());
            } else {
                settings_ui.set_license_action_message("Paste failed. Clipboard does not contain a license key.".into());
            }
        }
    });

    let ui_weak_account_sign_in = ui.as_weak();
    let settings_ui_weak_account_sign_in = settings_ui.as_weak();
    settings_ui.on_sign_in_account(move || {
        if let Some(settings_ui) = settings_ui_weak_account_sign_in.upgrade() {
            if settings_ui.get_account_email_label().is_empty() {
                settings_ui.set_license_action_message(
                    "Opening browser. Sign in, then approve opening Raven Notch.".into(),
                );
                open_external_url(&license::account_sign_in_url());
            } else {
                settings_ui.set_license_action_message("Refreshing account purchase...".into());
                let ui_weak = ui_weak_account_sign_in.clone();
                let settings_ui_weak = settings_ui_weak_account_sign_in.clone();
                std::thread::spawn(move || {
                    let result = license::get_license_status();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(settings_ui) = settings_ui_weak.upgrade() {
                            match result {
                                Ok(status) => {
                                    if let Some(ui) = ui_weak.upgrade() {
                                        apply_license_status(&ui, &settings_ui, &status);
                                    }
                                }
                                Err(error) => {
                                    settings_ui.set_license_action_message(
                                        format!("Account refresh failed: {error}").into(),
                                    );
                                }
                            }
                        }
                    });
                });
            }
        }
    });

    let ui_weak_account_sign_out = ui.as_weak();
    let settings_ui_weak_account_sign_out = settings_ui.as_weak();
    settings_ui.on_sign_out_account(move || {
        let ui_weak = ui_weak_account_sign_out.clone();
        let settings_ui_weak = settings_ui_weak_account_sign_out.clone();
        std::thread::spawn(move || {
            let result = license::sign_out_account();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(settings_ui) = settings_ui_weak.upgrade() {
                    match result {
                        Ok(status) => {
                            if let Some(ui) = ui_weak.upgrade() {
                                apply_license_status(&ui, &settings_ui, &status);
                            }
                        }
                        Err(error) => {
                            settings_ui.set_license_action_message(
                                format!("Sign out failed: {error}").into(),
                            );
                        }
                    }
                }
            });
        });
    });

    settings_ui.on_open_url(move |url| {
        println!("[ABOUT] Opening URL: {}", url);
        open_external_url(url.as_str());
    });

    // New Widget Studio Callbacks
    let update_widget_lifecycles_todo_clear = update_widget_lifecycles.clone();
    settings_ui.on_settings_todo_clear_completed(move || {
        let _ = settings::todo_clear_completed();
        update_widget_lifecycles_todo_clear();
    });

    let update_widget_lifecycles_quotes_add = update_widget_lifecycles.clone();
    settings_ui.on_settings_quotes_add_custom(move |text, author| {
        let _ = settings::quotes_add_custom(text.as_str(), author.as_str());
        update_widget_lifecycles_quotes_add();
    });

    let settings_ui_weak_pic = settings_ui.as_weak();
    settings_ui.on_settings_picture_select_image(move || {
        let ui_weak = settings_ui_weak_pic.clone();
        std::thread::spawn(move || {
            if let Some(path) = select_image_file() {
                let _ = slint::invoke_from_event_loop(move || {
                    settings::set_string(&["widgets", "picture_path"], &path);
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_picture_selected_path(path.into());
                    }
                    GLOBAL_UPDATE_LIFECYCLES.with(|cell| {
                        if let Some(f) = cell.borrow().as_ref() {
                            f();
                        }
                    });
                });
            }
        });
    });

    let settings_ui_weak_vid = settings_ui.as_weak();
    settings_ui.on_settings_video_select_media(move || {
        let ui_weak = settings_ui_weak_vid.clone();
        std::thread::spawn(move || {
            if let Some(path) = select_video_file() {
                let _ = slint::invoke_from_event_loop(move || {
                    settings::set_string(&["widgets", "video_path"], &path);
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_video_selected_path(path.into());
                    }
                    GLOBAL_UPDATE_LIFECYCLES.with(|cell| {
                        if let Some(f) = cell.borrow().as_ref() {
                            f();
                        }
                    });
                });
            }
        });
    });

    let update_widget_lifecycles_apply = update_widget_lifecycles.clone();
    let settings_ui_weak_apply = settings_ui.as_weak();
    settings_ui.on_settings_widgets_apply_changes(move || {
        let current = settings::RavenSettings::load();
        let all_builtin_off = !current.widgets.year_journey_enabled
            && !current.widgets.day_journey_enabled
            && !current.widgets.month_journey_enabled
            && !current.widgets.media_enabled
            && !current.widgets.notes_enabled
            && !current.widgets.todo_enabled
            && !current.widgets.quotes_enabled
            && !current.widgets.picture_enabled
            && !current.widgets.video_enabled
            && !current.widgets.battery_widget_enabled
            && !current.widgets.calendar_focus_enabled
            && !current.widgets.apps_container_enabled
            && !current.widgets.focus_score_widget_enabled
            && !current.widgets.streak_widget_enabled
            && current.widgets.clock_count <= 0.0;
        if all_builtin_off {
            settings::clear_widget_instances();
        }
        if let Some(s_ui) = settings_ui_weak_apply.upgrade() {
            reconcile_widget_order(&s_ui);
        }
        update_widget_lifecycles_apply();
    });
 
    // Settings search engine
    #[derive(Clone)]
    struct SettingsSearchItem {
        category: &'static str,
        label: &'static str,
        description: &'static str,
        tab_id: &'static str,
    }

    const SEARCH_ITEMS: &[SettingsSearchItem] = &[
        // Appearance
        SettingsSearchItem { category: "APPEARANCE", label: "Auto hide/show", description: "Toggle notch auto-hiding when idle", tab_id: "appearance" },
        SettingsSearchItem { category: "APPEARANCE", label: "Idle Pill Personality", description: "Configure date, time, clock and custom name", tab_id: "appearance" },
        SettingsSearchItem { category: "APPEARANCE", label: "Corner Smoothing", description: "Adjust notch border radius and corner curvature", tab_id: "appearance" },
        SettingsSearchItem { category: "APPEARANCE", label: "Notch Style / Mode", description: "Toggle Transparent vs. Solid opacity modes", tab_id: "appearance" },
        SettingsSearchItem { category: "APPEARANCE", label: "Notch Opacity", description: "Adjust transparency percentage slider", tab_id: "appearance" },
        // Layout
        SettingsSearchItem { category: "LAYOUT & DESIGN", label: "Idle Width", description: "Set rest-state pill width", tab_id: "layout" },
        SettingsSearchItem { category: "LAYOUT & DESIGN", label: "Idle Height", description: "Set rest-state pill height", tab_id: "layout" },
        // Tab Visibility
        SettingsSearchItem { category: "TAB VISIBILITY", label: "Home Tab Visibility", description: "Show or hide the main home dashboard", tab_id: "tabs" },
        SettingsSearchItem { category: "TAB VISIBILITY", label: "Media Tab Visibility", description: "Show or hide the media controls panel", tab_id: "tabs" },
        SettingsSearchItem { category: "TAB VISIBILITY", label: "Calendar Tab Visibility", description: "Show or hide the live calendar display", tab_id: "tabs" },
        SettingsSearchItem { category: "TAB VISIBILITY", label: "Clock Tab Visibility", description: "Show or hide the clock and stopwatch", tab_id: "tabs" },
        SettingsSearchItem { category: "TAB VISIBILITY", label: "File Shelf Visibility", description: "Show or hide the drag and drop shelf", tab_id: "tabs" },
        SettingsSearchItem { category: "TAB VISIBILITY", label: "Stats Panel Visibility", description: "Show or hide system CPU/RAM monitoring", tab_id: "tabs" },
        SettingsSearchItem { category: "TAB VISIBILITY", label: "Caffeine Toggle Visibility", description: "Show or hide caffeine screen stay-awake control", tab_id: "tabs" },
        SettingsSearchItem { category: "TAB VISIBILITY", label: "Settings Cog Visibility", description: "Show or hide settings access point", tab_id: "tabs" },
        SettingsSearchItem { category: "TAB VISIBILITY", label: "Battery Status Visibility", description: "Show or hide battery percentage metrics", tab_id: "tabs" },
        // Sounds
        SettingsSearchItem { category: "SOUNDS", label: "System Sounds", description: "Enable/disable global Raven audio feedback", tab_id: "sounds" },
        SettingsSearchItem { category: "SOUNDS", label: "Screen Unlock", description: "Welcoming sound on PC unlock", tab_id: "sounds" },
        SettingsSearchItem { category: "SOUNDS", label: "Battery Low", description: "Play sound when battery drops below threshold", tab_id: "sounds" },
        SettingsSearchItem { category: "SOUNDS", label: "Charger Connected", description: "Play audio feedback on plug insertion", tab_id: "sounds" },
        SettingsSearchItem { category: "SOUNDS", label: "Charger Disconnected", description: "Play audio feedback on charger removal", tab_id: "sounds" },
        SettingsSearchItem { category: "SOUNDS", label: "Caps Lock Sounds", description: "Acoustic feedback on Caps Lock state shifts", tab_id: "sounds" },
        // Calendar
        SettingsSearchItem { category: "LIVE CALENDAR", label: "Calendar url", description: "Specify third-party ICS integration path", tab_id: "calendar" },
        SettingsSearchItem { category: "LIVE CALENDAR", label: "Calendar Geometry", description: "Adjust calendar width and height metrics", tab_id: "calendar" },
        // Shelf & Share
        SettingsSearchItem { category: "SHELF & SHARE", label: "File Drop Shelf", description: "Toggle the quick storage shelf", tab_id: "shelf" },
        SettingsSearchItem { category: "SHELF & SHARE", label: "Auto Expand Shelf", description: "Automatically expand on drag insertion", tab_id: "shelf" },
        SettingsSearchItem { category: "SHELF & SHARE", label: "Open after drop", description: "Reveal files folder when sharing completes", tab_id: "shelf" },
        SettingsSearchItem { category: "SHELF & SHARE", label: "Keep Max Files", description: "Limit number of items preserved in shelf cache", tab_id: "shelf" },
        SettingsSearchItem { category: "SHELF & SHARE", label: "Default Provider", description: "Select between LocalSend and KDE Connect", tab_id: "shelf" },
        // Shortcuts
        SettingsSearchItem { category: "KEYBOARD SHORTCUTS", label: "Shortcut bindings", description: "Set triggers for screenshot, notch display, tab jumps, and media", tab_id: "shortcuts" },
        // Intelligence & Alerts
        SettingsSearchItem { category: "INTELLIGENCE & ALERTS", label: "Low Battery Alert", description: "Monitor battery levels and trigger alerts", tab_id: "intelligence" },
        SettingsSearchItem { category: "INTELLIGENCE & ALERTS", label: "Always On Charging", description: "Keep notch visible while charging", tab_id: "intelligence" },
        SettingsSearchItem { category: "INTELLIGENCE & ALERTS", label: "Always On Charging Mode", description: "Select between bolt, percentage, and battery outlines", tab_id: "intelligence" },
        SettingsSearchItem { category: "INTELLIGENCE & ALERTS", label: "Lock screen unlock alert", description: "Animated open-padlock overlay warning on login", tab_id: "intelligence" },
        SettingsSearchItem { category: "INTELLIGENCE & ALERTS", label: "Caps Lock monitor", description: "Visual notification overlay on keys change", tab_id: "intelligence" },
        SettingsSearchItem { category: "INTELLIGENCE & ALERTS", label: "Caffeine toggle monitor", description: "Visual alert notification on caffeine toggling", tab_id: "intelligence" },
        SettingsSearchItem { category: "INTELLIGENCE & ALERTS", label: "Camera active monitor", description: "Webcam surveillance privacy dot dynamic overlay", tab_id: "intelligence" },
        // Advanced
        SettingsSearchItem { category: "ADVANCED", label: "Keep apps below Notch", description: "Reserve screen area to prevent overlap", tab_id: "advanced" },
        SettingsSearchItem { category: "ADVANCED", label: "Full horizontal bar", description: "Simulate solid top-bar bezel integration", tab_id: "advanced" },
        SettingsSearchItem { category: "ADVANCED", label: "Reserved Height", description: "Set vertical height of screen reservation", tab_id: "advanced" },
        // About
        SettingsSearchItem { category: "ABOUT", label: "Website", description: "Visit the Raven Notch home page", tab_id: "system" },
        SettingsSearchItem { category: "ABOUT", label: "Changelog", description: "Read about version release history", tab_id: "system" },
        SettingsSearchItem { category: "ABOUT", label: "Send Feedback", description: "Open issue tracker and send feedback", tab_id: "system" },
    ];

    let settings_ui_weak_search = settings_ui.as_weak();
    settings_ui.on_search_changed(move |query| {
        if let Some(s_ui) = settings_ui_weak_search.upgrade() {
            if query.is_empty() {
                s_ui.set_search_results(std::rc::Rc::new(slint::VecModel::default()).into());
            } else {
                let results: Vec<SearchResult> = SEARCH_ITEMS
                    .iter()
                    .filter(|item| {
                        let q = query.to_lowercase();
                        item.label.to_lowercase().contains(&q)
                            || item.description.to_lowercase().contains(&q)
                            || item.category.to_lowercase().contains(&q)
                    })
                    .map(|item| SearchResult {
                        category: item.category.into(),
                        label: item.label.into(),
                        description: item.description.into(),
                        tab_id: item.tab_id.into(),
                    })
                    .collect();
                s_ui.set_search_results(std::rc::Rc::new(slint::VecModel::from(results)).into());
            }
        }
    });

    // Background thread to poll blocking Windows Media APIs
    let services_bg = services.clone();
    std::thread::spawn(move || {
        loop {
            services_bg.refresh_media();
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });

    // Background thread to poll/refresh Google/ICS Calendar
    let services_cal = services.clone();
    std::thread::spawn(move || {
        // Sleep for 3 seconds initially to allow the app window and thread pool to settle
        std::thread::sleep(std::time::Duration::from_secs(3));
        
        loop {
            println!("[CALENDAR-BG] Polling calendar events...");
            let snapshot = services_cal.refresh_calendar();
            println!("[CALENDAR-BG] Calendar events polled. Count: {}", snapshot.calendar.items.len());
            
            // Sleep for 15 minutes before the next refresh
            std::thread::sleep(std::time::Duration::from_secs(900));
        }
    });

    // Background thread to poll system stats every 1 second
    let services_stats = services.clone();
    let ui_stats_poll = ui.as_weak();
    std::thread::spawn(move || {
        loop {
            let active = ui_stats_poll
                .upgrade()
                .map(|ui| {
                    ui.get_active_tab() == "stats"
                        || ui.get_show_topbar_stats_dropdown()
                        || crate::window::STATS_WIDGET_HWND.load(std::sync::atomic::Ordering::SeqCst) != 0
                })
                .unwrap_or(false);
            if active {
                services_stats.refresh_stats();
                std::thread::sleep(std::time::Duration::from_millis(1000));
            } else {
                services_stats.refresh_stats_light();
                std::thread::sleep(std::time::Duration::from_millis(3000));
            }
        }
    });

    // Background thread to track lock/unlock events. Single command per poll; never overlaps.
    let ui_weak_unlock = ui.as_weak();
    std::thread::spawn(move || {
        use std::os::windows::process::CommandExt;
        use std::process::Command;

        let mut last_was_locked = false;
        loop {
            let mut cmd = Command::new("tasklist");
            cmd.args(&["/FI", "IMAGENAME eq LogonUI.exe", "/NH"]);
            cmd.creation_flags(0x08000000);

            let is_locked = if let Ok(out) = cmd.output() {
                String::from_utf8_lossy(&out.stdout).contains("LogonUI.exe")
            } else {
                false
            };

            if is_locked {
                last_was_locked = true;
            } else if last_was_locked {
                // PC was just unlocked!
                last_was_locked = false;
                
                // Play sound
                play_sound_by_name("unlock");

                // Trigger visual alert
                let ui_weak_clear = ui_weak_unlock.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak_clear.upgrade() {
                        let settings_current = settings::RavenSettings::load();
                        if settings_current.raven_alert.enabled && settings_current.raven_alert.monitor_unlock {
                            ui.set_active_alert_type("unlock".into());
                            let ui_clear = ui.as_weak();
                            let duration = settings_current.raven_alert.duration as u64;
                            slint::Timer::single_shot(std::time::Duration::from_millis(duration), move || {
                                if let Some(ui) = ui_clear.upgrade() {
                                    if ui.get_active_alert_type() == "unlock" {
                                        ui.set_active_alert_type("".into());
                                    }
                                }
                            });
                        }
                    }
                });
            }

            let sleep_ms = if is_locked { 500 } else { 5000 };
            std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
        }
    });
    // Background thread to track camera activity. Slow single-flight fallback; no process pile-up.
    let ui_weak_camera = ui.as_weak();
    std::thread::spawn(move || {
        use std::os::windows::process::CommandExt;
        use std::process::Command;

        let mut last_cam_active = false;
        loop {
            let mut cmd = Command::new("powershell");
            cmd.args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
                r#"Get-ChildItem -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\webcam' -Recurse | Get-ItemProperty | Where-Object { $_.LastUsedTimeStop -eq 0 } | Select-Object -First 1"#]);
            cmd.creation_flags(0x08000000);

            let has_cam = if let Ok(out) = cmd.output() {
                !String::from_utf8_lossy(&out.stdout).trim().is_empty()
            } else {
                false
            };

            if has_cam != last_cam_active {
                last_cam_active = has_cam;
                let ui_weak_cam_clone = ui_weak_camera.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak_cam_clone.upgrade() {
                        ui.set_camera_active(has_cam);
                    }
                });
            }

            std::thread::sleep(std::time::Duration::from_secs(12));
        }
    });

    let _hidden_window = window::NativeWindow::create_hidden(settings.clone(), services.clone()).unwrap();

    // Live Clock & Battery Timer
    let ui_handle = ui.as_weak();
    let battery_widget_for_timer = battery_widget.clone();
    let calendar_focus_widget_for_timer = calendar_focus_widget.clone();
    let focus_timer_runtime_for_timer = focus_timer_runtime.clone();
    let clock_timer = slint::Timer::default();

    // 5-second timer: keep settings_ui google state in sync with snapshot
    let settings_ui_google_sync = settings_ui.as_weak();
    let services_gs = services.clone();
    let google_sync_timer = slint::Timer::default();
    google_sync_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(5), move || {
        if let Some(s_ui) = settings_ui_google_sync.upgrade() {
            if !s_ui.get_google_busy() {
                let snap = services_gs.snapshot();
                s_ui.set_google_connected(snap.calendar.google_connected);
                s_ui.set_google_email(snap.calendar.google_email.clone().into());

                let settings = settings::RavenSettings::load();
                let selected_ids = settings.media.google_calendar_ids.clone();
                let slint_cals = map_google_calendars(&snap.calendar.google_calendars, &selected_ids);
                let selected_count = if selected_ids.is_empty() && snap.calendar.google_connected {
                    1
                } else {
                    selected_ids.len()
                };
                s_ui.set_google_calendars(std::rc::Rc::new(slint::VecModel::from(slint_cals)).into());
                s_ui.set_google_selected_calendars_count(selected_count as i32);
            }
        }
    });

    // State for second-by-second media progress tracking
    let media_last_update = std::sync::Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
    let media_last_title = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let media_last_artist = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let media_smoothed_position = std::sync::Arc::new(std::sync::Mutex::new(0.0_f64));

    let clock_last_update = media_last_update.clone();
    let clock_last_title = media_last_title.clone();
    let clock_last_artist = media_last_artist.clone();
    let clock_smoothed = media_smoothed_position.clone();

    let drag_scale_state = std::rc::Rc::new(std::cell::RefCell::new(DragScaleState {
        is_dragging: false,
        is_left_edge: false,
        start_mouse_x: 0,
        start_scale: 1.0,
        start_rect: windows::Win32::Foundation::RECT::default(),
    }));
    let drag_move_state = std::rc::Rc::new(std::cell::RefCell::new(DragMoveState {
        is_dragging: false,
        start_mouse_x: 0,
        start_mouse_y: 0,
        start_rect: windows::Win32::Foundation::RECT::default(),
    }));
    let drag_scale_state_timer = drag_scale_state.clone();
    let drag_move_state_timer = drag_move_state.clone();

    // Fast 30ms Live Update Timer for Stopwatch/Timer & Smooth Media Progress
    let ui_live_handle = ui.as_weak();
    let live_services = services.clone();
    let live_timer = slint::Timer::default();
    let mut prev_timer_running = false;
    let mut prev_stopwatch_running = false;
    let mut prev_focus_running = false;
    let mut prev_focus_paused = false;
    let mut glow_pulse_phase = 0.0f32;

    let focus_completed_shown_timer = focus_completed_shown.clone();
    let focus_bar_hidden_by_user_timer = focus_bar_hidden_by_user.clone();
    let focus_bar_window_timer = focus_bar_window.clone();
    let focus_completion_overlay_window_timer = focus_completion_overlay_window.clone();

    live_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(30), move || {
        if let Some(ui) = ui_live_handle.upgrade() {
            let clock = live_services.clock.read();
            let timer_running = clock.timer_running;
            let stopwatch_running = clock.stopwatch_running;
            let focus_running = clock.focus_running;
            
            // Check for timer completion (transition from running to not-running and remaining is 0)
            if prev_timer_running && !timer_running && clock.timer_remaining_seconds == 0 {
                play_sound_by_name("timer_complete");
            }
            
            // Check for focus timer completion (transition from active to not-active and remaining is 0)
            let prev_focus_active = prev_focus_running || prev_focus_paused;
            let focus_active = focus_running || clock.focus_paused;
            if prev_focus_active && !focus_active && (clock.focus_remaining_seconds == 0 || clock.focus_no_limit) {
                play_sound_by_name("timer_complete");
                // Save completed session to settings
                let local_time = chrono::Local::now();
                let completed_at_str = local_time.format("%b %d, %H:%M").to_string();
                let duration_mins = if clock.focus_no_limit {
                    (clock.focus_remaining_seconds / 60) as i32
                } else {
                    (clock.focus_duration_seconds / 60) as i32
                };
                let updated_settings = crate::settings::add_focus_session(
                    &clock.focus_goal,
                    duration_mins,
                    &completed_at_str
                );
                // Update Slint history model
                let slint_history: Vec<SlintFocusSession> = updated_settings.focus_sessions.iter().map(|s| SlintFocusSession {
                    goal: s.goal.clone().into(),
                    duration_mins: s.duration_mins,
                    completed_at: s.completed_at.clone().into(),
                }).collect();
                ui.set_focus_session_history(std::rc::Rc::new(slint::VecModel::from(slint_history)).into());
                ui.set_focus_session_state("history".into());
                *focus_completed_shown_timer.borrow_mut() = true;
            }

            let focus_progress = if clock.focus_no_limit {
                0.0
            } else if clock.focus_duration_seconds > 0 {
                let remaining = clock.focus_remaining_seconds as f32;
                let duration = clock.focus_duration_seconds as f32;
                ((duration - remaining) / duration).clamp(0.0, 1.0)
            } else {
                0.0
            };
            ui.set_focus_session_progress(focus_progress);

            let (timer_remaining_val, timer_running_val, timer_progress_val) = if focus_running {
                (clock.focus_remaining_label(), true, focus_progress)
            } else {
                let timer_progress = if clock.timer_duration_seconds > 0 {
                    let remaining = clock.timer_remaining_seconds as f32;
                    let duration = clock.timer_duration_seconds as f32;
                    ((duration - remaining) / duration).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (clock.timer_label(), timer_running, timer_progress)
            };

            ui.set_timer_remaining_str(timer_remaining_val.into());
            ui.set_timer_running(timer_running_val);
            ui.set_timer_progress(timer_progress_val);

            ui.set_stopwatch_str(clock.stopwatch_label().into());
            ui.set_stopwatch_running(stopwatch_running);

            ui.set_focus_session_running(focus_running);
            ui.set_focus_session_paused(clock.focus_paused);
            ui.set_focus_bar_hidden(*focus_bar_hidden_by_user_timer.borrow());
            ui.set_focus_session_remaining_str(clock.focus_remaining_label().into());

            // Manage floating focus session status bar
            let focus_completed = *focus_completed_shown_timer.borrow();
            let is_active_focus = (focus_running || clock.focus_paused)
                && !*focus_bar_hidden_by_user_timer.borrow();
            
            if is_active_focus {
                let mut bar_ref = focus_bar_window_timer.borrow_mut();
                if bar_ref.is_none() {
                    if let Ok(w) = FocusStatusBarWindow::new() {
                        // Show briefly to force HWND creation, then immediately apply Win32 styles
                        // BEFORE any paint occurs so it never flashes as a plain window
                        let _ = w.show();
                        if let Some(hwnd_val) = slint_component_hwnd(&w) {
                            unsafe {
                                use windows::Win32::Foundation::HWND;
                                use windows::Win32::UI::WindowsAndMessaging::{
                                    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos,
                                    GWL_EXSTYLE, GWL_STYLE,
                                    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_NOACTIVATE,
                                    WS_EX_TRANSPARENT, WS_EX_APPWINDOW, WS_EX_LAYERED,
                                    WS_POPUP, WS_CAPTION, WS_THICKFRAME, WS_MINIMIZEBOX,
                                    WS_MAXIMIZEBOX, WS_SYSMENU, WS_DLGFRAME,
                                    SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_FRAMECHANGED,
                                    SW_HIDE, ShowWindow,
                                };
                                let hwnd = HWND(hwnd_val as _);

                                // Strip frame/caption, add toolwindow + topmost + noactivate
                                let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                                let new_ex = (ex & !WS_EX_TRANSPARENT.0 & !WS_EX_APPWINDOW.0)
                                    | WS_EX_TOOLWINDOW.0
                                    | WS_EX_TOPMOST.0
                                    | WS_EX_NOACTIVATE.0
                                    | WS_EX_LAYERED.0;
                                let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex as isize);

                                let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
                                let new_style = (style | WS_POPUP.0)
                                    & !(WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0
                                        | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0 | WS_DLGFRAME.0);
                                let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);

                                // Apply frame change then immediately hide — no flash
                                let _ = SetWindowPos(
                                    hwnd, HWND(-1), 0, 0, 0, 0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                                );
                                let _ = ShowWindow(hwnd, SW_HIDE);
                            }
                        }
                        let _ = w.hide();

                        let drag_move_state_c = drag_move_state_timer.clone();
                        let w_weak = w.as_weak();
                        w.on_drag_move_start(move || {
                            if let Some(w) = w_weak.upgrade() {
                                if let Some(hwnd_val) = slint_component_hwnd(&w) {
                                    unsafe {
                                        let hwnd = windows::Win32::Foundation::HWND(hwnd_val as _);
                                        let mut rect = windows::Win32::Foundation::RECT::default();
                                        if windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect).is_ok() {
                                            let mut pt = windows::Win32::Foundation::POINT::default();
                                            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
                                            
                                            let mut state = drag_move_state_c.borrow_mut();
                                            state.is_dragging = true;
                                            state.start_mouse_x = pt.x;
                                            state.start_mouse_y = pt.y;
                                            state.start_rect = rect;
                                            
                                            windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
                                        }
                                    }
                                }
                            }
                        });

                        let s_pause = live_services.clone();
                        w.on_pause_toggle(move || {
                            s_pause.clock.toggle_pause_focus_session();
                        });

                        let s_complete = live_services.clone();
                        w.on_complete_session(move || {
                            s_complete.clock.complete_focus_session();
                        });

                        let focus_completed_shown_c = focus_completed_shown_timer.clone();
                        let focus_bar_hidden_by_user_c = focus_bar_hidden_by_user_timer.clone();
                        let focus_completion_overlay_window_c = focus_completion_overlay_window_timer.clone();
                        let ui_weak = ui_live_handle.clone();
                        w.on_dismiss(move || {
                            *focus_completed_shown_c.borrow_mut() = false;
                            *focus_bar_hidden_by_user_c.borrow_mut() = true;
                            if let Some(overlay) = focus_completion_overlay_window_c.borrow_mut().take() {
                                let _ = overlay.hide();
                            }
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_focus_bar_hidden(true);
                            }
                        });

                        let drag_scale_state_c = drag_scale_state_timer.clone();
                        let w_weak = w.as_weak();
                        w.on_drag_scale_start(move |is_left_edge| {
                            if let Some(w) = w_weak.upgrade() {
                                if let Some(hwnd_val) = slint_component_hwnd(&w) {
                                    unsafe {
                                        let hwnd = windows::Win32::Foundation::HWND(hwnd_val as _);
                                        let mut rect = windows::Win32::Foundation::RECT::default();
                                        if windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect).is_ok() {
                                            let mut pt = windows::Win32::Foundation::POINT::default();
                                            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
                                            
                                            let mut state = drag_scale_state_c.borrow_mut();
                                            state.is_dragging = true;
                                            state.is_left_edge = is_left_edge;
                                            state.start_mouse_x = pt.x;
                                            state.start_scale = w.get_scale_val();
                                            state.start_rect = rect;
                                            
                                            windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
                                        }
                                    }
                                }
                            }
                        });

                        let w_weak = w.as_weak();
                        FOCUS_BAR_WEAK.with(|cell| {
                            *cell.borrow_mut() = Some(w_weak);
                        });

                        w.set_dropped_down(true);
                        *bar_ref = Some((w, FocusBarConfig::Uninitialized));
                    }
                }

                if let Some((w, config_state)) = bar_ref.as_mut() {
                    if let Some(hwnd) = slint_component_hwnd(w) {
                        let mut pt = windows::Win32::Foundation::POINT::default();
                        unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt); }

                        // Resizing
                        let mut drag_state = drag_scale_state_timer.borrow_mut();
                        if drag_state.is_dragging {
                            let lbutton_down = unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x01) } as u16 & 0x8000 != 0;
                            if !lbutton_down {
                                drag_state.is_dragging = false;
                                unsafe { let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture(); }
                            } else {
                                let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(windows::Win32::Foundation::HWND(hwnd as _)) };
                                let dpi_scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
                                
                                let delta_x = pt.x - drag_state.start_mouse_x;
                                let delta_w = if drag_state.is_left_edge { -delta_x } else { delta_x };
                                
                                let delta_scale = delta_w as f32 / (420.0 * dpi_scale);
                                let new_scale = (drag_state.start_scale + delta_scale).clamp(1.0, 1.8);
                                
                                w.set_scale_val(new_scale);
                                
                                let phys_w = (420.0 * new_scale * dpi_scale) as i32;
                                let phys_h = (84.0 * new_scale * dpi_scale) as i32;
                                
                                let x = drag_state.start_rect.left + if drag_state.is_left_edge { delta_x } else { 0 };
                                let y = drag_state.start_rect.top;
                                
                                unsafe {
                                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                                        windows::Win32::Foundation::HWND(hwnd as _),
                                        windows::Win32::Foundation::HWND(-1),
                                        x, y, phys_w, phys_h,
                                        windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE |
                                        windows::Win32::UI::WindowsAndMessaging::SWP_FRAMECHANGED
                                    );
                                }
                            }
                        }

                        // Moving
                        let mut drag_move = drag_move_state_timer.borrow_mut();
                        if drag_move.is_dragging {
                            let lbutton_down = unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x01) } as u16 & 0x8000 != 0;
                            if !lbutton_down {
                                drag_move.is_dragging = false;
                                unsafe { let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture(); }
                            } else {
                                let delta_x = pt.x - drag_move.start_mouse_x;
                                let delta_y = pt.y - drag_move.start_mouse_y;
                                
                                let x = drag_move.start_rect.left + delta_x;
                                let y = drag_move.start_rect.top + delta_y;
                                
                                unsafe {
                                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                                        windows::Win32::Foundation::HWND(hwnd as _),
                                        windows::Win32::Foundation::HWND(-1),
                                        x, y, 0, 0,
                                        windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE |
                                        windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE
                                    );
                                }
                            }
                        }

                        match config_state {
                            FocusBarConfig::Uninitialized => {
                                if focus_completed {
                                    let _ = w.hide();
                                    *config_state = FocusBarConfig::Completed;
                                } else {
                                    let scale = w.get_scale_val();
                                    configure_focus_bar_hwnd(hwnd, scale);
                                    // Tell Slint the window is visible so it renders content
                                    let _ = w.show();
                                    *config_state = FocusBarConfig::Normal;
                                }
                            }
                            FocusBarConfig::Normal => {
                                if focus_completed {
                                    let _ = w.hide();
                                    *config_state = FocusBarConfig::Completed;
                                }
                            }
                            FocusBarConfig::Completed => {
                                if !focus_completed {
                                    let scale = w.get_scale_val();
                                    configure_focus_bar_hwnd(hwnd, scale);
                                    let _ = w.show();
                                    *config_state = FocusBarConfig::Normal;
                                }
                            }
                        }
                    }
                    w.set_goal_text(clock.focus_goal.clone().into());
                    w.set_timer_str(clock.focus_remaining_label().into());
                    w.set_timer_progress(focus_progress);
                    w.set_is_paused(clock.focus_paused);
                    w.set_is_completed(false);
                }
            } else {
                let mut bar_ref = focus_bar_window_timer.borrow_mut();
                if let Some((w, _)) = bar_ref.take() {
                    let _ = w.hide();
                    FOCUS_BAR_WEAK.with(|cell| {
                        *cell.borrow_mut() = None;
                    });
                }
            }

            if focus_completed {
                let mut overlay_ref = focus_completion_overlay_window_timer.borrow_mut();
                if overlay_ref.is_none() {
                    if let Ok(overlay) = FocusCompletionOverlayWindow::new() {
                        overlay.set_goal_text(clock.focus_goal.clone().into());
                        let focus_completed_shown_c = focus_completed_shown_timer.clone();
                        let focus_bar_hidden_by_user_c = focus_bar_hidden_by_user_timer.clone();
                        let ui_weak = ui_live_handle.clone();
                        overlay.on_dismiss(move || {
                            *focus_completed_shown_c.borrow_mut() = false;
                            *focus_bar_hidden_by_user_c.borrow_mut() = true;
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_focus_bar_hidden(true);
                            }
                        });

                        let _ = overlay.show();
                        overlay_ref.replace(overlay);
                    }
                }

                if let Some(overlay) = overlay_ref.as_ref() {
                    overlay.set_goal_text(clock.focus_goal.clone().into());
                    if let Some(overlay_hwnd) = slint_component_hwnd(overlay) {
                        unsafe {
                            use windows::Win32::Foundation::{POINT, RECT};
                            use windows::Win32::Graphics::Gdi::{
                                GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
                            };
                            use windows::Win32::UI::WindowsAndMessaging::{
                                GetCursorPos, GetWindowLongPtrW, GetWindowRect, GWL_EXSTYLE,
                                WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
                            };

                            let mut pt = POINT::default();
                            let _ = GetCursorPos(&mut pt);
                            let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
                            let mut info = MONITORINFO {
                                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                ..Default::default()
                            };
                            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                                let x = info.rcMonitor.left;
                                let y = info.rcMonitor.top;
                                let width = info.rcMonitor.right - info.rcMonitor.left;
                                let height = info.rcMonitor.bottom - info.rcMonitor.top;
                                let slint_scale = overlay.window().scale_factor().max(0.1);

                                overlay.window().set_position(slint::PhysicalPosition::new(x, y));
                                overlay.set_overlay_width(width as f32 / slint_scale);
                                overlay.set_overlay_height(height as f32 / slint_scale);

                                let mut rect = RECT::default();
                                let needs_position = GetWindowRect(
                                    windows::Win32::Foundation::HWND(overlay_hwnd as _),
                                    &mut rect,
                                )
                                .is_err()
                                    || rect.left != x
                                    || rect.top != y
                                    || rect.right - rect.left != width
                                    || rect.bottom - rect.top != height;

                                let ex_style = GetWindowLongPtrW(
                                    windows::Win32::Foundation::HWND(overlay_hwnd as _),
                                    GWL_EXSTYLE,
                                ) as u32;
                                let needs_style = (ex_style & WS_EX_APPWINDOW.0) != 0
                                    || (ex_style & WS_EX_TOOLWINDOW.0) == 0
                                    || (ex_style & WS_EX_TOPMOST.0) == 0
                                    || (ex_style & WS_EX_TRANSPARENT.0) != 0;

                                if needs_position || needs_style {
                                    configure_floating_overlay_hwnd(
                                        overlay_hwnd,
                                        x,
                                        y,
                                        width,
                                        height,
                                        false,
                                    );
                                    let _ = overlay.show();
                                }
                            }
                        }
                    }
                }
            } else if let Some(overlay) = focus_completion_overlay_window_timer.borrow_mut().take() {
                let _ = overlay.hide();
            }

            if focus_completed {
                glow_pulse_phase = (glow_pulse_phase + 0.06) % (2.0 * std::f32::consts::PI);
                let is_glowing = glow_pulse_phase.sin() > 0.0;
                let overlay_ref = focus_completion_overlay_window_timer.borrow();
                if let Some(overlay) = overlay_ref.as_ref() {
                    overlay.set_is_glowing(is_glowing);
                }
            }

            prev_timer_running = timer_running;
            prev_stopwatch_running = stopwatch_running;
            prev_focus_running = focus_running;
            prev_focus_paused = clock.focus_paused;
        }
    });
    
    // Initialize image load cache variables
    let mut last_source_icon_path = String::new();
    let mut session_icon_cache = std::collections::HashMap::<String, slint::Image>::new();

    // Initialize stats history queues (20 data points) and a tick counter
    let mut cpu_history = std::collections::VecDeque::from(vec![0.0f32; 20]);
    let mut ram_history = std::collections::VecDeque::from(vec![0.0f32; 20]);
    let mut gpu_history = std::collections::VecDeque::from(vec![0.0f32; 20]);
    let mut tick_count = 0u64;

    let mut last_album_art_path = String::new();
    let mut last_adaptive_accent = true;
    let mut last_settings_accent_color = String::new();
    let mut last_ui_has_media: Option<bool> = None;
    let mut last_ui_media_title = String::new();
    let mut last_ui_media_artist = String::new();
    let mut last_ui_media_album = String::new();
    let mut last_ui_media_source_id = String::new();
    let mut last_ui_media_source = String::new();
    let mut last_ui_is_playing: Option<bool> = None;
    let mut last_ui_waveform_active: Option<bool> = None;
    let mut last_ui_media_pct = -1.0_f32;
    let mut last_ui_media_pos_str = String::new();
    let mut last_ui_media_dur_str = String::new();
    let mut last_media_sessions_signature = String::new();
    let mut last_extra_widget_position_save = std::time::Instant::now() - std::time::Duration::from_secs(10);
    let mut last_lyrics_lines_signature = String::new();
    let mut last_lyrics_active_index: Option<i32> = None;
    let mut last_lyrics_has: Option<bool> = None;
    let mut last_lyrics_status = String::new();

    let services_cloned = services.clone();
    let mut prev_charging: Option<bool> = None;
    let mut prev_battery_low: Option<bool> = None;
    let mut prev_caps_lock: Option<bool> = None;

    struct CalendarState {
        selected_day_index: i32,
        master_events: Vec<services::CalendarEvent>,
        last_today: Option<chrono::NaiveDate>,
        center_date: chrono::NaiveDate,
    }
    let calendar_state = std::sync::Arc::new(std::sync::Mutex::new(CalendarState {
        selected_day_index: 180,
        master_events: Vec::new(),
        last_today: None,
        center_date: chrono::Local::now().date_naive(),
    }));

    let ui_select_weak = ui.as_weak();
    let calendar_state_select = calendar_state.clone();
    ui.on_select_day(move |idx| {
        if let Some(ui) = ui_select_weak.upgrade() {
            let mut state = calendar_state_select.lock().unwrap();
            state.selected_day_index = idx;
            ui.set_selected_day_index(idx);
            update_calendar_ui(&ui, idx, &state.master_events, false, state.center_date);
        }
    });

    let ui_dbclick_weak = ui.as_weak();
    let calendar_state_dbclick = calendar_state.clone();
    ui.on_month_year_double_clicked(move || {
        if let Some(ui) = ui_dbclick_weak.upgrade() {
            let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
            static LAST_CLICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let prev = LAST_CLICK.swap(now_ms, std::sync::atomic::Ordering::SeqCst);
            if now_ms - prev < 350 {
                use chrono::Datelike;
                let state = calendar_state_dbclick.lock().unwrap();
                let view_date = state.center_date;
                
                ui.set_show_mini_calendar(true);
                ui.set_picker_mode("days".into());
                ui.set_mini_calendar_view_month(view_date.month() as i32);
                ui.set_mini_calendar_view_year(view_date.year());
                
                let mini_days = generate_mini_calendar_days(view_date.year(), view_date.month() as i32, view_date);
                ui.set_mini_calendar_days(std::rc::Rc::new(slint::VecModel::from(mini_days)).into());
                
                let mini_years: Vec<i32> = (2020..=2035).collect();
                ui.set_mini_calendar_years(std::rc::Rc::new(slint::VecModel::from(mini_years)).into());
            }
        }
    });

    let ui_change_weak = ui.as_weak();
    let calendar_state_change = calendar_state.clone();
    ui.on_mini_calendar_change_month_year(move |month, year| {
        if let Some(ui) = ui_change_weak.upgrade() {
            let state = calendar_state_change.lock().unwrap();
            let selected_offset = (state.selected_day_index - 180) as i64;
            let selected_date = state.center_date.checked_add_signed(chrono::Duration::days(selected_offset)).unwrap_or(state.center_date);
            
            let mini_days = generate_mini_calendar_days(year, month, selected_date);
            ui.set_mini_calendar_days(std::rc::Rc::new(slint::VecModel::from(mini_days)).into());
        }
    });

    let ui_select_date_weak = ui.as_weak();
    let calendar_state_select_date = calendar_state.clone();
    ui.on_mini_calendar_select_date(move |cell_idx, month, year| {
        if let Some(ui) = ui_select_date_weak.upgrade() {
            use chrono::Datelike;
            let mut state = calendar_state_select_date.lock().unwrap();
            
            let first_day = chrono::NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap();
            let start_weekday_idx = match first_day.weekday() {
                chrono::Weekday::Sun => 0,
                chrono::Weekday::Mon => 1,
                chrono::Weekday::Tue => 2,
                chrono::Weekday::Wed => 3,
                chrono::Weekday::Thu => 4,
                chrono::Weekday::Fri => 5,
                chrono::Weekday::Sat => 6,
            };
            let clicked_day = cell_idx as usize - start_weekday_idx + 1;
            
            if let Some(target_date) = chrono::NaiveDate::from_ymd_opt(year, month as u32, clicked_day as u32) {
                state.center_date = target_date;
                state.selected_day_index = 180;
                ui.set_selected_day_index(180);
                
                update_calendar_ui(&ui, 180, &state.master_events, true, target_date);
                ui.set_show_mini_calendar(false);
            }
        }
    });

    let ui_close_weak = ui.as_weak();
    ui.on_mini_calendar_close(move || {
        if let Some(ui) = ui_close_weak.upgrade() {
            ui.set_show_mini_calendar(false);
        }
    });

    {
        let mut state = calendar_state.lock().unwrap();
        state.last_today = None;
        ui.set_selected_day_index(180);
        update_calendar_ui(&ui, 180, &state.master_events, true, state.center_date);
    }
    
    let calendar_state_tick = calendar_state.clone();
    let stats_widget_for_timer = stats_widget.clone();
    let year_progress_widget_for_timer = year_progress_widget.clone();
    let day_progress_widget_for_timer = day_progress_widget.clone();
    let month_progress_widget_for_timer = month_progress_widget.clone();
    let media_widget_for_timer = media_widget.clone();
    let notes_widget_for_timer = notes_widget.clone();
    let todo_widget_for_timer = todo_widget.clone();
    let quotes_widget_for_timer = quotes_widget.clone();
    let picture_widget_for_timer = picture_widget.clone();
    let instance_widgets_for_timer = instance_widgets.clone();
    let focus_runtime_for_extra_widgets = focus_timer_runtime.clone();
    let quotes_last_change = std::rc::Rc::new(std::cell::Cell::new(std::time::Instant::now()));
    let last_play_pause_click = std::rc::Rc::new(std::cell::Cell::new(None::<std::time::Instant>));
    let last_play_pause_click_for_clock = last_play_pause_click.clone();
    let clipboard_history: Rc<RefCell<Vec<ClipboardHistoryEntry>>> = Rc::new(RefCell::new(Vec::new()));
    let clipboard_next_id = Rc::new(std::cell::Cell::new(1_i32));
    let clipboard_last_text = Rc::new(RefCell::new(String::new()));
    let clipboard_paste_target = Rc::new(std::cell::Cell::new(0_isize));
    let clipboard_history_for_timer = clipboard_history.clone();
    let clipboard_next_id_for_timer = clipboard_next_id.clone();
    let clipboard_last_text_for_timer = clipboard_last_text.clone();
    let mut pending_media_paused_since: Option<std::time::Instant> = None;


    clock_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(1), move || {
        if let Some(ui) = ui_handle.upgrade() {
            let snapshot = services_cloned.snapshot();
            update_clock_display(&ui);

            // --- Set World Clock Properties ---
            let utc_now = chrono::Utc::now();
            
            // New York
            let (ny_offset, ny_offset_str) = if is_ny_dst(&utc_now) {
                (chrono::FixedOffset::west_opt(4 * 3600).unwrap(), "UTC -4")
            } else {
                (chrono::FixedOffset::west_opt(5 * 3600).unwrap(), "UTC -5")
            };
            let ny_time = utc_now.with_timezone(&ny_offset);
            ui.set_ny_time_str(ny_time.format("%I:%M").to_string().into());
            ui.set_ny_ampm_str(ny_time.format("%p").to_string().into());
            ui.set_ny_date_str(ny_time.format("%b %-d").to_string().into());
            ui.set_ny_hand_hour(ny_time.hour() as i32);
            ui.set_ny_hand_min(ny_time.minute() as i32);
            ui.set_ny_offset_str(ny_offset_str.into());

            // London
            let (ldn_offset, ldn_offset_str) = if is_ldn_dst(&utc_now) {
                (chrono::FixedOffset::east_opt(1 * 3600).unwrap(), "UTC +1")
            } else {
                (chrono::FixedOffset::east_opt(0).unwrap(), "UTC +0")
            };
            let ldn_time = utc_now.with_timezone(&ldn_offset);
            ui.set_ldn_time_str(ldn_time.format("%I:%M").to_string().into());
            ui.set_ldn_ampm_str(ldn_time.format("%p").to_string().into());
            ui.set_ldn_date_str(ldn_time.format("%b %-d").to_string().into());
            ui.set_ldn_hand_hour(ldn_time.hour() as i32);
            ui.set_ldn_hand_min(ldn_time.minute() as i32);
            ui.set_ldn_offset_str(ldn_offset_str.into());

            // Tokyo
            let tky_offset = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
            let tky_time = utc_now.with_timezone(&tky_offset);
            ui.set_tky_time_str(tky_time.format("%I:%M").to_string().into());
            ui.set_tky_ampm_str(tky_time.format("%p").to_string().into());
            ui.set_tky_date_str(tky_time.format("%b %-d").to_string().into());
            ui.set_tky_hand_hour(tky_time.hour() as i32);
            ui.set_tky_hand_min(tky_time.minute() as i32);
            ui.set_tky_offset_str("UTC +9".into());

            // New Delhi
            let del_offset = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
            let del_time = utc_now.with_timezone(&del_offset);
            ui.set_del_time_str(del_time.format("%I:%M").to_string().into());
            ui.set_del_ampm_str(del_time.format("%p").to_string().into());
            ui.set_del_date_str(del_time.format("%b %-d").to_string().into());
            ui.set_del_hand_hour(del_time.hour() as i32);
            ui.set_del_hand_min(del_time.minute() as i32);
            ui.set_del_offset_str("UTC +5:30".into());

            // Sydney
            let (syd_offset, syd_offset_str) = if is_syd_dst(&utc_now) {
                (chrono::FixedOffset::east_opt(11 * 3600).unwrap(), "UTC +11")
            } else {
                (chrono::FixedOffset::east_opt(10 * 3600).unwrap(), "UTC +10")
            };
            let syd_time = utc_now.with_timezone(&syd_offset);
            ui.set_syd_time_str(syd_time.format("%I:%M").to_string().into());
            ui.set_syd_ampm_str(syd_time.format("%p").to_string().into());
            ui.set_syd_date_str(syd_time.format("%b %-d").to_string().into());
            ui.set_syd_hand_hour(syd_time.hour() as i32);
            ui.set_syd_hand_min(syd_time.minute() as i32);
            ui.set_syd_offset_str(syd_offset_str.into());

            // Paris
            let (par_offset, par_offset_str) = if is_par_dst(&utc_now) {
                (chrono::FixedOffset::east_opt(2 * 3600).unwrap(), "UTC +2")
            } else {
                (chrono::FixedOffset::east_opt(1 * 3600).unwrap(), "UTC +1")
            };
            let par_time = utc_now.with_timezone(&par_offset);
            ui.set_par_time_str(par_time.format("%I:%M").to_string().into());
            ui.set_par_ampm_str(par_time.format("%p").to_string().into());
            ui.set_par_date_str(par_time.format("%b %-d").to_string().into());
            ui.set_par_hand_hour(par_time.hour() as i32);
            ui.set_par_hand_min(par_time.minute() as i32);
            ui.set_par_offset_str(par_offset_str.into());

            // Dubai
            let dxb_offset = chrono::FixedOffset::east_opt(4 * 3600).unwrap();
            let dxb_time = utc_now.with_timezone(&dxb_offset);
            ui.set_dxb_time_str(dxb_time.format("%I:%M").to_string().into());
            ui.set_dxb_ampm_str(dxb_time.format("%p").to_string().into());
            ui.set_dxb_date_str(dxb_time.format("%b %-d").to_string().into());
            ui.set_dxb_hand_hour(dxb_time.hour() as i32);
            ui.set_dxb_hand_min(dxb_time.minute() as i32);
            ui.set_dxb_offset_str("UTC +4".into());

            // Singapore
            let sin_offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            let sin_time = utc_now.with_timezone(&sin_offset);
            ui.set_sin_time_str(sin_time.format("%I:%M").to_string().into());
            ui.set_sin_ampm_str(sin_time.format("%p").to_string().into());
            ui.set_sin_date_str(sin_time.format("%b %-d").to_string().into());
            ui.set_sin_hand_hour(sin_time.hour() as i32);
            ui.set_sin_hand_min(sin_time.minute() as i32);
            ui.set_sin_offset_str("UTC +8".into());

            let settings_current = settings::RavenSettings::load();
            let trigger_visual_alert = |ui_ref: &Pill, alert_type: &str, alert_event: &str, duration_ms: u64| {
                ui_ref.set_active_alert_type(alert_type.into());
                ui_ref.set_active_alert_event(alert_event.into());
                if alert_type == "charger_in" || alert_type == "charger_out" || alert_type == "low_battery" {
                    crate::window::HOTKEY_OPENED.store(true, Ordering::SeqCst);
                    ui_ref.set_system_hud_kind(alert_type.into());
                    ui_ref.set_system_hud_value(ui_ref.get_battery_pct().round() as i32);
                    ui_ref.set_system_hud_muted(false);
                    ui_ref.invoke_request_notch_open();
                    crate::window::update_pill_window_layout();
                }
                let ui_clear = ui_ref.as_weak();
                let alert_type_owned = alert_type.to_string();
                slint::Timer::single_shot(std::time::Duration::from_millis(duration_ms), move || {
                    if let Some(ui) = ui_clear.upgrade() {
                        if ui.get_active_alert_type() == alert_type_owned.as_str() {
                            ui.set_active_alert_type("".into());
                            ui.set_active_alert_event("".into());
                            if ui.get_system_hud_kind() == alert_type_owned.as_str() {
                                ui.set_system_hud_kind("".into());
                                ui.invoke_request_notch_close();
                                crate::window::HOTKEY_OPENED.store(false, Ordering::SeqCst);
                                crate::window::update_pill_window_layout();
                            }
                        }
                    }
                });
            };

            // Update Open Apps list periodically (every 1 second)
            if ui.get_full_width_bar() && ui.get_top_bar_widgets() {
                let windows_list = crate::window::get_open_apps();
                let mut open_apps = Vec::new();
                for (hwnd, name) in windows_list {
                    let icon = crate::window::get_window_icon(hwnd).unwrap_or_else(|| slint::Image::default());
                    open_apps.push(crate::SlintOpenApp {
                        id: hwnd.0 as i32,
                        name: name.into(),
                        icon,
                    });
                }
                ui.set_open_apps(std::rc::Rc::new(slint::VecModel::from(open_apps)).into());
            }

            #[allow(unused_assignments)]
            let mut is_charging = false;
            #[allow(unused_assignments)]
            let mut is_battery_low = false;
            #[allow(unused_variables, unused_assignments)]
            let mut battery_pct = 100.0f32;
            // Get live battery percentage using Win32 API
            unsafe {
                let mut status = windows::Win32::System::Power::SYSTEM_POWER_STATUS::default();
                if windows::Win32::System::Power::GetSystemPowerStatus(&mut status).is_ok() {
                    let raw_pct = status.BatteryLifePercent;
                    let pct = if raw_pct == 255 { 100 } else { raw_pct.min(100) };
                    ui.set_battery_pct(pct as f32);
                    
                    is_charging = status.ACLineStatus == 1;
                    battery_pct = pct as f32;
                    let battery_present = status.BatteryFlag != 128 && status.BatteryFlag != 255 && status.BatteryLifePercent != 255;
                    is_battery_low = battery_present && battery_pct <= 20.0 && !is_charging;

                    // Play Charger Connected/Disconnected sound on transitions
                    if let Some(prev) = prev_charging {
                        if is_charging && !prev {
                            play_sound_by_name("charger_connected");
                            if settings_current.raven_alert.enabled && settings_current.raven_alert.monitor_charger_in {
                                trigger_visual_alert(&ui, "charger_in", "", settings_current.raven_alert.duration as u64);
                            }
                        } else if !is_charging && prev {
                            play_sound_by_name("charger_disconnected");
                            if settings_current.raven_alert.enabled && settings_current.raven_alert.monitor_charger_out {
                                trigger_visual_alert(&ui, "charger_out", "", settings_current.raven_alert.duration as u64);
                            }
                        }
                    }
                    prev_charging = Some(is_charging);

                    // Play Battery Low sound on transition
                    if let Some(prev_low) = prev_battery_low {
                        if is_battery_low && !prev_low {
                            play_sound_by_name("battery_low");
                            if settings_current.raven_alert.enabled && settings_current.raven_alert.monitor_low_battery {
                                trigger_visual_alert(&ui, "low_battery", "", settings_current.raven_alert.duration as u64);
                            }
                        }
                    }
                    prev_battery_low = Some(is_battery_low);

                    ui.set_is_charging(is_charging);
                    if let Some(w) = battery_widget_for_timer.borrow().as_ref() {
                        w.set_battery_pct(battery_pct);
                        w.set_is_charging(is_charging);
                        w.set_progress_ring_img(render_battery_progress_ring(battery_pct));
                    }
                }
            }

            let is_fs = crate::window::IS_FOREGROUND_FULLSCREEN.load(std::sync::atomic::Ordering::SeqCst);
            ui.set_auto_hide(settings_current.appearance.auto_hide || (settings_current.appearance.auto_hide_on_fullscreen && is_fs));

            if focus_timer_runtime_for_timer.tick() {
                play_sound_by_name("timer_complete");
            }
            if let Some(w) = calendar_focus_widget_for_timer.borrow().as_ref() {
                update_calendar_focus_widget_properties(w, &focus_timer_runtime_for_timer);
                w.set_is_locked(settings_current.widgets.locked);
                w.set_bg_opacity(settings_current.widgets.opacity);
                w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
            }

            // Update settings preview clock ticking
            if SETTINGS_WINDOW_OPEN.load(Ordering::SeqCst) {
                if let Some(weak_settings_ui) = crate::window::SETTINGS_UI_WEAK.get() {
                    if let Some(settings_ui) = weak_settings_ui.upgrade() {
                        update_settings_preview_clock(&settings_ui, &settings_current);
                    }
                }
            }

            // Get Caps Lock toggle state using Win32 API
            let caps_lock_on = unsafe { (windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x14) & 1) != 0 };
            if let Some(prev) = prev_caps_lock {
                if caps_lock_on && !prev {
                    play_sound_by_name("capslock_on");
                    if settings_current.raven_alert.enabled && settings_current.raven_alert.monitor_keys {
                        trigger_visual_alert(&ui, "keys", "locked", settings_current.raven_alert.duration as u64);
                    }
                } else if !caps_lock_on && prev {
                    play_sound_by_name("capslock_off");
                    if settings_current.raven_alert.enabled && settings_current.raven_alert.monitor_keys {
                        trigger_visual_alert(&ui, "keys", "unlocked", settings_current.raven_alert.duration as u64);
                    }
                }
            }
            prev_caps_lock = Some(caps_lock_on);
            
            // Increment tick counter for pseudo-random GPU simulation
            tick_count += 1;

            // Get live CPU and RAM stats from snapshot cache (updated by background thread)
            let cpu_usage = snapshot.cpu_pct;
            let ram_usage = snapshot.ram_pct;

            // GPU Simulation - dynamic compound wave with LCG deterministic noise
            let base_gpu = 25.0f32;
            let sine_wave = ((tick_count as f32 * 0.1).sin() * 12.0) + ((tick_count as f32 * 0.23).cos() * 6.0);
            let noise = ((tick_count % 7) as f32 - 3.0) * 1.5;
            let gpu_usage = (base_gpu + sine_wave + noise).clamp(10.0, 90.0);

            // Update stats history buffers
            cpu_history.pop_front();
            cpu_history.push_back(cpu_usage);

            ram_history.pop_front();
            ram_history.push_back(ram_usage);

            gpu_history.pop_front();
            gpu_history.push_back(gpu_usage);

            // Generate historical SVG wave paths and filled area data
            let (cpu_path, cpu_fill) = generate_svg_path(&cpu_history);
            let (ram_path, ram_fill) = generate_svg_path(&ram_history);
            let (gpu_path, gpu_fill) = generate_svg_path(&gpu_history);

            // Update Slint UI properties
            ui.set_cpu_pct(cpu_usage);
            ui.set_ram_pct(ram_usage);
            ui.set_gpu_pct(gpu_usage);
            ui.set_topbar_stats_ring_img(render_topbar_stats_ring(cpu_usage, ram_usage, gpu_usage));
            let process_rows: Vec<SlintProcessStat> = snapshot.process_stats.iter().map(|proc_stat| {
                SlintProcessStat {
                    name: proc_stat.name.clone().into(),
                    icon: crate::window::get_file_icon(&proc_stat.exe_path).unwrap_or_default(),
                    cpu_pct: proc_stat.cpu_pct,
                    ram_pct: proc_stat.ram_pct,
                    gpu_pct: proc_stat.gpu_pct,
                }
            }).collect();
            ui.set_process_stats(std::rc::Rc::new(slint::VecModel::from(process_rows)).into());

            ui.set_cpu_path_data(cpu_path.into());
            ui.set_cpu_fill_data(cpu_fill.into());

            ui.set_ram_path_data(ram_path.into());
            ui.set_ram_fill_data(ram_fill.into());

            ui.set_gpu_path_data(gpu_path.into());
            ui.set_gpu_fill_data(gpu_fill.into());

            if let Some(text) = read_clipboard_text() {
                let mut last = clipboard_last_text_for_timer.borrow_mut();
                if *last != text {
                    *last = text.clone();
                    let mut history = clipboard_history_for_timer.borrow_mut();
                    if !history.iter().any(|entry| entry.text == text) {
                        let id = clipboard_next_id_for_timer.get();
                        clipboard_next_id_for_timer.set(id + 1);
                        history.insert(0, ClipboardHistoryEntry {
                            id,
                            title: clipboard_title(&text),
                            text,
                            copied_at: chrono::Local::now(),
                            pinned: false,
                            selected: false,
                        });
                        if history.len() > 120 {
                            history.truncate(120);
                        }
                        refresh_clipboard_model(&ui, &history);
                    }
                }
            }



            // Caffeine
            ui.set_caffeine_enabled(snapshot.caffeine.enabled);

            // ── DESKTOP WIDGET ENGINE PROPERTY SYNCHRONIZATION ──
            
            // Stats widget sync — update ALL instances
            {
                let stats_guard = stats_widget_for_timer.borrow();
                if !stats_guard.is_empty() {
                    unsafe { save_current_widget_position_if_active(&stats_widget_for_timer); }
                    let time_now = chrono::Local::now();
                    for (idx, w) in stats_guard.iter().enumerate() {
                        let inst = settings_current.widgets.get_clock_instance(idx);
                        let mut time_fmt = if inst.show_cpu { "%H:%M" } else { "%I:%M" }.to_string();
                        if inst.show_ram { time_fmt.push_str(":%S"); }
                        let mut time_str = time_now.format(&time_fmt).to_string();
                        if !inst.show_cpu && time_str.starts_with('0') { time_str.remove(0); }
                        let ampm_str = if !inst.show_cpu && inst.show_battery {
                            time_now.format("%p").to_string().to_lowercase()
                        } else { "".to_string() };
                        let date_str = time_now.format("%A, %e %B").to_string();
                        
                        w.set_time_str(time_str.into());
                        w.set_ampm_str(ampm_str.into());
                        w.set_date_str(date_str.into());
                        w.set_is_locked(settings_current.widgets.locked);
                    }
                }
            }

            // Year Progress widget sync
            {
                let yp_guard = year_progress_widget_for_timer.borrow();
                if let Some(w) = yp_guard.as_ref() {
                    update_year_progress_widget_properties(w);
                    w.set_is_locked(settings_current.widgets.locked);
                    w.set_bg_opacity(settings_current.widgets.opacity);
                    w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
                }
            }

            // Day Progress widget sync
            {
                let dp_guard = day_progress_widget_for_timer.borrow();
                if let Some(w) = dp_guard.as_ref() {
                    update_day_progress_widget_properties(w);
                    w.set_is_locked(settings_current.widgets.locked);
                    w.set_bg_opacity(settings_current.widgets.opacity);
                    w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
                }
            }

            // Month Progress widget sync
            {
                let mp_guard = month_progress_widget_for_timer.borrow();
                if let Some(w) = mp_guard.as_ref() {
                    update_month_progress_widget_properties(w);
                    w.set_is_locked(settings_current.widgets.locked);
                    w.set_bg_opacity(settings_current.widgets.opacity);
                    w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
                }
            }

            // (Media widget sync moved to after local_pos is calculated)

            // Clock panel stopwatch/timer values updated via fast 30ms live_timer

            // Media metadata & state
            // ── When no media source is active, clear everything so the UI widget hides ──
            if !snapshot.media.has_media {
                if last_ui_has_media != Some(false) {
                    ui.set_media_title("".into());
                    ui.set_media_artist("".into());
                    ui.set_media_album("".into());
                    ui.set_media_source_id("".into());
                    ui.set_media_source("".into());
                    ui.set_has_media_art(false);
                    ui.set_has_media_source_icon(false);
                    ui.set_is_playing(false);
                    ui.set_media_waveform_active(false);
                    ui.set_media_pct(0.0);
                    ui.set_media_pos_str("0:00".into());
                    ui.set_media_dur_str("0:00".into());
                    last_ui_media_title.clear();
                    last_ui_media_artist.clear();
                    last_ui_media_album.clear();
                    last_ui_media_source_id.clear();
                    last_ui_media_source.clear();
                    last_ui_is_playing = Some(false);
                    last_ui_waveform_active = Some(false);
                    last_ui_media_pct = 0.0;
                    last_ui_media_pos_str = "0:00".to_string();
                    last_ui_media_dur_str = "0:00".to_string();
                    pending_media_paused_since = None;
                    last_ui_has_media = Some(false);
                }
            } else {
                last_ui_has_media = Some(true);
                if snapshot.media.title != last_ui_media_title {
                    last_ui_media_title = snapshot.media.title.clone();
                    ui.set_media_title(last_ui_media_title.clone().into());
                }
                if snapshot.media.artist != last_ui_media_artist {
                    last_ui_media_artist = snapshot.media.artist.clone();
                    ui.set_media_artist(last_ui_media_artist.clone().into());
                }
                if snapshot.media.album != last_ui_media_album {
                    last_ui_media_album = snapshot.media.album.clone();
                    ui.set_media_album(last_ui_media_album.clone().into());
                }
                if snapshot.media.source_id != last_ui_media_source_id {
                    last_ui_media_source_id = snapshot.media.source_id.clone();
                    ui.set_media_source_id(last_ui_media_source_id.clone().into());
                }
                if snapshot.media.source_id != last_ui_media_source {
                    last_ui_media_source = snapshot.media.source_id.clone();
                    ui.set_media_source(last_ui_media_source.clone().into());
                }
            }
            // println!("[DEBUG-MEDIA] SourceAppUserModelId = '{}'", snapshot.media.source_id);
            let is_in_playback_lockout = if snapshot.media.has_media {
                if let Some(last_click) = last_play_pause_click_for_clock.get() {
                    if last_click.elapsed() < std::time::Duration::from_millis(1500) {
                        if snapshot.media.is_playing == ui.get_is_playing() {
                            last_play_pause_click_for_clock.set(None);
                            false
                        } else {
                            true
                        }
                    } else {
                        last_play_pause_click_for_clock.set(None);
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // Update shared media progress states second-by-second
            let new_title = snapshot.media.title.clone();
            let new_artist = snapshot.media.artist.clone();
            let new_pos = snapshot.media.position_seconds;
            let new_dur = snapshot.media.duration_seconds;
            let detected_playing = snapshot.media.is_playing && snapshot.media.has_media;
            let now_for_waveform = std::time::Instant::now();
            if detected_playing {
                pending_media_paused_since = None;
            } else if snapshot.media.has_media && pending_media_paused_since.is_none() {
                pending_media_paused_since = Some(now_for_waveform);
            }
            let paused_confirmed = pending_media_paused_since
                .map(|since| now_for_waveform.duration_since(since) >= std::time::Duration::from_millis(650))
                .unwrap_or(!snapshot.media.has_media);
            let new_playing = detected_playing || (snapshot.media.has_media && !paused_confirmed && last_ui_is_playing == Some(true));

            if !is_in_playback_lockout && last_ui_is_playing != Some(new_playing) {
                ui.set_is_playing(new_playing);
                last_ui_is_playing = Some(new_playing);
            }

            let waveform_active = snapshot.media.has_media && new_playing;
            if last_ui_waveform_active != Some(waveform_active) {
                ui.set_media_waveform_active(waveform_active);
                last_ui_waveform_active = Some(waveform_active);
            }
            
            let mut track_changed = false;
            {
                let mut current_title = clock_last_title.lock().unwrap();
                let mut current_artist = clock_last_artist.lock().unwrap();
                if new_title != *current_title || new_artist != *current_artist {
                    track_changed = true;
                    *current_title = new_title.clone();
                    *current_artist = new_artist;
                }
            }

            let mut local_pos = 0.0;
            if snapshot.media.has_media {
                if let (Ok(mut smoothed), Ok(mut last_update)) = (clock_smoothed.lock(), clock_last_update.lock()) {
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(*last_update).as_secs_f64();
                    *last_update = now;
                    
                    if new_playing {
                        let diff = new_pos - *smoothed;
                        if track_changed || diff.abs() >= 3.0 || *smoothed > new_dur {
                            // For track changes, user seeks (>= 3.0s), or overrun, snap to OS position
                            *smoothed = new_pos;
                        } else {
                            // Slew the step size slightly to gently converge to the OS position.
                            // Since step is clamped >= 0.1 * elapsed, it is guaranteed to be positive (no recoil).
                            let step = elapsed + diff * 0.25;
                            let step = step.clamp(0.1 * elapsed, 2.0 * elapsed);
                            *smoothed += step;
                        }
                    } else {
                        *smoothed = new_pos;
                    }
                    
                    local_pos = *smoothed;
                }

                if new_dur > 0.0 {
                    let pct = ((local_pos / new_dur) * 100.0).clamp(0.0, 100.0) as f32;
                    if (pct - last_ui_media_pct).abs() >= 0.05 {
                        ui.set_media_pct(pct);
                        last_ui_media_pct = pct;
                    }
                    
                    let pos_sec = local_pos.round() as i32;
                    let pos_str = format!("{}:{:02}", pos_sec / 60, pos_sec % 60);
                    if pos_str != last_ui_media_pos_str {
                        ui.set_media_pos_str(pos_str.clone().into());
                        last_ui_media_pos_str = pos_str;
                    }
                } else {
                    if last_ui_media_pct != 0.0 {
                        ui.set_media_pct(0.0);
                        last_ui_media_pct = 0.0;
                    }
                    if last_ui_media_pos_str != "0:00" {
                        ui.set_media_pos_str("0:00".into());
                        last_ui_media_pos_str = "0:00".to_string();
                    }
                }

                let dur_sec = snapshot.media.duration_seconds as i32;
                let dur_str = format!("{}:{:02}", dur_sec / 60, dur_sec % 60);
                if dur_str != last_ui_media_dur_str {
                    ui.set_media_dur_str(dur_str.clone().into());
                    last_ui_media_dur_str = dur_str;
                }
            }

            // Media widget sync (using calculated local_pos for smooth progress ticks)
            {
                let m_guard = media_widget_for_timer.borrow();
                if let Some(w) = m_guard.as_ref() {
                    update_media_widget_properties(w, &snapshot, local_pos);
                    w.set_is_locked(settings_current.widgets.locked);
                    w.set_bg_opacity(settings_current.widgets.opacity);
                    w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
                }
            }

            // Notes widget sync
            {
                let n_guard = notes_widget_for_timer.borrow();
                if let Some(w) = n_guard.as_ref() {
                    w.set_is_locked(settings_current.widgets.locked);
                    w.set_bg_opacity(settings_current.widgets.opacity);
                    w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
                    w.set_notes_text(settings_current.widgets.notes_text.clone().into());
                }
            }

            // Todo widget sync
            {
                let t_guard = todo_widget_for_timer.borrow();
                if let Some(w) = t_guard.as_ref() {
                    w.set_is_locked(settings_current.widgets.locked);
                    w.set_bg_opacity(settings_current.widgets.opacity);
                    w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
                    w.set_accent_color(parse_hex_color(&settings_current.widgets.todo_accent_color));
                    
                    let slint_items: Vec<TodoItem> = settings_current.widgets.todo_items.iter()
                        .filter(|item| !(settings_current.widgets.todo_hide_completed && item.completed))
                        .map(|item| TodoItem {
                            id: item.id,
                            text: item.text.clone().into(),
                            completed: item.completed,
                        })
                        .collect();
                    w.set_todo_items(std::rc::Rc::new(slint::VecModel::from(slint_items)).into());
                }
            }

            // Quotes widget sync and cycling
            {
                let q_guard = quotes_widget_for_timer.borrow();
                if let Some(w) = q_guard.as_ref() {
                    w.set_is_locked(settings_current.widgets.locked);
                    w.set_bg_opacity(settings_current.widgets.opacity);
                    w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
                    
                    // Periodic Cycling check
                    let cycle_enabled = settings_current.widgets.quotes_cycle_enabled;
                    let interval_secs = settings_current.widgets.quotes_change_interval_mins as u64 * 60;
                    let elapsed = quotes_last_change.get().elapsed().as_secs();
                    
                    let mut trigger_refresh = false;
                    if cycle_enabled && elapsed >= interval_secs {
                        trigger_refresh = true;
                    }
                    
                    // If first tick, or text is empty, refresh it
                    if w.get_quote_text().is_empty() {
                        trigger_refresh = true;
                    }
                    
                    if trigger_refresh {
                        quotes_last_change.set(std::time::Instant::now());
                        
                        let mut all_quotes: Vec<(String, String)> = DEFAULT_QUOTES.iter()
                            .map(|(q, a)| (q.to_string(), a.to_string()))
                            .collect();
                        
                        for custom in &settings_current.widgets.quotes_custom_quotes {
                            let parts: Vec<&str> = custom.split('|').collect();
                            if parts.len() == 2 {
                                all_quotes.push((parts[0].to_string(), parts[1].to_string()));
                            } else if parts.len() == 1 {
                                all_quotes.push((parts[0].to_string(), "Unknown".to_string()));
                            }
                        }
                        
                        use rand::seq::SliceRandom;
                        let mut rng = rand::thread_rng();
                        if let Some((quote, author)) = all_quotes.choose(&mut rng) {
                            w.set_quote_text(quote.clone().into());
                            w.set_quote_author(author.clone().into());
                        }
                    }
                }
            }

            // Picture widget sync
            {
                let p_guard = picture_widget_for_timer.borrow();
                if let Some(w) = p_guard.as_ref() {
                    w.set_is_locked(settings_current.widgets.locked);
                    w.set_bg_opacity(settings_current.widgets.opacity);
                    w.set_border_radius_val(settings_current.widgets.stats_border_radius as i32);
                    let path = settings_current.widgets.picture_path.clone();
                    let old_path: String = w.get_picture_path().into();
                    if path != old_path {
                        w.set_picture_path(path.clone().into());
                        if !path.is_empty() {
                            if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&path)) {
                                w.set_picture_img(img);
                                w.set_has_picture(true);
                            } else {
                                w.set_has_picture(false);
                            }
                        } else {
                            w.set_has_picture(false);
                        }
                    }
                }
            }

            // Extra widget instances sync - duplicates use the same real widget components.
            {
                let widgets_guard = instance_widgets_for_timer.borrow();
                for instance in settings_current
                    .widgets
                    .instances
                    .iter()
                    .filter(|instance| instance.visible && !instance.id.is_empty())
                {
                    if let Some(widget) = widgets_guard.get(&instance.id) {
                        if let ExtraWidgetWindow::Media(media_widget) = widget {
                            update_media_widget_properties(media_widget, &snapshot, local_pos);
                        }
                        sync_extra_widget_window(
                            widget,
                            &settings_current,
                            instance,
                            &focus_runtime_for_extra_widgets,
                        );
                    }
                }
            }
            if last_extra_widget_position_save.elapsed() >= std::time::Duration::from_secs(10) {
                last_extra_widget_position_save = std::time::Instant::now();
                unsafe {
                    save_current_extra_widget_positions(&instance_widgets_for_timer);
                }
            }

            // Synced lyrics update
            let (lyr_lines, lyr_index, lyr_has, lyr_status) = get_lyrics_for_track(
                &snapshot.media.title,
                &snapshot.media.artist,
                snapshot.media.position_seconds,
                snapshot.media.duration_seconds,
            );
            // println!("[DEBUG-LYRICS-UI] title='{}', artist='{}', pos={:.2}, lyr_has={}, lyr_index={}, lyr_lines_len={}, lyr_status='{}'",
            //          snapshot.media.title, snapshot.media.artist, snapshot.media.position_seconds, lyr_has, lyr_index, lyr_lines.len(), lyr_status);
            let lyrics_signature = lyr_lines.join("\n");
            if lyrics_signature != last_lyrics_lines_signature {
                last_lyrics_lines_signature = lyrics_signature;
                let model = slint::ModelRc::new(slint::VecModel::from(
                    lyr_lines.into_iter().map(slint::SharedString::from).collect::<Vec<_>>()
                ));
                ui.set_lyrics_lines(model);
            }
            if last_lyrics_active_index != Some(lyr_index) {
                ui.set_active_lyric_index(lyr_index);
                last_lyrics_active_index = Some(lyr_index);
            }
            if last_lyrics_has != Some(lyr_has) {
                ui.set_has_lyrics(lyr_has);
                last_lyrics_has = Some(lyr_has);
            }
            if lyr_status != last_lyrics_status {
                last_lyrics_status = lyr_status;
                ui.set_lyrics_status(last_lyrics_status.clone().into());
            }

            // Update media sessions list
            let sessions_signature = snapshot.media.sessions.iter()
                .map(|sess| format!("{}|{}|{}|{}", sess.source_id, sess.clean_name, sess.icon_path, sess.is_active))
                .collect::<Vec<_>>()
                .join("\n");
            if sessions_signature != last_media_sessions_signature {
                last_media_sessions_signature = sessions_signature;
                let slint_sessions: Vec<SlintMediaSession> = snapshot.media.sessions.iter().map(|sess| {
                    let icon_img = if sess.icon_path.is_empty() {
                        slint::Image::default()
                    } else if let Some(cached_img) = session_icon_cache.get(&sess.icon_path) {
                        cached_img.clone()
                    } else {
                        let img = slint::Image::load_from_path(std::path::Path::new(&sess.icon_path)).unwrap_or_default();
                        session_icon_cache.insert(sess.icon_path.clone(), img.clone());
                        img
                    };
                    SlintMediaSession {
                        source_id: sess.source_id.clone().into(),
                        clean_name: sess.clean_name.clone().into(),
                        icon: icon_img,
                        is_active: sess.is_active,
                    }
                }).collect();
                ui.set_media_sessions(std::rc::Rc::new(slint::VecModel::from(slint_sessions)).into());
            }

            // Media source app icon loading
            if snapshot.media.source_icon_path != last_source_icon_path {
                last_source_icon_path = snapshot.media.source_icon_path.clone();
                if !last_source_icon_path.is_empty() {
                    if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&last_source_icon_path)) {
                        ui.set_media_source_icon_url(img);
                        ui.set_has_media_source_icon(true);
                    } else {
                        ui.set_has_media_source_icon(false);
                    }
                } else {
                    ui.set_has_media_source_icon(false);
                }
            }

            let adaptive_accent_changed = settings_current.media.adaptive_accent != last_adaptive_accent;
            let settings_accent_changed = settings_current.appearance.accent_color != last_settings_accent_color;

            // Media dynamic art loading & accent extraction
            if snapshot.media.album_art_path != last_album_art_path 
                || track_changed 
                || adaptive_accent_changed 
                || settings_accent_changed 
            {
                last_album_art_path = snapshot.media.album_art_path.clone();
                last_adaptive_accent = settings_current.media.adaptive_accent;
                last_settings_accent_color = settings_current.appearance.accent_color.clone();

                if !last_album_art_path.is_empty() {
                    if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&last_album_art_path)) {
                        ui.set_media_art_url(img);
                        ui.set_has_media_art(true);
                    } else {
                        ui.set_has_media_art(false);
                    }

                    // Extract vibrant accent color
                    let mut accent_applied = false;
                    if settings_current.media.adaptive_accent {
                        if let Some(color) = extract_accent_color(&last_album_art_path) {
                            ui.set_media_accent_color(color);
                            accent_applied = true;
                        }
                    }
                    if !accent_applied {
                        // Fallback to settings accent color
                        let color = parse_hex_color(&settings_current.appearance.accent_color);
                        ui.set_media_accent_color(color);
                    }
                } else {
                    ui.set_has_media_art(false);
                    // Fallback to settings accent color when no art
                    let color = parse_hex_color(&settings_current.appearance.accent_color);
                    ui.set_media_accent_color(color);
                }
            }

            // Calendar events
            let events: Vec<SlintCalendarEvent> = snapshot.calendar.items.iter().map(|item| {
                SlintCalendarEvent {
                    title: item.title.clone().into(),
                    date_str: item.date_str.clone().into(),
                }
            }).collect();
            ui.set_calendar_events(std::rc::Rc::new(slint::VecModel::from(events)).into());
            ui.set_google_connected(snapshot.calendar.google_connected);
            if !ui.get_google_busy() {
                let message = if snapshot.calendar.google_connected
                    || snapshot.calendar.message.contains("failed")
                    || snapshot.calendar.message.contains("timed out")
                    || snapshot.calendar.message.contains("OAuth")
                    || snapshot.calendar.message.contains("Unable")
                    || snapshot.calendar.message.contains("connected")
                {
                    snapshot.calendar.message.clone()
                } else {
                    String::new()
                };
                ui.set_google_message(message.into());
            }

            {
                let today_date = chrono::Local::now().date_naive();
                let mut state = calendar_state_tick.lock().unwrap();
                let force_rebuild = state.last_today != Some(today_date);
                let snap_events = snapshot.calendar.items.clone();
                let events_changed = state.master_events != snap_events;
                
                if force_rebuild || events_changed {
                    if force_rebuild {
                        state.last_today = Some(today_date);
                        state.center_date = today_date;
                    }
                    if events_changed {
                        state.master_events = snap_events;
                    }
                    update_calendar_ui(&ui, state.selected_day_index, &state.master_events, force_rebuild, state.center_date);
                }
            }

            // Notifications history
            let notifications: Vec<SlintNotification> = snapshot.notifications.iter().map(|item| {
                let avatar = item.app_name.chars().next().map(|c| c.to_string()).unwrap_or_else(|| "🔔".to_string());
                SlintNotification {
                    id: item.id as i32,
                    app_name: item.app_name.clone().into(),
                    title: item.title.clone().into(),
                    body: item.body.clone().into(),
                    avatar: avatar.into(),
                }
            }).collect();
            ui.set_notifications(std::rc::Rc::new(slint::VecModel::from(notifications)).into());

            // Shelf items
            let shelf_items_raw = services_cloned.shelf.items();
            let shelf_items: Vec<SlintShelfItem> = shelf_items_raw.into_iter().map(|item| {
                let thumbnail = if item.is_image {
                    slint::Image::load_from_path(std::path::Path::new(&item.path)).unwrap_or_default()
                } else {
                    slint::Image::default()
                };
                
                let size_str = if item.size >= 1_048_576 {
                    format!("{:.1} MB", item.size as f64 / 1_048_576.0)
                } else if item.size >= 1024 {
                    format!("{:.1} KB", item.size as f64 / 1024.0)
                } else {
                    format!("{} B", item.size)
                };

                SlintShelfItem {
                    id: item.id.into(),
                    name: item.name.into(),
                    path: item.path.into(),
                    size_str: size_str.into(),
                    is_image: item.is_image,
                    is_video: item.is_video,
                    thumbnail,
                }
            }).collect();
            ui.set_shelf_items(std::rc::Rc::new(slint::VecModel::from(shelf_items)).into());
        }
    });

    // Wire Up Callback Handlers
    let motion_state_open = ui_motion_state.clone();
    let ui_motion_open_weak = ui.as_weak();
    ui.on_request_notch_open(move || {
        if let Some(ui) = ui_motion_open_weak.upgrade() {
            begin_notch_motion(&ui, &motion_state_open, true);
        }
    });

    let motion_state_close = ui_motion_state.clone();
    let ui_motion_close_weak = ui.as_weak();
    ui.on_request_notch_close(move || {
        if let Some(ui) = ui_motion_close_weak.upgrade() {
            ui.invoke_close_transient_panels();
            crate::window::set_window_interactive_mode(false);
            crate::window::update_pill_window_layout();
            begin_notch_motion(&ui, &motion_state_close, false);
        }
    });

    let tab_clear_timer: Rc<RefCell<Option<slint::Timer>>> = Rc::new(RefCell::new(None));
    let tab_clear_timer_cell = tab_clear_timer.clone();
    let ui_tab_weak = ui.as_weak();
    ui.on_switch_tab(move |next_tab| {
        let next_tab_str = next_tab.to_string();
        window::set_active_tab(next_tab_str.clone());
        if let Some(ui) = ui_tab_weak.upgrade() {
            let next = next_tab.to_string();
            let current = ui.get_active_tab().to_string();
            if next == current {
                return;
            }
            let direction = if tab_index(&next) >= tab_index(&current) {
                "forward"
            } else {
                "backward"
            };
            ui.set_panel_direction(direction.into());
            ui.set_exiting_tab(current.into());
            ui.set_active_tab(next.into());

            let clear_ui = ui.as_weak();
            let timer = slint::Timer::default();
            timer.start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(560),
                move || {
                    if let Some(ui) = clear_ui.upgrade() {
                        ui.set_exiting_tab("".into());
                    }
                },
            );
            *tab_clear_timer_cell.borrow_mut() = Some(timer);
        }
    });

    let flip_timer = std::rc::Rc::new(std::cell::RefCell::new(None));
    let last_play_pause_click_for_cb = last_play_pause_click.clone();
    let s_cloned = services.clone();
    ui.on_play_pause(move || {
        last_play_pause_click_for_cb.set(Some(std::time::Instant::now()));
        s_cloned.media.play_pause();
    });
    
    let ui_weak = ui.as_weak();
    let s_cloned = services.clone();
    let ft_cloned = flip_timer.clone();
    ui.on_prev(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_album_flip_scale(0.0);
            let ui_inner = ui_weak.clone();
            let s_inner = s_cloned.clone();
            let timer = slint::Timer::default();
            timer.start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(150),
                move || {
                    s_inner.media.previous();
                    if let Some(ui) = ui_inner.upgrade() {
                        ui.set_album_flip_scale(1.0);
                    }
                }
            );
            *ft_cloned.borrow_mut() = Some(timer);
        }
    });

    let ui_weak = ui.as_weak();
    let s_cloned = services.clone();
    let ft_cloned = flip_timer.clone();
    ui.on_next(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_album_flip_scale(0.0);
            let ui_inner = ui_weak.clone();
            let s_inner = s_cloned.clone();
            let timer = slint::Timer::default();
            timer.start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(150),
                move || {
                    s_inner.media.next();
                    if let Some(ui) = ui_inner.upgrade() {
                        ui.set_album_flip_scale(1.0);
                    }
                }
            );
            *ft_cloned.borrow_mut() = Some(timer);
        }
    });

    let s_cloned = services.clone();
    ui.on_seek_backward(move || {
        s_cloned.media.seek(false);
    });

    let s_cloned = services.clone();
    ui.on_seek_forward(move || {
        s_cloned.media.seek(true);
    });

    let s_cloned = services.clone();
    ui.on_cycle_media_source(move |forward| {
        s_cloned.media.cycle_session(forward);
    });

    let s_cloned = services.clone();
    ui.on_switch_media_source(move |source_id| {
        s_cloned.media.switch_to_session(&source_id);
    });

    ui.on_activate_app(move |hwnd_val| {
        crate::window::activate_window(hwnd_val as isize);
    });

    let ui_weak = ui.as_weak();
    ui.on_show_app_context_menu(move |hwnd_val| {
        if let Some(ui_instance) = ui_weak.upgrade() {
            use raw_window_handle::HasWindowHandle;
            if let Ok(handle) = ui_instance.window().window_handle().window_handle() {
                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                    show_app_context_menu(hwnd, hwnd_val as isize);
                }
            }
        }
    });

    let ui_weak_for_start = ui.as_weak();
    let settings_ui_weak_for_start = settings_ui.as_weak();
    let motion_state_for_start = ui_motion_state.clone();
    ui.on_show_start_menu(move || {
        if let Some(ui_instance) = ui_weak_for_start.upgrade() {
            use raw_window_handle::HasWindowHandle;
            if let Ok(handle) = ui_instance.window().window_handle().window_handle() {
                if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                    let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                    show_start_context_menu(
                        hwnd,
                        settings_ui_weak_for_start.clone(),
                        ui_weak_for_start.clone(),
                        motion_state_for_start.clone(),
                    );
                }
            }
        }
    });

    // Raven icon control center: Lock
    ui.on_raven_lock(|| {
        let _ = std::process::Command::new("rundll32")
            .args(["user32.dll,LockWorkStation"])
            .spawn();
    });

    // Raven icon control center: Sleep
    ui.on_raven_sleep(|| {
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command",
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Application]::SetSuspendState([System.Windows.Forms.PowerState]::Suspend, $false, $false)"])
            .spawn();
    });

    // Raven icon control center: Shut Down
    ui.on_raven_shutdown(|| {
        let _ = std::process::Command::new("shutdown")
            .args(["/s", "/t", "0"])
            .spawn();
    });

    // Raven icon control center: Restart
    ui.on_raven_restart(|| {
        let _ = std::process::Command::new("shutdown")
            .args(["/r", "/t", "0"])
            .spawn();
    });

    let settings_ui_weak = settings_ui.as_weak();
    let motion_state_for_settings = ui_motion_state.clone();
    let ui_weak_for_settings = ui.as_weak();
    ui.on_open_settings(move || {
        println!("[SETTINGS-OPENED] on_open_settings callback fired");
        if let Some(s_ui) = settings_ui_weak.upgrade() {
            // Hide first to suppress any OS paint at default location
            s_ui.hide().unwrap();
            // Center before show so it never appears at (0,0)
            center_settings_window(&s_ui);
            println!("[SETTINGS-OPENED] calling show()");
            SETTINGS_WINDOW_OPEN.store(true, Ordering::SeqCst);
            s_ui.show().unwrap();
            println!("[SETTINGS-OPENED] show() returned — window should be visible");
        }
        if let Some(ui) = ui_weak_for_settings.upgrade() {
            begin_notch_motion(&ui, &motion_state_for_settings, false);
        }
        // Note: do NOT call open_settings_file() here — that opens the raw JSON in an editor.
        // The Slint SettingsWindow IS the settings panel.
    });
    ui.on_buy_license(move || {
        open_external_url("https://ravennotch.me/");
    });
    let s_cloned = services.clone();
    ui.on_toggle_caffeine(move |keep_screen| { s_cloned.toggle_caffeine_screen(keep_screen); });
    let s_cloned = services.clone();
    ui.on_timer_toggle(move || { s_cloned.toggle_timer(); });
    let s_cloned = services.clone();
    ui.on_timer_reset(move || { s_cloned.reset_timer(); });
    let s_cloned = services.clone();
    ui.on_set_timer_duration(move |secs| { s_cloned.set_timer_duration(secs as u64); });
    let s_cloned = services.clone();
    ui.on_timer_set_remaining_pct(move |pct| {
        let clock = s_cloned.clock.read();
        let duration = clock.timer_duration_seconds;
        if duration > 0 {
            let remaining = ((1.0 - pct) * duration as f32).round() as u64;
            s_cloned.clock.set_timer_remaining(remaining);
        }
    });
    let s_cloned = services.clone();
    let focus_completed_shown_cb = focus_completed_shown.clone();
    let focus_bar_hidden_by_user_cb = focus_bar_hidden_by_user.clone();
    ui.on_focus_session_start(move |goal, duration_str| {
        s_cloned.start_focus_session(goal.to_string(), duration_str.to_string());
        *focus_completed_shown_cb.borrow_mut() = false;
        *focus_bar_hidden_by_user_cb.borrow_mut() = false;
    });

    let s_cloned = services.clone();
    ui.on_focus_session_stop(move || {
        s_cloned.stop_focus_session();
    });

    let s_cloned = services.clone();
    ui.on_focus_session_toggle_pause(move || {
        s_cloned.clock.toggle_pause_focus_session();
    });

    let focus_bar_hidden_by_user_cb = focus_bar_hidden_by_user.clone();
    let ui_weak = ui.as_weak();
    ui.on_focus_session_toggle_bar_visibility(move || {
        let current = *focus_bar_hidden_by_user_cb.borrow();
        let next = !current;
        *focus_bar_hidden_by_user_cb.borrow_mut() = next;
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_focus_bar_hidden(next);
        }
    });

    let ui_weak_clear = ui.as_weak();
    ui.on_clear_focus_history(move || {
        let updated_settings = crate::settings::clear_focus_history();
        if let Some(ui) = ui_weak_clear.upgrade() {
            let slint_history: Vec<SlintFocusSession> = updated_settings.focus_sessions.iter().map(|s| SlintFocusSession {
                goal: s.goal.clone().into(),
                duration_mins: s.duration_mins,
                completed_at: s.completed_at.clone().into(),
            }).collect();
            ui.set_focus_session_history(std::rc::Rc::new(slint::VecModel::from(slint_history)).into());
        }
    });

    ui.on_validate_duration_input(move |text| {
        let filtered: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
        filtered.into()
    });

    ui.on_format_numeric_duration(move |val| {
        let mins: i32 = val.trim().parse().unwrap_or(0);
        if mins == 0 {
            return "0 minute".into();
        }
        let hours = mins / 60;
        let remaining_mins = mins % 60;
        
        let mut result = String::new();
        if hours > 0 {
            result.push_str(&format!("{} hour", hours));
        }
        if remaining_mins > 0 {
            if !result.is_empty() {
                result.push_str(" ");
            }
            result.push_str(&format!("{} minute", remaining_mins));
        }
        result.into()
    });

    ui.on_focus_duration_dropdown_changed(move |open| {
        crate::window::set_window_interactive_mode(open);
    });

    let s_cloned = services.clone();
    ui.on_volume_mute(move || { s_cloned.volume_mute(); });
    
    let s_cloned = services.clone();
    ui.on_toggle_caffeine_screen_setting(move |keep_screen| {
        let snapshot = s_cloned.snapshot();
        if snapshot.caffeine.enabled {
            s_cloned.caffeine.start(keep_screen);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_timer_text_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let remaining_str = ui.get_timer_remaining_str().to_string();
            let parts: Vec<&str> = remaining_str.split(':').collect();
            if parts.len() == 3 {
                ui.set_input_h(parts[0].trim().parse::<u32>().unwrap_or(0).to_string().into());
                ui.set_input_m(parts[1].trim().parse::<u32>().unwrap_or(25).to_string().into());
                ui.set_input_s(parts[2].trim().parse::<u32>().unwrap_or(30).to_string().into());
            } else if parts.len() == 2 {
                ui.set_input_h("0".into());
                ui.set_input_m(parts[0].trim().parse::<u32>().unwrap_or(25).to_string().into());
                ui.set_input_s(parts[1].trim().parse::<u32>().unwrap_or(30).to_string().into());
            }
            ui.set_show_timer_input_panel(true);
            // Allow keyboard input in TextInput fields
            crate::window::set_window_interactive_mode(true);
        }
    });

    let ui_weak_drop = ui.as_weak();
    let calendar_state_toggle = calendar_state.clone();
    ui.on_toggle_calendar_dropdown_rust(move || {
        if let Some(ui) = ui_weak_drop.upgrade() {
            let next = !ui.get_show_calendar_dropdown();
            ui.set_show_calendar_dropdown(next);
            ui.set_show_timer_dropdown(false);
            ui.set_show_volume_dropdown(false);
            ui.set_show_raven_menu(false);
            
            if next {
                let state = calendar_state_toggle.lock().unwrap();
                let selected_offset = (state.selected_day_index - 180) as i64;
                let selected_date = state.center_date.checked_add_signed(chrono::Duration::days(selected_offset)).unwrap_or(state.center_date);
                
                use chrono::Datelike;
                let view_month = selected_date.month() as i32;
                let view_year = selected_date.year();
                ui.set_mini_calendar_view_month(view_month);
                ui.set_mini_calendar_view_year(view_year);
                
                let mini_days = generate_mini_calendar_days(view_year, view_month, selected_date);
                ui.set_mini_calendar_days(std::rc::Rc::new(slint::VecModel::from(mini_days)).into());
            }
            
            crate::window::update_pill_window_layout();
        }
    });

    let ui_weak_drop = ui.as_weak();
    ui.on_toggle_timer_dropdown_rust(move || {
        if let Some(ui) = ui_weak_drop.upgrade() {
            let next = !ui.get_show_timer_dropdown();
            ui.set_show_timer_dropdown(next);
            ui.set_show_calendar_dropdown(false);
            ui.set_show_volume_dropdown(false);
            ui.set_show_raven_menu(false);
            // Make window interactive when timer is open so TextInput can receive keyboard events
            if !next {
                crate::window::set_window_interactive_mode(false);
                ui.set_show_timer_input_panel(false);
            }
            crate::window::update_pill_window_layout();
        }
    });

    let ui_weak_drop = ui.as_weak();
    ui.on_toggle_volume_dropdown_rust(move || {
        if let Some(ui) = ui_weak_drop.upgrade() {
            let next = !ui.get_show_volume_dropdown();
            if next {
                let vol = crate::widgets::get_exact_volume();
                ui.set_volume_value(vol);
            }
            ui.set_show_volume_dropdown(next);
            ui.set_show_calendar_dropdown(false);
            ui.set_show_timer_dropdown(false);
            ui.set_show_raven_menu(false);
            ui.set_show_topbar_stats_dropdown(false);
            ui.set_show_clipboard_dropdown(false);
            ui.set_show_wifi_dropdown(false);
            crate::window::update_pill_window_layout();
        }
    });

    let ui_weak_drop = ui.as_weak();
    ui.on_toggle_raven_menu_rust(move || {
        if let Some(ui) = ui_weak_drop.upgrade() {
            let next = !ui.get_show_raven_menu();
            ui.set_show_raven_menu(next);
            ui.set_show_calendar_dropdown(false);
            ui.set_show_timer_dropdown(false);
            ui.set_show_volume_dropdown(false);
            ui.set_show_topbar_stats_dropdown(false);
            ui.set_show_clipboard_dropdown(false);
            ui.set_show_wifi_dropdown(false);
            crate::window::update_pill_window_layout();
        }
    });

    let ui_weak_drop = ui.as_weak();
    ui.on_toggle_topbar_stats_rust(move || {
        if let Some(ui) = ui_weak_drop.upgrade() {
            let next = !ui.get_show_topbar_stats_dropdown();
            ui.set_show_topbar_stats_dropdown(next);
            ui.set_show_clipboard_dropdown(false);
            ui.set_show_wifi_dropdown(false);
            ui.set_show_calendar_dropdown(false);
            ui.set_show_timer_dropdown(false);
            ui.set_show_volume_dropdown(false);
            ui.set_show_raven_menu(false);
            crate::window::update_pill_window_layout();
        }
    });

    let ui_weak_drop = ui.as_weak();
    let clipboard_history_open = clipboard_history.clone();
    let clipboard_next_id_open = clipboard_next_id.clone();
    let clipboard_paste_target_open = clipboard_paste_target.clone();
    ui.on_toggle_clipboard_dropdown_rust(move || {
        if let Some(ui) = ui_weak_drop.upgrade() {
            let next = !ui.get_show_clipboard_dropdown();
            if next {
                unsafe {
                    let hwnd = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
                    clipboard_paste_target_open.set(hwnd.0 as isize);
                }
                if let Some(text) = read_clipboard_text() {
                    let mut history = clipboard_history_open.borrow_mut();
                    if !history.iter().any(|entry| entry.text == text) {
                        let id = clipboard_next_id_open.get();
                        clipboard_next_id_open.set(id + 1);
                        history.insert(0, ClipboardHistoryEntry {
                            id,
                            title: clipboard_title(&text),
                            text,
                            copied_at: chrono::Local::now(),
                            pinned: false,
                            selected: false,
                        });
                    }
                    refresh_clipboard_model(&ui, &history);
                }
                ui.set_clipboard_search_query("".into());
                ui.set_selected_clipboard_index(0);
                ui.set_show_clipboard_settings_panel(false);
                ui.set_show_clipboard_rename_panel(false);
                ui.set_show_clipboard_context_menu(false);
            }
            ui.set_show_clipboard_dropdown(next);
            ui.set_show_topbar_stats_dropdown(false);
            ui.set_show_wifi_dropdown(false);
            ui.set_show_calendar_dropdown(false);
            ui.set_show_timer_dropdown(false);
            ui.set_show_volume_dropdown(false);
            ui.set_show_raven_menu(false);
            crate::window::update_pill_window_layout();
        }
    });

    let ui_weak_drop = ui.as_weak();
    ui.on_toggle_wifi_dropdown_rust(move || {
        if let Some(ui) = ui_weak_drop.upgrade() {
            let next = !ui.get_show_wifi_dropdown();
            ui.set_show_wifi_dropdown(next);
            if next {
                ui.set_wifi_searching(true);
                apply_wifi_snapshot(&ui, wifi_scan_snapshot());
                ui.set_wifi_searching(false);
            }
            ui.set_show_topbar_stats_dropdown(false);
            ui.set_show_clipboard_dropdown(false);
            ui.set_show_calendar_dropdown(false);
            ui.set_show_timer_dropdown(false);
            ui.set_show_volume_dropdown(false);
            ui.set_show_raven_menu(false);
            crate::window::update_pill_window_layout();
        }
    });

    let ui_wifi_refresh = ui.as_weak();
    ui.on_wifi_refresh(move || {
        if let Some(ui) = ui_wifi_refresh.upgrade() {
            ui.set_wifi_searching(true);
            apply_wifi_snapshot(&ui, wifi_scan_snapshot());
            ui.set_wifi_searching(false);
        }
    });

    let ui_wifi_toggle = ui.as_weak();
    ui.on_wifi_toggle(move |enabled| {
        if let Some(ui) = ui_wifi_toggle.upgrade() {
            if !enabled {
                wifi_disconnect_current();
            } else {
                let ssid = ui.get_selected_wifi_ssid().to_string();
                if !ssid.is_empty() {
                    wifi_connect_network(&ssid, &ui.get_wifi_password().to_string());
                }
            }
            apply_wifi_snapshot(&ui, wifi_scan_snapshot());
        }
    });

    let ui_wifi_disconnect = ui.as_weak();
    ui.on_wifi_disconnect(move || {
        if let Some(ui) = ui_wifi_disconnect.upgrade() {
            wifi_disconnect_current();
            apply_wifi_snapshot(&ui, wifi_scan_snapshot());
        }
    });

    let ui_wifi_connect = ui.as_weak();
    ui.on_wifi_connect(move |ssid, password| {
        if let Some(ui) = ui_wifi_connect.upgrade() {
            wifi_connect_network(&ssid.to_string(), &password.to_string());
            apply_wifi_snapshot(&ui, wifi_scan_snapshot());
        }
    });

    let ui_wifi_status_sync = ui.as_weak();
    std::thread::spawn(move || {
        loop {
            let snapshot = wifi_scan_snapshot();
            let ui_weak = ui_wifi_status_sync.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    if !ui.get_wifi_searching() {
                        apply_wifi_snapshot(&ui, snapshot);
                    }
                }
            });
            std::thread::sleep(std::time::Duration::from_secs(8));
        }
    });

    let clipboard_history_select = clipboard_history.clone();
    let ui_clip_select = ui.as_weak();
    ui.on_clipboard_item_selected(move |index| {
        if let Some(ui) = ui_clip_select.upgrade() {
            ui.set_selected_clipboard_index(index);
            refresh_clipboard_model(&ui, &clipboard_history_select.borrow());
        }
    });

    let clipboard_history_select_mod = clipboard_history.clone();
    let ui_clip_select_mod = ui.as_weak();
    ui.on_clipboard_item_selected_mod(move |index, shift, ctrl| {
        if let Some(ui) = ui_clip_select_mod.upgrade() {
            let query = ui.get_clipboard_search_query().to_string();
            let entries = visible_clipboard_entries(&clipboard_history_select_mod.borrow(), &query);
            if let Some(entry) = entries.get(index.max(0) as usize) {
                let target_id = entry.id;
                let mut history = clipboard_history_select_mod.borrow_mut();
                if shift || ctrl || ui.get_clipboard_multi_delete_mode() {
                    if let Some(item) = history.iter_mut().find(|item| item.id == target_id) {
                        item.selected = !item.selected;
                    }
                } else {
                    for item in history.iter_mut() {
                        item.selected = item.id == target_id;
                    }
                }
            }
            ui.set_selected_clipboard_index(index);
            refresh_clipboard_model(&ui, &clipboard_history_select_mod.borrow());
        }
    });

    let clipboard_history_trigger = clipboard_history.clone();
    let clipboard_paste_target_trigger = clipboard_paste_target.clone();
    let ui_clip_trigger = ui.as_weak();
    ui.on_clipboard_item_triggered(move |index| {
        if let Some(ui) = ui_clip_trigger.upgrade() {
            let query = ui.get_clipboard_search_query().to_string();
            let entries = visible_clipboard_entries(&clipboard_history_trigger.borrow(), &query);
            if let Some(entry) = entries.get(index.max(0) as usize) {
                ui.set_show_clipboard_dropdown(false);
                ui.set_show_clipboard_settings_panel(false);
                ui.set_show_clipboard_context_menu(false);
                crate::window::update_pill_window_layout();
                paste_text_to_target(entry.text.clone(), clipboard_paste_target_trigger.get());
            }
        }
    });

    let clipboard_history_search = clipboard_history.clone();
    let ui_clip_search = ui.as_weak();
    ui.on_clipboard_search_changed(move |query| {
        if let Some(ui) = ui_clip_search.upgrade() {
            ui.set_clipboard_search_query(query);
            ui.set_selected_clipboard_index(0);
            refresh_clipboard_model(&ui, &clipboard_history_search.borrow());
        }
    });

    let clipboard_history_next = clipboard_history.clone();
    let ui_clip_next = ui.as_weak();
    ui.on_clipboard_select_next(move || {
        if let Some(ui) = ui_clip_next.upgrade() {
            let len = visible_clipboard_entries(&clipboard_history_next.borrow(), &ui.get_clipboard_search_query().to_string()).len() as i32;
            if len > 0 {
                ui.set_selected_clipboard_index((ui.get_selected_clipboard_index() + 1).min(len - 1));
            }
        }
    });

    let clipboard_history_prev = clipboard_history.clone();
    let ui_clip_prev = ui.as_weak();
    ui.on_clipboard_select_prev(move || {
        if let Some(ui) = ui_clip_prev.upgrade() {
            let len = visible_clipboard_entries(&clipboard_history_prev.borrow(), &ui.get_clipboard_search_query().to_string()).len() as i32;
            if len > 0 {
                ui.set_selected_clipboard_index((ui.get_selected_clipboard_index() - 1).max(0));
            }
        }
    });

    let clipboard_history_rename = clipboard_history.clone();
    let ui_clip_rename = ui.as_weak();
    ui.on_clipboard_rename_selected(move |new_title| {
        if let Some(ui) = ui_clip_rename.upgrade() {
            let entries = visible_clipboard_entries(&clipboard_history_rename.borrow(), &ui.get_clipboard_search_query().to_string());
            if let Some(entry) = entries.get(ui.get_selected_clipboard_index().max(0) as usize) {
                if let Some(item) = clipboard_history_rename.borrow_mut().iter_mut().find(|item| item.id == entry.id) {
                    item.title = new_title.to_string();
                }
            }
            refresh_clipboard_model(&ui, &clipboard_history_rename.borrow());
        }
    });

    let clipboard_history_pin = clipboard_history.clone();
    let ui_clip_pin = ui.as_weak();
    ui.on_clipboard_toggle_pin(move || {
        if let Some(ui) = ui_clip_pin.upgrade() {
            let entries = visible_clipboard_entries(&clipboard_history_pin.borrow(), &ui.get_clipboard_search_query().to_string());
            if let Some(entry) = entries.get(ui.get_selected_clipboard_index().max(0) as usize) {
                if let Some(item) = clipboard_history_pin.borrow_mut().iter_mut().find(|item| item.id == entry.id) {
                    item.pinned = !item.pinned;
                }
            }
            refresh_clipboard_model(&ui, &clipboard_history_pin.borrow());
        }
    });

    let clipboard_history_delete = clipboard_history.clone();
    let ui_clip_delete = ui.as_weak();
    ui.on_clipboard_delete_selected(move || {
        if let Some(ui) = ui_clip_delete.upgrade() {
            let entries = visible_clipboard_entries(&clipboard_history_delete.borrow(), &ui.get_clipboard_search_query().to_string());
            if let Some(entry) = entries.get(ui.get_selected_clipboard_index().max(0) as usize) {
                clipboard_history_delete.borrow_mut().retain(|item| item.id != entry.id);
            }
            refresh_clipboard_model(&ui, &clipboard_history_delete.borrow());
        }
    });

    let ui_clip_multi = ui.as_weak();
    ui.on_clipboard_delete_multiple(move || {
        if let Some(ui) = ui_clip_multi.upgrade() {
            ui.set_clipboard_multi_delete_mode(true);
        }
    });

    let clipboard_history_delete_multi = clipboard_history.clone();
    let ui_clip_delete_multi = ui.as_weak();
    ui.on_clipboard_delete_multi_selected(move || {
        if let Some(ui) = ui_clip_delete_multi.upgrade() {
            clipboard_history_delete_multi.borrow_mut().retain(|item| !item.selected);
            ui.set_clipboard_multi_delete_mode(false);
            refresh_clipboard_model(&ui, &clipboard_history_delete_multi.borrow());
        }
    });

    let clipboard_history_delete_all = clipboard_history.clone();
    let ui_clip_delete_all = ui.as_weak();
    ui.on_clipboard_delete_all(move || {
        if let Some(ui) = ui_clip_delete_all.upgrade() {
            clipboard_history_delete_all.borrow_mut().clear();
            refresh_clipboard_model(&ui, &clipboard_history_delete_all.borrow());
        }
    });

    ui.on_clipboard_item_context_menu(move |_, _, _| {});
    let clipboard_history_drag = clipboard_history.clone();
    let clipboard_paste_target_drag = clipboard_paste_target.clone();
    let ui_clip_drag = ui.as_weak();
    let clipboard_drag_preview_cell: Rc<RefCell<Option<ClipboardDragPreviewWindow>>> =
        Rc::new(RefCell::new(None));
    let clipboard_drag_preview_cell_cb = clipboard_drag_preview_cell.clone();
    ui.on_clipboard_item_drag_started(move |index| {
        let Some(ui) = ui_clip_drag.upgrade() else { return; };
        let query = ui.get_clipboard_search_query().to_string();
        let entries = visible_clipboard_entries(&clipboard_history_drag.borrow(), &query);
        let Some(entry) = entries.get(index.max(0) as usize).cloned() else { return; };
        let mut preview_ref = clipboard_drag_preview_cell_cb.borrow_mut();
        if preview_ref.is_none() {
            match ClipboardDragPreviewWindow::new() {
                Ok(preview) => *preview_ref = Some(preview),
                Err(_) => return,
            }
        }
        let Some(preview) = preview_ref.as_ref() else { return; };
        preview.set_drag_title(entry.title.clone().into());
        let mut pt = windows::Win32::Foundation::POINT::default();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
        }
        preview.window().set_position(slint::PhysicalPosition::new(pt.x + 12, pt.y + 12));
        let _ = preview.show();
        let preview_weak = preview.as_weak();
        drop(preview_ref);

        let target = clipboard_paste_target_drag.get();
        let paste_text = entry.text.clone();
        std::thread::spawn(move || {
            loop {
                let mut pt = windows::Win32::Foundation::POINT::default();
                let down = unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
                    (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                        windows::Win32::UI::Input::KeyboardAndMouse::VK_LBUTTON.0 as i32,
                    ) as u16 & 0x8000) != 0
                };

                let x = pt.x + 12;
                let y = pt.y + 12;
                let _ = slint::invoke_from_event_loop({
                    let preview_weak = preview_weak.clone();
                    move || {
                        if let Some(preview) = preview_weak.upgrade() {
                            preview.window().set_position(slint::PhysicalPosition::new(x, y));
                        }
                    }
                });

                if !down {
                    let _ = slint::invoke_from_event_loop({
                        let preview_weak = preview_weak.clone();
                        move || {
                            if let Some(preview) = preview_weak.upgrade() {
                                let _ = preview.hide();
                            }
                        }
                    });
                    paste_text_to_target(paste_text, target);
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        });
    });
    ui.on_clipboard_item_dropped(move |_| {});
    ui.on_clipboard_save_hotkey(move |_| {});
    ui.on_clipboard_retention_selected(move |_| {});

    let spider_thread_overlay = {
        let mut prewarm: Option<SpiderThreadOverlayWindow> = None;
        if let Ok(overlay) = SpiderThreadOverlayWindow::new() {
            let _ = overlay.show();
            let _ = overlay.hide();
            prewarm = Some(overlay);
        }
        Rc::new(RefCell::new(prewarm))
    };
    let spider_drag_token = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let s_spider = services.clone();
    let spider_overlay_for_drag = spider_thread_overlay.clone();
    let spider_token_for_drag = spider_drag_token.clone();
    ui.on_spider_timer_drag_start(move |local_x, local_y, w, h| {
        let scale = overlay_scale();
        let (origin_x, origin_y) = unsafe {
            use windows::Win32::Foundation::POINT;
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let widget_left = pt.x - (local_x * scale).round() as i32;
            let widget_top = pt.y - (local_y * scale).round() as i32;
            (
                widget_left + ((w / 2.0) * scale).round() as i32,
                widget_top + (h * scale).round() as i32,
            )
        };

        let mut overlay_ref = spider_overlay_for_drag.borrow_mut();
        if overlay_ref.is_none() {
            if let Ok(overlay) = SpiderThreadOverlayWindow::new() {
                *overlay_ref = Some(overlay);
            }
        }
        let Some(overlay) = overlay_ref.as_ref() else {
            return;
        };

        overlay.set_thread_height(80.0);
        overlay.set_timer_minutes(16);
        let overlay_x = origin_x - scaled_px(90.0);
        overlay.window().set_position(slint::PhysicalPosition::new(overlay_x, origin_y));
        let _ = overlay.show();
        let Some(hwnd_val) = slint_component_hwnd(overlay) else {
            return;
        };

        configure_floating_overlay_hwnd(
            hwnd_val,
            overlay_x,
            origin_y,
            scaled_px(180.0),
            scaled_px(2400.0),
            true,
        );

        let token = spider_token_for_drag.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let token_ref = spider_token_for_drag.clone();
        let overlay_weak = overlay.as_weak();
        let services_for_release = s_spider.clone();
        std::thread::spawn(move || {
            let mut last_minutes = 16_i32;
            let mut last_dy = -1_i32;
            unsafe {
                use windows::Win32::Foundation::POINT;
                use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
                use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

                loop {
                    if token_ref.load(std::sync::atomic::Ordering::SeqCst) != token {
                        hide_overlay_hwnd(hwnd_val);
                        let _ = slint::invoke_from_event_loop({
                            let overlay_weak = overlay_weak.clone();
                            move || {
                                if let Some(overlay) = overlay_weak.upgrade() {
                                    let _ = overlay.hide();
                                }
                            }
                        });
                        break;
                    }

                    let mut pt = POINT::default();
                    let _ = GetCursorPos(&mut pt);
                    let dy = (pt.y - origin_y).max(12);
                    let minutes = (dy / 5).max(1);
                    last_minutes = minutes;

                    if dy != last_dy {
                        last_dy = dy;
                        let height = dy as f32 / overlay_scale();
                        let _ = slint::invoke_from_event_loop({
                            let overlay_weak = overlay_weak.clone();
                            move || {
                                if let Some(overlay) = overlay_weak.upgrade() {
                                    overlay.set_thread_height(height);
                                    overlay.set_timer_minutes(minutes);
                                }
                            }
                        });
                    }

                    let down = (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0;
                    if !down {
                        hide_overlay_hwnd(hwnd_val);
                        let _ = slint::invoke_from_event_loop({
                            let overlay_weak = overlay_weak.clone();
                            move || {
                                if let Some(overlay) = overlay_weak.upgrade() {
                                    let _ = overlay.hide();
                                }
                            }
                        });
                        let seconds = (last_minutes as u64).saturating_mul(60);
                        services_for_release.set_timer_duration(seconds);
                        services_for_release.toggle_timer();
                        break;
                    }

                    std::thread::sleep(std::time::Duration::from_millis(16));
                }
            }
        });
    });

    let s_spider_set = services.clone();
    ui.on_spider_timer_set(move |minutes| {
        s_spider_set.set_timer_duration((minutes.max(1) as u64) * 60);
    });

    let ui_weak_drop = ui.as_weak();
    ui.on_close_dropdowns_rust(move || {
        if let Some(ui) = ui_weak_drop.upgrade() {
            ui.invoke_close_transient_panels();
            crate::window::set_window_interactive_mode(false);
            crate::window::update_pill_window_layout();
        }
    });


    ui.on_volume_changed(move |vol| {
        crate::widgets::set_exact_volume(vol);
        crate::window::show_system_hud("volume", vol, false);
    });

    let s_cloned = services.clone();
    let ui_weak_set = ui.as_weak();
    ui.on_set_timer_from_input(move || {
        if let Some(ui) = ui_weak_set.upgrade() {
            let h_str = ui.get_input_h().to_string();
            let m_str = ui.get_input_m().to_string();
            let s_str = ui.get_input_s().to_string();
            
            let h = h_str.trim().parse::<u64>().unwrap_or(0);
            let m = m_str.trim().parse::<u64>().unwrap_or(0);
            let s = s_str.trim().parse::<u64>().unwrap_or(0);
            
            let total_secs = h * 3600 + m * 60 + s;
            s_cloned.set_timer_duration(total_secs);
            ui.set_show_timer_input_panel(false);
            crate::window::set_window_interactive_mode(false);
        }
    });

    let ui_weak_close = ui.as_weak();
    ui.on_close_timer_input_panel(move || {
        if let Some(ui) = ui_weak_close.upgrade() {
            ui.set_show_timer_input_panel(false);
            crate::window::set_window_interactive_mode(false);
        }
    });

    let s_cloned = services.clone();
    
    thread_local! {
        static CAFFEINE_SHUTDOWN_TIMER: std::cell::RefCell<Option<slint::Timer>> = std::cell::RefCell::new(None);
        pub static GLOBAL_UPDATE_LIFECYCLES: std::cell::RefCell<Option<std::rc::Rc<dyn Fn()>>> = std::cell::RefCell::new(None);
    }
    
    ui.on_set_caffeine_duration(move |mins, keep_screen| {
        if mins <= 0 {
            s_cloned.toggle_caffeine_screen(keep_screen);
        } else {
            let snapshot = s_cloned.snapshot();
            if !snapshot.caffeine.enabled {
                s_cloned.toggle_caffeine_screen(keep_screen);
            } else {
                s_cloned.caffeine.start(keep_screen);
            }
            
            let s_cloned_timer = s_cloned.clone();
            CAFFEINE_SHUTDOWN_TIMER.with(|cell| {
                let timer = slint::Timer::default();
                timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_secs(mins as u64 * 60), move || {
                    let snapshot = s_cloned_timer.snapshot();
                    if snapshot.caffeine.enabled {
                        s_cloned_timer.toggle_caffeine_screen(keep_screen);
                    }
                });
                *cell.borrow_mut() = Some(timer);
            });
        }
    });
    let s_cloned = services.clone();
    ui.on_stopwatch_toggle(move || {
        s_cloned.toggle_stopwatch();
        play_sound_by_name("stopwatch");
    });
    let s_cloned = services.clone();
    ui.on_stopwatch_reset(move || { s_cloned.reset_stopwatch(); });
    let s_cloned = services.clone();
    ui.on_trigger_screenshot(move || { s_cloned.capture_screenshot(); });
    let s_cloned = services.clone();
    ui.on_trigger_region_screenshot(move || { s_cloned.capture_region(); });
    let s_cloned = services.clone();
    let ui_weak_google = ui.as_weak();
    let settings_ui_weak_google = settings_ui.as_weak();
    ui.on_google_sign_in(move || {
        if ui_weak_google
            .upgrade()
            .map(|ui| ui.get_premium_locked())
            .unwrap_or(false)
        {
            return;
        }
        if let Some(ui) = ui_weak_google.upgrade() {
            ui.set_google_busy(true);
            ui.set_google_message("".into());
        }
        let services_google = s_cloned.clone();
        let ui_weak_done = ui_weak_google.clone();
        let settings_ui_done = settings_ui_weak_google.clone();
        std::thread::spawn(move || {
            let snapshot = services_google.connect_google_calendar();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak_done.upgrade() {
                    let events: Vec<SlintCalendarEvent> = snapshot.calendar.items.iter().map(|item| {
                        SlintCalendarEvent {
                            title: item.title.clone().into(),
                            date_str: item.date_str.clone().into(),
                        }
                    }).collect();
                    ui.set_calendar_events(std::rc::Rc::new(slint::VecModel::from(events)).into());
                    ui.set_google_connected(snapshot.calendar.google_connected);
                    ui.set_google_message(snapshot.calendar.message.clone().into());
                    ui.set_google_busy(false);
                }
                if let Some(s_ui) = settings_ui_done.upgrade() {
                    s_ui.set_google_connected(snapshot.calendar.google_connected);
                    s_ui.set_google_email(snapshot.calendar.google_email.clone().into());
                    s_ui.set_google_message(snapshot.calendar.message.clone().into());
                    s_ui.set_google_busy(false);

                    let settings = settings::RavenSettings::load();
                    let selected_ids = settings.media.google_calendar_ids.clone();
                    let slint_cals = map_google_calendars(&snapshot.calendar.google_calendars, &selected_ids);
                    let selected_count = if selected_ids.is_empty() && snapshot.calendar.google_connected {
                        1
                    } else {
                        selected_ids.len()
                    };
                    s_ui.set_google_calendars(std::rc::Rc::new(slint::VecModel::from(slint_cals)).into());
                    s_ui.set_google_selected_calendars_count(selected_count as i32);
                }
            });
        });
    });
    let settings_ui_weak_freeze = settings_ui.as_weak();
    let ui_weak_freeze = ui.as_weak();
    ui.on_toggle_freeze(move || {
        let current = HOVER_ENABLED.load(Ordering::SeqCst);
        let new_val = !current;
        HOVER_ENABLED.store(new_val, Ordering::SeqCst);
        settings::set_bool(&["hover", "enabled"], new_val);
        if let Some(s_ui) = settings_ui_weak_freeze.upgrade() {
            s_ui.set_hover_enabled(new_val);
        }
        if let Some(main_ui) = ui_weak_freeze.upgrade() {
            main_ui.set_hover_enabled(new_val);
        }
    });

    ui.on_close_notch(move || {
        // Find the hidden native window and send WM_CLOSE to clean up hotkeys and tray icon,
        // then exit cleanly.
        unsafe {
            let class_name = wide("RavenNativeHidden");
            let title = wide("Raven Native Hidden");
            let resolved = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                windows::core::PCWSTR(class_name.as_ptr()),
                windows::core::PCWSTR(title.as_ptr()),
            );
            if resolved.0 != 0 {
                let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                    resolved,
                    windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
        }
        std::process::exit(0);
    });

    // Wire Up Shelf Callbacks
    ui.on_shelf_open_file(move |path| {
        let path = path.trim_matches('"').to_string();
        println!("[Shelf] Open file: {}", path);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", &path])
                .creation_flags(0x08000000)
                .spawn();
        }
    });

    ui.on_shelf_reveal_file(move |path| {
        let path = path.trim_matches('"').to_string();
        println!("[Shelf] Reveal file: {}", path);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("explorer")
                .arg(format!("/select,{}", path))
                .creation_flags(0x08000000)
                .spawn();
        }
    });

    ui.on_shelf_copy_path(move |path| {
        let path = path.trim_matches('"').to_string();
        println!("[Shelf] Copy path: {}", path);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let escaped = path.replace('\'', "''");
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
        }
    });

    let shelf_cloned = services.shelf.clone();
    ui.on_shelf_remove_item(move |id| {
        println!("[Shelf] Remove item: {}", id);
        shelf_cloned.remove_item(&id);
    });

    let shelf_cloned = services.shelf.clone();
    ui.on_shelf_clear_items(move || {
        println!("[Shelf] Clear items");
        shelf_cloned.clear();
    });

    let shelf_cloned = services.shelf.clone();
    ui.on_shelf_share_file(move |path, provider| {
        let path_str = path.to_string();
        let provider_str = provider.to_string();
        println!("[Shelf] Share file: {} using {}", path_str, provider_str);
        shelf_cloned.share_file(path_str, provider_str);
    });

    let ui_weak = ui.as_weak();
    let settings_ui_weak = settings_ui.as_weak();
    ui.on_shelf_select_next_provider(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let current = ui.get_share_provider_id().to_string();
            let (next_id, next_name) = match current.as_str() {
                "localsend" => ("quickshare", "Quick Share"),
                "quickshare" => ("kdeconnect", "KDE Connect"),
                "kdeconnect" | _ => ("localsend", "LocalSend"),
            };
            ui.set_share_provider_id(next_id.into());
            ui.set_share_provider_name(next_name.into());
            ui.set_share_notice_message("".into());
            ui.set_share_notice_state("info".into());
            println!("[Shelf] Cycle provider to {}", next_name);

            // Persist choice in settings.json
            settings::set_string(&["drop", "default_provider"], next_id);

            // Update settings UI if open
            if let Some(s_ui) = settings_ui_weak.upgrade() {
                s_ui.set_drop_default_provider(next_id.into());
            }
        }
    });


    // Show Notch
    ui.show().unwrap();

    // Setup Slint Window subclassing and position snapping *after* window is shown/mapped
    use raw_window_handle::HasWindowHandle;
    let offset_x = settings.appearance.pill_offset;
    let offset_y = settings.appearance.pill_y_offset;
    let idle_width = settings.appearance.idle_width;
    let idle_height = settings.appearance.idle_height;

    match ui.window().window_handle().window_handle() {
        Ok(handle) => {
            if let raw_window_handle::RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                let hwnd = windows::Win32::Foundation::HWND(win32_handle.hwnd.get() as _);
                window::PILL_HWND.store(hwnd.0, std::sync::atomic::Ordering::SeqCst);
                println!("[SETUP] HWND successfully resolved: {:?}", hwnd);
                let _ = std::io::Write::flush(&mut std::io::stdout());
                let scale = ui.window().scale_factor();
                unsafe {
                    window::setup_slint_window_positioning(hwnd, offset_x, offset_y, idle_width, idle_height, scale);
                    window::register_custom_drop_target(hwnd, ui.as_weak(), services.shelf.clone());
                    window::update_appbar_reservation();
                }
            } else {
                println!("[SETUP] Window handle is not Win32!");
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
        }
        Err(e) => {
            println!("[SETUP] Failed to get window handle: {:?}", e);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }

    {
        let snap_ui_weak = ui.as_weak();
        let snap_timer = slint::Timer::default();
        let logical_offset_x = offset_x;
        let logical_offset_y = offset_y;
        let logical_idle_w = idle_width;
        let logical_idle_h = idle_height;
        let shelf_for_drop = services.shelf.clone();
        let drop_ui_weak = ui.as_weak();
        let mut drop_target_registered = false;
        
        let mut hover_in_time: Option<std::time::Instant> = None;
        let mut hover_out_time: Option<std::time::Instant> = None;
        let mut last_hover_state: Option<bool> = None;
        let mut auto_hide_show_time: Option<std::time::Instant> = None; // Grace period after triggering show
        let mut last_fullscreen_check = std::time::Instant::now() - std::time::Duration::from_millis(250);
        let mut cached_fullscreen = false;
        let hover_debug_enabled = std::env::var("RAVEN_HOVER_DEBUG")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let mut subclass_tick = 0u32;
        snap_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(16),
            move || {
                let hover_open_delay = std::time::Duration::from_millis(HOVER_OPEN_DELAY_MS.load(Ordering::SeqCst) as u64);
                let hover_close_delay = std::time::Duration::from_millis(HOVER_CLOSE_DELAY_MS.load(Ordering::SeqCst) as u64);
                if let Some(pill) = snap_ui_weak.upgrade() {
                    let hwnd_val = window::PILL_HWND.load(std::sync::atomic::Ordering::SeqCst);
                    let hwnd = if hwnd_val != 0 {
                        let h = windows::Win32::Foundation::HWND(hwnd_val);
                        if subclass_tick % 100 == 0 {
                            unsafe {
                                window::setup_drag_drop_subclass_recursive(h);
                            }
                        }
                        subclass_tick = subclass_tick.wrapping_add(1);
                        // Register drop target if not done yet (handles case where
                        // window_handle() returned NotSupported and the HWND was
                        // stored directly from the initial setup path)
                        if !drop_target_registered {
                            drop_target_registered = true;
                            unsafe {
                                window::register_custom_drop_target(h, drop_ui_weak.clone(), shelf_for_drop.clone());
                            }
                        }
                        h
                    } else {
                        // Attempt to resolve HWND by title
                        unsafe {
                            let title = wide("Raven Notch Pill");
                            let resolved = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                                None,
                                windows::core::PCWSTR(title.as_ptr()),
                            );
                            if resolved.0 != 0 {
                                println!("[SETUP] HWND successfully resolved via FindWindowW: {:?}", resolved);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                                window::PILL_HWND.store(resolved.0, std::sync::atomic::Ordering::SeqCst);
                                let scale = pill.window().scale_factor();
                                window::setup_slint_window_positioning(
                                    resolved,
                                    logical_offset_x,
                                    logical_offset_y,
                                    logical_idle_w,
                                    logical_idle_h,
                                    scale,
                                );
                                window::update_appbar_reservation();
                                // Register OLE drop target on the resolved HWND
                                if !drop_target_registered {
                                    drop_target_registered = true;
                                    window::register_custom_drop_target(resolved, drop_ui_weak.clone(), shelf_for_drop.clone());
                                }
                                resolved
                            } else {
                                return;
                            }
                        }
                    };

                    unsafe {
                        let is_fs = crate::window::IS_FOREGROUND_FULLSCREEN.load(Ordering::SeqCst);
                        if SETTINGS_WINDOW_OPEN.load(Ordering::SeqCst) && !is_fs {
                            use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE};
                            let _ = SetWindowPos(
                                hwnd,
                                windows::Win32::Foundation::HWND(-1), // HWND_TOPMOST
                                0, 0, 0, 0,
                                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                            );
                        }

                        let mut pt_screen = windows::Win32::Foundation::POINT::default();
                        let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt_screen);

                        let is_currently_hovered = pill.get_is_hovered();

                        let monitor = if !is_currently_hovered {
                            windows::Win32::Graphics::Gdi::MonitorFromPoint(
                                pt_screen,
                                windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
                            )
                        } else {
                            windows::Win32::Graphics::Gdi::MonitorFromWindow(
                                hwnd,
                                windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
                            )
                        };

                        let mut info = windows::Win32::Graphics::Gdi::MONITORINFO {
                            cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
                            ..Default::default()
                        };
                        if windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info).as_bool() {
                            let mut scale = pill.window().scale_factor();
                            let mut dpi_x = 0;
                            let mut dpi_y = 0;
                            let _ = windows::Win32::UI::HiDpi::GetDpiForMonitor(
                                monitor,
                                windows::Win32::UI::HiDpi::MDT_EFFECTIVE_DPI,
                                &mut dpi_x,
                                &mut dpi_y,
                            );
                            if dpi_x > 0 {
                                scale = dpi_x as f32 / 96.0;
                            }
                            window::store_slint_scale_factor(scale);
                            
                            let current_idle_w = pill.get_idle_width();
                            let current_idle_h = pill.get_idle_height();
                            let current_offset_x = crate::window::PILL_LOGICAL_OFFSET_X.load(Ordering::SeqCst) as f32;
                            let current_offset_y = crate::window::PILL_LOGICAL_OFFSET_Y.load(Ordering::SeqCst) as f32;

                            let full_width = crate::window::PILL_FULL_WIDTH_BAR.load(Ordering::SeqCst);

                            let comp_w = if full_width {
                                info.rcMonitor.right - info.rcMonitor.left
                            } else {
                                ((f32::max(720.0, current_idle_w) + 24.0) * scale).round() as i32
                            };

                            let show_cal = crate::window::PILL_SHOW_CALENDAR_DROPDOWN.load(Ordering::SeqCst);
                            let show_timer = crate::window::PILL_SHOW_TIMER_DROPDOWN.load(Ordering::SeqCst);
                            let show_vol = crate::window::PILL_SHOW_VOLUME_DROPDOWN.load(Ordering::SeqCst);
                            let show_raven = crate::window::PILL_SHOW_RAVEN_MENU.load(Ordering::SeqCst);
                            let show_wifi = crate::window::PILL_SHOW_WIFI_DROPDOWN.load(Ordering::SeqCst);
                            let show_stats = crate::window::PILL_SHOW_TOPBAR_STATS_DROPDOWN.load(Ordering::SeqCst);
                            let show_clip = crate::window::PILL_SHOW_CLIPBOARD_DROPDOWN.load(Ordering::SeqCst);

                            let mut target_logical_h = f32::max(244.0, current_idle_h);
                            if show_cal {
                                target_logical_h = 560.0;
                            } else if show_timer {
                                target_logical_h = 300.0;
                            } else if show_vol {
                                target_logical_h = f32::max(244.0, 160.0 + current_idle_h);
                            } else if show_raven {
                                target_logical_h = f32::max(244.0, 180.0 + current_idle_h);
                            } else if show_wifi {
                                target_logical_h = f32::max(244.0, 364.0 + current_idle_h);
                            } else if show_stats {
                                target_logical_h = f32::max(244.0, 228.0 + current_idle_h);
                            } else if show_clip {
                                target_logical_h = f32::max(244.0, 336.0 + current_idle_h);
                            }
                            let comp_h = (target_logical_h * scale).round() as i32;
                            let offset_x_phys = if full_width { 0 } else { (current_offset_x * scale).round() as i32 };
                            let offset_y_phys = (current_offset_y * scale).round() as i32;
                            
                            let monitor_bounds = monitor_math::MonitorBounds {
                                left: info.rcMonitor.left,
                                top: info.rcMonitor.top,
                                right: info.rcMonitor.right,
                                bottom: info.rcMonitor.bottom,
                            };

                            let auto_hide_static = APPEARANCE_AUTO_HIDE.load(Ordering::SeqCst);
                            let auto_hide_on_fs_setting = APPEARANCE_AUTO_HIDE_ON_FULLSCREEN.load(Ordering::SeqCst);
                            if auto_hide_on_fs_setting
                                && last_fullscreen_check.elapsed() >= std::time::Duration::from_millis(250)
                            {
                                cached_fullscreen = is_foreground_window_fullscreen();
                                last_fullscreen_check = std::time::Instant::now();
                            } else if !auto_hide_on_fs_setting {
                                cached_fullscreen = false;
                            }
                            let is_fs_for_pos = cached_fullscreen;
                            let old_fs = crate::window::IS_FOREGROUND_FULLSCREEN.load(Ordering::SeqCst);
                            if is_fs_for_pos != old_fs {
                                let cur_hwnd = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
                                println!("[FULLSCREEN-DEBUG] state changed from {} to {}. Active HWND: {:?}", old_fs, is_fs_for_pos, cur_hwnd);
                            }
                            let dynamic_auto_hide_for_pos = auto_hide_static || is_fs_for_pos;
                            crate::window::IS_FOREGROUND_FULLSCREEN.store(is_fs_for_pos, Ordering::SeqCst);
                            let effective_offset_y_phys = if dynamic_auto_hide_for_pos { 0 } else { offset_y_phys };

                            let (x, y) = if full_width {
                                (info.rcMonitor.left, info.rcMonitor.top + effective_offset_y_phys)
                            } else {
                                monitor_math::calculate_notch_center_pos(
                                    monitor_bounds,
                                    comp_w,
                                    offset_x_phys,
                                    effective_offset_y_phys,
                                )
                            };
                            
                            // Store the target rectangle to subclass so it is enforced dynamically
                            window::store_slint_target_rect(x, y, comp_w, comp_h);
                            
                            window::snap_slint_window_to_top_center(hwnd);
                            
                            let is_expanded = pill.get_is_expanded();
                            let vis_w_phys = (pill.get_visible_width() * scale).round() as i32;
                            let vis_h_phys = (pill.get_visible_height() * scale).round() as i32;
                            let vis_y_phys = (pill.get_visible_y() * scale).round() as i32;
                            let padding = (10.0 * scale).round() as i32; // 10px forgiving margin
                            
                            let disable_hover = false;
                            
                            // ── AUTO-HIDE HOVER DETECTION ──
                            // Three distinct cases to avoid race conditions with Slint animations:
                            //   1. auto_hide + hidden  → edge detection at top of screen
                            //   2. auto_hide + showing → stable TARGET bounds (no animated values!)
                            //   3. normal / expanded   → actual visible bounds
                            
                            let dynamic_auto_hide = dynamic_auto_hide_for_pos;
                            pill.set_auto_hide(dynamic_auto_hide);

                            let is_any_dropdown_open = show_cal || show_timer || show_vol || show_wifi || show_stats || show_clip || show_raven;
                            let auto_hide_on = dynamic_auto_hide && !is_expanded && !is_any_dropdown_open;
                            let is_currently_hovered = pill.get_is_hovered();
                            
                            let is_hovering = if disable_hover {
                                false
                            } else if auto_hide_on {
                                let edge_hit = (crate::motion::AUTO_HIDE_EDGE_HIT_PX * scale).round() as i32;
                                monitor_math::check_hover(
                                    monitor_math::CursorPos { x: pt_screen.x, y: pt_screen.y },
                                    monitor_bounds,
                                    scale,
                                    current_idle_w,
                                    current_idle_h,
                                    offset_x_phys,
                                    offset_y_phys,
                                    is_currently_hovered,
                                    padding,
                                    edge_hit,
                                    full_width,
                                )
                            } else {
                                // STATE 3: NORMAL MODE or EXPANDED — use actual visible bounds
                                let left = x + (comp_w - vis_w_phys) / 2;
                                let right = left + vis_w_phys;
                                let top = y;
                                let bottom = y + vis_y_phys + vis_h_phys;
                                
                                pt_screen.x >= left - padding && pt_screen.x <= right + padding
                                    && pt_screen.y >= top && pt_screen.y <= bottom + padding
                            };

                            let is_hovering_notch = if disable_hover {
                                false
                            } else if auto_hide_on {
                                let edge_hit = (crate::motion::AUTO_HIDE_EDGE_HIT_PX * scale).round() as i32;
                                monitor_math::check_hover(
                                    monitor_math::CursorPos { x: pt_screen.x, y: pt_screen.y },
                                    monitor_bounds,
                                    scale,
                                    current_idle_w,
                                    current_idle_h,
                                    offset_x_phys,
                                    offset_y_phys,
                                    is_currently_hovered,
                                    padding,
                                    edge_hit,
                                    false, // ALWAYS false for center-notch only hover!
                                )
                            } else {
                                // STATE 3: NORMAL MODE or EXPANDED — use actual visible bounds
                                let left = x + (comp_w - vis_w_phys) / 2;
                                let right = left + vis_w_phys;
                                let top = y;
                                let bottom = y + vis_y_phys + vis_h_phys;
                                
                                pt_screen.x >= left - padding && pt_screen.x <= right + padding
                                    && pt_screen.y >= top && pt_screen.y <= bottom + padding
                            };
                            
                            // Grace period: after triggering auto-hide show, keep hovered for
                            // at least 350ms to let the Slint animation complete. This prevents
                            // any residual timing issues on extremely slow hardware.
                            let in_grace_period = if auto_hide_on {
                                if let Some(show_time) = auto_hide_show_time {
                                    show_time.elapsed() < std::time::Duration::from_millis(350)
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                            
                            let widget_drag_active = widgets::WIDGET_DRAG_ACTIVE
                                .load(std::sync::atomic::Ordering::SeqCst);
                            let effective_hovering =
                                is_hovering || in_grace_period || widget_drag_active;

                            if auto_hide_on && hover_debug_enabled {
                                println!("[HOVER-DEBUG] Cursor: ({}, {}), is_hovering: {}, in_grace: {}, drag: {}, eff_hover: {}",
                                    pt_screen.x, pt_screen.y, is_hovering, in_grace_period, widget_drag_active, effective_hovering);
                            }

                            // --- DETAILED TELEMETRY LOGGING (RAVEN_MONITOR_DEBUG=1) ---
                            let monitor_debug = std::env::var("RAVEN_MONITOR_DEBUG")
                                .map(|v| v == "1" || v.to_lowercase() == "true")
                                .unwrap_or(false);
                            
                            if monitor_debug {
                                let monitor_under_cursor = windows::Win32::Graphics::Gdi::MonitorFromPoint(
                                    pt_screen,
                                    windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
                                );
                                let mut cursor_mon_info = windows::Win32::Graphics::Gdi::MONITORINFO {
                                    cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
                                    ..Default::default()
                                };
                                let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor_under_cursor, &mut cursor_mon_info);
                                let cursor_mon_bounds = cursor_mon_info.rcMonitor;

                                let mut dpi_x = 0;
                                let mut dpi_y = 0;
                                let _ = windows::Win32::UI::HiDpi::GetDpiForMonitor(
                                    monitor_under_cursor,
                                    windows::Win32::UI::HiDpi::MDT_EFFECTIVE_DPI,
                                    &mut dpi_x,
                                    &mut dpi_y,
                                );
                                let cursor_mon_scale = if dpi_x > 0 { dpi_x as f32 / 96.0 } else { 1.0 };

                                let mut win_rect = windows::Win32::Foundation::RECT::default();
                                let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut win_rect);
                                let ex_style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE) as u32;
                                let is_transparent = (ex_style & windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT.0) != 0;
                                let is_layered = (ex_style & windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED.0) != 0;

                                let notch_phase_str = pill.get_notch_phase();
                                let raven_state = if is_expanded {
                                    if notch_phase_str == "opening" || notch_phase_str == "open_content_staging" {
                                        "Expanding"
                                    } else if notch_phase_str == "open" {
                                        "Visible"
                                    } else if notch_phase_str == "closing_content" || notch_phase_str == "closing" {
                                        "Collapsing"
                                    } else {
                                        "Visible"
                                    }
                                } else {
                                    if auto_hide_on {
                                        if is_currently_hovered {
                                            "Visible"
                                        } else {
                                            "Hidden"
                                        }
                                    } else {
                                        "Visible"
                                    }
                                };

                                println!("[HOVER-POLL] Cursor: ({}, {}), MonUnderCursor: {:?}, MonBounds: [L:{}, T:{}, R:{}, B:{}], State: {}, WinRect: [L:{}, T:{}, R:{}, B:{}], TargetRect: [L:{}, T:{}, R:{}, B:{}], ExStyle: 0x{:X}, Transp: {}, Layered: {}, Hovering: {}, EffHovering: {}, MonScale: {:.2}, NotchPhase: {}",
                                         pt_screen.x, pt_screen.y,
                                         monitor_under_cursor.0,
                                         cursor_mon_bounds.left, cursor_mon_bounds.top, cursor_mon_bounds.right, cursor_mon_bounds.bottom,
                                         raven_state,
                                         win_rect.left, win_rect.top, win_rect.right, win_rect.bottom,
                                         x, y, x + comp_w, y + comp_h, // Target window rect
                                         ex_style,
                                         is_transparent,
                                         is_layered,
                                         is_hovering,
                                         effective_hovering,
                                         cursor_mon_scale,
                                         notch_phase_str);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                                            
                            if effective_hovering {
                                hover_out_time = None;
                                // Mouse entered! Reset hotkey flag so moving mouse out will close as normal
                                crate::window::HOTKEY_OPENED.store(false, Ordering::SeqCst);
                                if !is_expanded {
                                    if is_hovering_notch {
                                        if hover_in_time.is_none() {
                                            hover_in_time = Some(std::time::Instant::now());
                                        } else if hover_in_time.unwrap().elapsed() >= hover_open_delay {
                                            if HOVER_ENABLED.load(Ordering::SeqCst) {
                                                pill.invoke_request_notch_open();
                                            }
                                        }
                                    } else {
                                        hover_in_time = None;
                                    }
                                }

                                if last_hover_state != Some(true) {
                                    pill.set_share_active(false);
                                    pill.set_keep_active(false);
                                    pill.set_is_hovered(true);
                                    last_hover_state = Some(true);
                                    // Record when we triggered the show for grace period
                                    if auto_hide_on {
                                        auto_hide_show_time = Some(std::time::Instant::now());
                                    }
                                }
                            } else {
                                hover_in_time = None;
                                if is_expanded {
                                    // ONLY auto-close if NOT opened by a hotkey toggle
                                    if !crate::window::HOTKEY_OPENED.load(Ordering::SeqCst) {
                                        if HOVER_ENABLED.load(Ordering::SeqCst) {
                                            if hover_out_time.is_none() {
                                                hover_out_time = Some(std::time::Instant::now());
                                            } else if hover_out_time.unwrap().elapsed() >= hover_close_delay {
                                                pill.invoke_request_notch_close();
                                            }
                                        }
                                    }
                                }
                                if last_hover_state != Some(false) {
                                    pill.set_share_active(false);
                                    pill.set_keep_active(false);
                                    pill.set_is_hovered(false);
                                    last_hover_state = Some(false);
                                    auto_hide_show_time = None; // Clear grace period
                                }
                            }
                        }
                    }
                }
            },
        );
        std::mem::forget(snap_timer);
    }

    // Phase 5: 8ms Slint timer reads physics bridge → pushes content_opacity for spring reveal
    let physics_ui_handle = ui.as_weak();
    let physics_timer = slint::Timer::default();
    let mut last_scale = 0.0;
    let mut last_vis_w = 0.0;
    let mut last_expanded = false;
    let mut last_vis_w_phys = -1;
    let mut last_vis_h_phys = -1;
    let mut last_vis_y_phys = -1;
    let mut last_is_split = false;
    let mut last_appbar_scale = 0.0f32;
    let mut last_click_through: Option<bool> = None;
    let waveform_clock_start = std::time::Instant::now();

    physics_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(16),
        move || {
            if let Some(pill) = physics_ui_handle.upgrade() {
                pill.set_waveform_clock_ms(waveform_clock_start.elapsed().as_secs_f32() * 1000.0);

                // Update hit-test variables dynamically from Slint properties
                let hwnd_val = window::PILL_HWND.load(std::sync::atomic::Ordering::SeqCst);
                if hwnd_val != 0 {
                    let vis_w = pill.get_visible_width();
                    let vis_h = pill.get_visible_height();
                    let vis_y = pill.get_visible_y();
                    
                    let scale = pill.window().scale_factor();
                    window::store_slint_scale_factor(scale);
                    if (scale - last_appbar_scale).abs() > f32::EPSILON {
                        window::update_appbar_reservation();
                        last_appbar_scale = scale;
                    }
                    
                    let vis_w_phys = (vis_w * scale).round() as i32;
                    let vis_h_phys = (vis_h * scale).round() as i32;
                    let vis_y_phys = (vis_y * scale).round() as i32;
                    
                    let is_expanded = pill.get_is_expanded();
                    let active_tab = pill.get_active_tab();
                    let is_split = is_expanded && (active_tab == "clock" || active_tab == "media");

                    let dims_changed = vis_w_phys != last_vis_w_phys 
                        || vis_h_phys != last_vis_h_phys 
                        || vis_y_phys != last_vis_y_phys
                        || is_split != last_is_split;

                    if dims_changed {
                        window::PILL_VIS_WIDTH_PHYS.store(vis_w_phys, std::sync::atomic::Ordering::SeqCst);
                        window::PILL_VIS_HEIGHT_PHYS.store(vis_h_phys, std::sync::atomic::Ordering::SeqCst);
                        window::PILL_VIS_Y_PHYS.store(vis_y_phys, std::sync::atomic::Ordering::SeqCst);
                        window::PILL_IS_SPLIT_LAYOUT.store(is_split, std::sync::atomic::Ordering::SeqCst);

                        unsafe {
                            window::update_window_region(windows::Win32::Foundation::HWND(hwnd_val));
                        }
                        
                        last_vis_w_phys = vis_w_phys;
                        last_vis_h_phys = vis_h_phys;
                        last_vis_y_phys = vis_y_phys;
                        last_is_split = is_split;
                    }

                    let click_through = if vis_w_phys <= 0 || vis_h_phys <= 0 {
                        true
                    } else {
                        // Query cursor position
                        let mut pt = windows::Win32::Foundation::POINT::default();
                        unsafe {
                            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
                        }

                        // Get window rect
                        let hwnd = windows::Win32::Foundation::HWND(hwnd_val);
                        let mut rect = windows::Win32::Foundation::RECT::default();
                        unsafe {
                            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
                        }

                        let win_w = rect.right - rect.left;
                        let cx = pt.x - rect.left;
                        let cy = pt.y - rect.top;

                        let full_width_bar = crate::window::PILL_FULL_WIDTH_BAR.load(std::sync::atomic::Ordering::SeqCst);
                        let top_bar_widgets = crate::window::PILL_TOP_BAR_WIDGETS.load(std::sync::atomic::Ordering::SeqCst);

                        let inside_full_width_bar = full_width_bar && {
                            let logical_idle_h = crate::window::PILL_LOGICAL_IDLE_HEIGHT.load(std::sync::atomic::Ordering::SeqCst) as f32;
                            let bar_h = (if is_split { logical_idle_h } else { vis_h_phys as f32 / scale } * scale).round() as i32;
                            if bar_h > 0 && win_w > 0 {
                                cy >= 0 && cy < bar_h && cx >= 0 && cx < win_w
                            } else {
                                false
                            }
                        };

                        let inside_top_bar_widgets = full_width_bar && top_bar_widgets && {
                            let logical_idle_h = crate::window::PILL_LOGICAL_IDLE_HEIGHT.load(std::sync::atomic::Ordering::SeqCst) as f32;
                            let bar_h = (if is_split { logical_idle_h } else { vis_h_phys as f32 / scale } * scale).round() as i32;
                            if bar_h > 0 && win_w > 0 {
                                let top_bar_w = (240.0 * scale).round() as i32;
                                let left = win_w - top_bar_w;
                                let right = win_w;
                                let top = 0;
                                let bottom = bar_h;
                                cx >= left && cx < right && cy >= top && cy < bottom
                            } else {
                                false
                            }
                        };

                        let show_cal = crate::window::PILL_SHOW_CALENDAR_DROPDOWN.load(std::sync::atomic::Ordering::SeqCst);
                        let show_timer = crate::window::PILL_SHOW_TIMER_DROPDOWN.load(std::sync::atomic::Ordering::SeqCst);
                        let show_vol = crate::window::PILL_SHOW_VOLUME_DROPDOWN.load(std::sync::atomic::Ordering::SeqCst);
                        let show_raven = crate::window::PILL_SHOW_RAVEN_MENU.load(std::sync::atomic::Ordering::SeqCst);
                        let show_wifi = crate::window::PILL_SHOW_WIFI_DROPDOWN.load(std::sync::atomic::Ordering::SeqCst);
                        let show_stats = crate::window::PILL_SHOW_TOPBAR_STATS_DROPDOWN.load(std::sync::atomic::Ordering::SeqCst);
                        let show_clip = crate::window::PILL_SHOW_CLIPBOARD_DROPDOWN.load(std::sync::atomic::Ordering::SeqCst);
                        let any_dropdown_open = show_cal || show_timer || show_vol || show_raven || show_wifi || show_stats || show_clip;

                        let inside_ui = if full_width_bar && any_dropdown_open {
                            true
                        } else if inside_full_width_bar || inside_top_bar_widgets {
                            true
                        } else if is_split {
                            let panel_width = (720.0 * scale).round() as i32;
                            let panel_left = (win_w - panel_width) / 2;
                            let cx_rel = cx - panel_left;

                            let in_header = cx_rel >= (4.0 * scale).round() as i32 
                                && cx_rel <= (692.0 * scale).round() as i32 
                                && cy >= (10.0 * scale).round() as i32 
                                && cy <= (38.0 * scale).round() as i32;

                            let in_left_panel = cx_rel >= (4.0 * scale).round() as i32 
                                && cx_rel <= (348.0 * scale).round() as i32 
                                && cy >= (50.0 * scale).round() as i32 
                                && cy <= (216.0 * scale).round() as i32;

                            let in_right_panel = cx_rel >= (432.0 * scale).round() as i32 
                                && cx_rel <= (692.0 * scale).round() as i32 
                                && cy >= (50.0 * scale).round() as i32 
                                && cy <= (216.0 * scale).round() as i32;

                            in_header || in_left_panel || in_right_panel
                        } else {
                            let inside_dropdown = if any_dropdown_open {
                                let idle_h_phys = (crate::window::PILL_LOGICAL_IDLE_HEIGHT.load(std::sync::atomic::Ordering::SeqCst) as f32 * scale).round() as i32;
                                if show_cal {
                                    let left = win_w - (356.0 * scale).round() as i32;
                                    let right = win_w - (16.0 * scale).round() as i32;
                                    let top = idle_h_phys;
                                    let bottom = idle_h_phys + (510.0 * scale).round() as i32;
                                    cx >= left && cx <= right && cy >= top && cy <= bottom
                                } else if show_timer {
                                    let left = win_w - (256.0 * scale).round() as i32;
                                    let right = win_w - (16.0 * scale).round() as i32;
                                    let top = idle_h_phys;
                                    let bottom = idle_h_phys + (200.0 * scale).round() as i32;
                                    cx >= left && cx <= right && cy >= top && cy <= bottom
                                } else if show_vol {
                                    let left = win_w - (256.0 * scale).round() as i32;
                                    let right = win_w - (16.0 * scale).round() as i32;
                                    let top = idle_h_phys;
                                    let bottom = idle_h_phys + (110.0 * scale).round() as i32;
                                    cx >= left && cx <= right && cy >= top && cy <= bottom
                                } else if show_raven {
                                    let left = (8.0 * scale).round() as i32;
                                    let right = left + (176.0 * scale).round() as i32;
                                    let top = idle_h_phys + (6.0 * scale).round() as i32;
                                    let bottom = top + (170.0 * scale).round() as i32;
                                    cx >= left && cx <= right && cy >= top && cy <= bottom
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if win_w > 0 {
                                let left = (win_w - vis_w_phys) / 2;
                                let right = left + vis_w_phys;
                                let top = vis_y_phys;
                                let bottom = top + vis_h_phys;
                                (cx >= left && cx < right && cy >= top && cy < bottom) || inside_dropdown
                            } else {
                                false
                            }
                        };

                        // Check if left mouse button is held down (indicating drag/drop in progress)
                        let lbutton_down = unsafe {
                            (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                                windows::Win32::UI::Input::KeyboardAndMouse::VK_LBUTTON.0 as i32,
                            ) as u16 & 0x8000) != 0
                        };

                        if lbutton_down && inside_ui {
                            false
                        } else {
                            !inside_ui
                        }
                    };

                    if last_click_through != Some(click_through) {
                        unsafe {
                            window::set_window_click_through(windows::Win32::Foundation::HWND(hwnd_val), click_through);
                        }
                        last_click_through = Some(click_through);
                    }


                    if scale != last_scale || vis_w != last_vis_w || is_expanded != last_expanded {
                        println!("[TELEMETRY] scale: {}, vis_w: {}, vis_h: {}, is_expanded: {}, window_size: {:?}", scale, vis_w, vis_h, is_expanded, pill.window().size());
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                        last_scale = scale;
                        last_vis_w = vis_w;
                        last_expanded = is_expanded;
                    }
                }
            }
        },
    );
    std::mem::forget(physics_timer);
    std::mem::forget(live_timer);


    // Listen for global events
    let settings_ui_handle = settings_ui.as_weak();
    let main_ui_weak_for_event = ui.as_weak();
    events.subscribe(move |event| {
        if let events::RavenEvent::ShowSettings = event {
            let settings_ui = settings_ui_handle.clone();
            let main_ui_weak = main_ui_weak_for_event.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = settings_ui.upgrade() {
                    // Hide first to avoid flash at default position
                    ui.hide().unwrap();
                    center_settings_window(&ui);
                    SETTINGS_WINDOW_OPEN.store(true, Ordering::SeqCst);
                    ui.show().unwrap();

                    static SUBCLASSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                    if !SUBCLASSED.load(std::sync::atomic::Ordering::SeqCst) {
                        let mut resolved_hwnd = None;
                        use raw_window_handle::HasWindowHandle;
                        if let Ok(handle) = ui.window().window_handle().window_handle() {
                            if let raw_window_handle::RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                                resolved_hwnd = Some(windows::Win32::Foundation::HWND(win32_handle.hwnd.get() as _));
                            }
                        }
                        if resolved_hwnd.is_none() {
                            unsafe {
                                let title = wide("Raven Settings");
                                let hwnd = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                                    None,
                                    windows::core::PCWSTR(title.as_ptr()),
                                );
                                if hwnd.0 != 0 {
                                    resolved_hwnd = Some(hwnd);
                                }
                            }
                        }
                        if let Some(hwnd) = resolved_hwnd {
                            unsafe {
                                window::setup_settings_window_subclass(hwnd, ui.as_weak());
                            }
                            SUBCLASSED.store(true, std::sync::atomic::Ordering::SeqCst);
                            println!("[SETTINGS-LOG] Subclassed settings window successfully!");
                        }
                    }
                }
                if let Some(main_ui) = main_ui_weak.upgrade() {
                    main_ui.invoke_request_notch_close();
                }
            });
        }
        if let events::RavenEvent::AccountTokenReceived(token) = event {
            let settings_ui = settings_ui_handle.clone();
            let main_ui_weak = main_ui_weak_for_event.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(settings_ui) = settings_ui.upgrade() {
                    settings_ui.set_license_action_message("Connecting account...".into());
                    let ui_weak = main_ui_weak.clone();
                    let settings_ui_weak = settings_ui.as_weak();
                    std::thread::spawn(move || {
                        let result = license::connect_account_token(token);
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(settings_ui) = settings_ui_weak.upgrade() {
                                match result {
                                    Ok(status) => {
                                        if let Some(ui) = ui_weak.upgrade() {
                                            apply_license_status(&ui, &settings_ui, &status);
                                        }
                                    }
                                    Err(error) => {
                                        settings_ui.set_license_action_message(
                                            format!("Account sign-in failed: {error}").into(),
                                        );
                                    }
                                }
                            }
                        });
                    });
                }
            });
        }
    });

    // Initialize and hide settings window to prevent freezes later.
    // Position it off the top of screen first so the warmup flash (if any)
    // cannot appear over the notch area.
    settings_ui.window().set_position(slint::PhysicalPosition::new(0, 2000));
    settings_ui.show().unwrap();

    // Set up subclassing for settings window layout/resize lifecycle logging
    let mut resolved_hwnd = None;
    if let Ok(handle) = settings_ui.window().window_handle().window_handle() {
        if let raw_window_handle::RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
            resolved_hwnd = Some(windows::Win32::Foundation::HWND(win32_handle.hwnd.get() as _));
        }
    }
    if resolved_hwnd.is_none() {
        unsafe {
            let title = wide("Raven Settings");
            let hwnd = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                None,
                windows::core::PCWSTR(title.as_ptr()),
            );
            if hwnd.0 != 0 {
                resolved_hwnd = Some(hwnd);
            }
        }
    }
    if let Some(hwnd) = resolved_hwnd {
        unsafe {
            window::setup_settings_window_subclass(hwnd, settings_ui.as_weak());
        }
    }

    settings_ui.hide().unwrap();

    slint::run_event_loop().unwrap();
}

fn get_widget_dimensions(size: &str) -> (i32, i32) {
    match size {
        "S" => (180, 104),
        "M" => (240, 138),
        "L" => (320, 184),
        "XL" => (400, 230),
        _ => (240, 138),
    }
}

fn progress_widget_copy_values(kind: &str) -> Option<(String, String, f32, String)> {
    use chrono::{Datelike, TimeZone, Timelike};

    let now = chrono::Local::now();
    match kind {
        "year_progress" => {
            let year = now.year();
            let start = chrono::Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
            let end = chrono::Local.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap();
            let elapsed = now.signed_duration_since(start).num_milliseconds();
            let total = end.signed_duration_since(start).num_milliseconds();
            let progress = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
            let days_passed = now.signed_duration_since(start).num_days() + 1;
            let days_total = end.signed_duration_since(start).num_days();
            Some((
                now.format("%b %e").to_string().trim().to_string(),
                format!("{:.1}%", progress * 100.0),
                progress as f32,
                format!("{}/{} Days", days_passed, days_total),
            ))
        }
        "day_progress" => {
            let time = now.time();
            let elapsed_ms =
                time.num_seconds_from_midnight() as i64 * 1000 + (time.nanosecond() / 1_000_000) as i64;
            let total_ms = 24 * 60 * 60 * 1000;
            let progress = (elapsed_ms as f64 / total_ms as f64).clamp(0.0, 1.0);
            let hours = elapsed_ms / (60 * 60 * 1000);
            let mins = (elapsed_ms % (60 * 60 * 1000)) / (60 * 1000);
            Some((
                now.format("%b %e").to_string().trim().to_string(),
                format!("{:.1}%", progress * 100.0),
                progress as f32,
                format!("{}h {}m / 24h", hours, mins),
            ))
        }
        "month_progress" => {
            let year = now.year();
            let month = now.month();
            let start = chrono::Local.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
            let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
            let end = chrono::Local
                .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
                .unwrap();
            let elapsed = now.signed_duration_since(start).num_milliseconds();
            let total = end.signed_duration_since(start).num_milliseconds();
            let progress = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
            let days_passed = now.signed_duration_since(start).num_days() + 1;
            let days_total = end.signed_duration_since(start).num_days();
            Some((
                now.format("%B").to_string(),
                format!("{:.1}%", progress * 100.0),
                progress as f32,
                format!("{}/{} Days", days_passed, days_total),
            ))
        }
        _ => None,
    }
}

fn apply_generic_widget_copy_values(w: &GenericWidgetWindow, kind: &str) {
    if let Some((date, pct, value, detail)) = progress_widget_copy_values(kind) {
        w.set_copy_date_str(date.into());
        w.set_copy_pct_str(pct.into());
        w.set_copy_pct_val(value);
        w.set_copy_detail_str(detail.into());
    }
}

fn update_year_progress_widget_properties(w: &YearProgressWidgetWindow) {
    use chrono::Datelike;
    use chrono::TimeZone;
    let now = chrono::Local::now();
    let year = now.year();
    
    // Year Progress Logic
    let start = chrono::Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let end = chrono::Local.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap();
    let elapsed = now.signed_duration_since(start).num_milliseconds();
    let total = end.signed_duration_since(start).num_milliseconds();
    let progress = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
    
    let pct = progress * 100.0;
    let pct_str = format!("{:.1}%", pct);
    
    let days_passed = now.signed_duration_since(start).num_days() + 1;
    let days_total = end.signed_duration_since(start).num_days();
    let days_str = format!("{}/{} Days", days_passed, days_total);
    
    let date_str = now.format("%b %e").to_string(); // e.g. "Jun  8"
    
    w.set_pct_val(progress as f32);
    w.set_pct_str(pct_str.into());
    w.set_days_str(days_str.into());
    w.set_date_str(date_str.trim().into());
}

fn update_day_progress_widget_properties(w: &DayProgressWidgetWindow) {
    use chrono::Timelike;
    let now = chrono::Local::now();
    let time = now.time();
    
    let elapsed_ms = time.num_seconds_from_midnight() as i64 * 1000 + (time.nanosecond() / 1_000_000) as i64;
    let total_ms = 24 * 60 * 60 * 1000;
    let progress = (elapsed_ms as f64 / total_ms as f64).clamp(0.0, 1.0);
    
    let pct = progress * 100.0;
    let pct_str = format!("{:.1}%", pct);
    
    let hours = elapsed_ms / (60 * 60 * 1000);
    let mins = (elapsed_ms % (60 * 60 * 1000)) / (60 * 1000);
    let hours_str = format!("{}h {}m / 24h", hours, mins);
    
    let date_str = now.format("%b %e").to_string(); // e.g. "Jun  9"
    
    w.set_pct_val(progress as f32);
    w.set_pct_str(pct_str.into());
    w.set_hours_str(hours_str.into());
    w.set_date_str(date_str.trim().into());
}

fn update_month_progress_widget_properties(w: &MonthProgressWidgetWindow) {
    use chrono::Datelike;
    use chrono::TimeZone;
    let now = chrono::Local::now();
    let year = now.year();
    let month = now.month();
    
    let start = chrono::Local.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
    
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let end = chrono::Local.with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0).unwrap();
    
    let elapsed = now.signed_duration_since(start).num_milliseconds();
    let total = end.signed_duration_since(start).num_milliseconds();
    let progress = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
    
    let pct = progress * 100.0;
    let pct_str = format!("{:.1}%", pct);
    
    let days_passed = now.signed_duration_since(start).num_days() + 1;
    let days_total = end.signed_duration_since(start).num_days();
    let days_str = format!("{}/{} Days", days_passed, days_total);
    
    let month_name = now.format("%B").to_string(); // e.g. "June"
    
    w.set_pct_val(progress as f32);
    w.set_pct_str(pct_str.into());
    w.set_days_str(days_str.into());
    w.set_month_name(month_name.into());
}

fn update_media_widget_properties(w: &MediaWidgetWindow, snapshot: &crate::services::RuntimeSnapshot, local_pos: f64) {
    if snapshot.media.has_media && !snapshot.media.title.trim().is_empty() {
        w.set_track_title(snapshot.media.title.clone().into());
        w.set_track_artist(snapshot.media.artist.clone().into());
        w.set_is_playing(snapshot.media.is_playing);
        
        let total_sec = snapshot.media.duration_seconds;
        let progress = (local_pos / total_sec.max(1.0)).clamp(0.0, 1.0) as f32;
        w.set_progress(progress);
        
        let elapsed_sec = local_pos.round() as i32;
        let total_sec_i32 = total_sec as i32;
        let remaining_sec = (total_sec_i32 - elapsed_sec).max(0);
        
        w.set_time_elapsed(format!("{}:{:02}", elapsed_sec / 60, elapsed_sec % 60).into());
        w.set_time_remaining(format!("-{}:{:02}", remaining_sec / 60, remaining_sec % 60).into());
        
        if !snapshot.media.album_art_path.is_empty() {
            if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&snapshot.media.album_art_path)) {
                w.set_album_art(img);
                w.set_has_album_art(true);
            } else {
                w.set_has_album_art(false);
            }
        } else {
            w.set_has_album_art(false);
        }
        
        if !snapshot.media.source_icon_path.is_empty() {
            if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&snapshot.media.source_icon_path)) {
                w.set_app_icon(img);
                w.set_has_app_icon(true);
            } else {
                w.set_has_app_icon(false);
            }
        } else {
            w.set_has_app_icon(false);
        }
    } else {
        w.set_track_title("Love On The Brain".into());
        w.set_track_artist("Rihanna".into());
        w.set_is_playing(false);
        w.set_progress(0.11);
        w.set_time_elapsed("0:18".into());
        w.set_time_remaining("-2:24".into());
        
        let fallback_art_path = std::path::Path::new("ui/assets/rihanna_art.png");
        if fallback_art_path.exists() {
            if let Ok(img) = slint::Image::load_from_path(fallback_art_path) {
                w.set_album_art(img);
                w.set_has_album_art(true);
            } else {
                w.set_has_album_art(false);
            }
        } else {
            w.set_has_album_art(false);
        }
        
        w.set_has_app_icon(false);
    }
}

unsafe fn save_current_widget_position_if_active(stats_w: &std::rc::Rc<std::cell::RefCell<Vec<StatsWidgetWindow>>>) {
    let stats_guard = stats_w.borrow();
    for (idx, w) in stats_guard.iter().enumerate() {
        use raw_window_handle::HasWindowHandle;
        if let Ok(handle) = w.window().window_handle().window_handle() {
            if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as _);
                if hwnd.0 != 0 {
                    {
                        let positioned = crate::widgets::POSITIONED_HWNDS.lock().unwrap();
                        if !positioned.contains(&hwnd.0) {
                            continue;
                        }
                    }
                    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
                    use windows::Win32::Graphics::Gdi::{MonitorFromWindow, GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTONEAREST};
                    use windows::Win32::UI::HiDpi::GetDpiForWindow;
                    use windows::Win32::Foundation::RECT;

                    let dpi = GetDpiForWindow(hwnd);
                    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
                    
                    let mut rect = RECT::default();
                    if GetWindowRect(hwnd, &mut rect).is_ok() {
                        let phys_width = rect.right - rect.left;
                        let phys_height = rect.bottom - rect.top;
                        
                        if phys_width > 0 && phys_height > 0 && rect.left > -10000 {
                            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                            let mut info = MONITORINFO {
                                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                ..Default::default()
                            };
                            let _ = GetMonitorInfoW(monitor, &mut info);
                            let screen_width = info.rcMonitor.right - info.rcMonitor.left;
                            let screen_height = info.rcMonitor.bottom - info.rcMonitor.top;
                            
                            let mut phys_pos_x = info.rcMonitor.left + screen_width - phys_width - rect.left;
                            let mut phys_pos_y = rect.top - info.rcMonitor.top;

                            let max_x = (screen_width - phys_width).max(0);
                            let max_y = (screen_height - phys_height).max(0);
                            phys_pos_x = phys_pos_x.clamp(0, max_x);
                            phys_pos_y = phys_pos_y.clamp(0, max_y);
                            
                            let pos_x = (phys_pos_x as f32 / scale).round() as i32;
                            let pos_y = (phys_pos_y as f32 / scale).round() as i32;
                            
                            let current = crate::settings::RavenSettings::load();
                            let inst = current.widgets.get_clock_instance(idx);
                            
                            if (inst.pos_x - pos_x as f64).abs() > 0.5 
                                || (inst.pos_y - pos_y as f64).abs() > 0.5 
                            {
                                crate::settings::update_clock_instance_setting(idx, |instance| {
                                    instance.pos_x = pos_x as f64;
                                    instance.pos_y = pos_y as f64;
                                });
                                println!("[WIDGET-DEBUG] save_current_widget_position_if_active: successfully saved widget {} position to x={}, y={}", idx, pos_x, pos_y);
                            }
                        }
                    }
                }
            }
        }
    }
}

unsafe fn save_current_extra_widget_positions(
    instance_widgets: &Rc<RefCell<HashMap<String, ExtraWidgetWindow>>>,
) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let settings = settings::RavenSettings::load();
    let widgets_guard = instance_widgets.borrow();
    for instance in &settings.widgets.instances {
        let Some(widget) = widgets_guard.get(&instance.id) else {
            continue;
        };
        let Some(hwnd) = widget.hwnd() else {
            continue;
        };
        if hwnd.0 == 0 {
            continue;
        }

        // Skip widgets that haven't been explicitly positioned yet.
        // On startup winit creates windows at a default location (0,0 / cascaded);
        // the real saved position is applied 50 ms later.  If the 1-second clock
        // timer fires before that 50 ms, we must NOT overwrite the saved coords
        // with the startup default.
        {
            let positioned = crate::widgets::POSITIONED_HWNDS.lock().unwrap();
            if !positioned.contains(&hwnd.0) {
                continue;
            }
        }

        let dpi = GetDpiForWindow(hwnd);
        let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            continue;
        }
        let phys_width = rect.right - rect.left;
        let phys_height = rect.bottom - rect.top;
        if phys_width <= 0 || phys_height <= 0 || rect.left <= -10000 {
            continue;
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);

        let pos_x = ((rect.left - info.rcMonitor.left) as f32 / scale).round() as i32;
        let pos_y = ((rect.top - info.rcMonitor.top) as f32 / scale).round() as i32;
        if (instance.x - pos_x).abs() > 1 || (instance.y - pos_y).abs() > 1 {
            settings::update_widget_instance_position(&instance.id, pos_x, pos_y);
        }
    }
}

fn show_focus_timer_context_menu(
    hwnd: windows::Win32::Foundation::HWND,
    widget_weak: slint::Weak<CalendarFocusWidgetWindow>,
    runtime: std::rc::Rc<FocusTimerRuntime>,
    settings_ui_weak: slint::Weak<SettingsWindow>,
    update_widget_lifecycles: std::rc::Rc<dyn Fn()>,
    _stats_widget: std::rc::Rc<std::cell::RefCell<Vec<StatsWidgetWindow>>>,
    _configure_only: bool,
) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let menu = CreatePopupMenu().unwrap();
        let flags = MF_STRING;
        let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, windows::core::w!("SET FOCUS TIMER"));
        let _ = AppendMenuW(menu, flags, 101, windows::core::w!("5 minutes"));
        let _ = AppendMenuW(menu, flags, 102, windows::core::w!("10 minutes"));
        let _ = AppendMenuW(menu, flags, 103, windows::core::w!("15 minutes"));
        let _ = AppendMenuW(menu, flags, 104, windows::core::w!("25 minutes"));
        let _ = AppendMenuW(menu, flags, 105, windows::core::w!("45 minutes"));
        let _ = AppendMenuW(menu, flags, 106, windows::core::w!("60 minutes"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let toggle_text = if runtime.running.get() {
            "Pause Timer"
        } else {
            "Start Timer"
        };
        let toggle_wide: Vec<u16> = toggle_text.encode_utf16().chain(Some(0)).collect();
        let _ = AppendMenuW(menu, flags, 107, windows::core::PCWSTR(toggle_wide.as_ptr()));
        let _ = AppendMenuW(menu, flags, 108, windows::core::w!("Reset Timer"));
        let _ = AppendMenuW(menu, flags, 109, windows::core::w!("Custom Duration..."));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let settings = settings::RavenSettings::load();
        let is_topmost = settings::is_widget_always_on_top(&settings, "calendar_focus");
        let top_text = if is_topmost {
            "Send to Back"
        } else {
            "Always on Top"
        };
        let top_wide: Vec<u16> = top_text.encode_utf16().chain(Some(0)).collect();
        let _ = AppendMenuW(menu, flags, 113, windows::core::PCWSTR(top_wide.as_ptr()));
        let _ = AppendMenuW(menu, flags, 110, windows::core::w!("Reset Position"));
        let _ = AppendMenuW(menu, flags, 111, windows::core::w!("Lock / Unlock Position"));
        let _ = AppendMenuW(menu, flags, 112, windows::core::w!("Close Widget"));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);

        if cmd.0 == 0 {
            return;
        }
        let command = cmd.0;
        slint::Timer::single_shot(std::time::Duration::from_millis(10), move || {
            match command {
                101..=106 => {
                    let minutes = match command {
                        101 => 5,
                        102 => 10,
                        103 => 15,
                        104 => 25,
                        105 => 45,
                        _ => 60,
                    };
                    settings::set_number(&["widgets", "focus_timer_minutes"], minutes as f64);
                    runtime.set_minutes(minutes);
                    if let Some(settings_ui) = settings_ui_weak.upgrade() {
                        settings_ui.set_focus_timer_minutes(minutes);
                    }
                }
                107 => runtime.toggle(),
                108 => runtime.reset(),
                109 => {
                    if let Some(settings_ui) = settings_ui_weak.upgrade() {
                        let _ = settings_ui.hide();
                        center_settings_window(&settings_ui);
                        settings_ui.set_active_tab("widgets".into());
                        settings_ui.set_selected_widget_id("calendar_focus".into());
                        SETTINGS_WINDOW_OPEN.store(true, Ordering::SeqCst);
                        let _ = settings_ui.show();
                    }
                }
                110 => {
                    settings::set_number(&["widgets", "calendar_focus_pos_x"], 720.0);
                    settings::set_number(&["widgets", "calendar_focus_pos_y"], 420.0);
                    update_widget_lifecycles();
                }
                111 => {
                    let settings = settings::RavenSettings::load();
                    let locked = !settings.widgets.locked;
                    settings::set_bool(&["widgets", "locked"], locked);
                    if let Some(settings_ui) = settings_ui_weak.upgrade() {
                        settings_ui.set_widgets_locked(locked);
                    }
                    update_widget_lifecycles();
                }
                112 => {
                    settings::set_bool(&["widgets", "calendar_focus_enabled"], false);
                    if let Some(settings_ui) = settings_ui_weak.upgrade() {
                        settings_ui.set_widgets_calendar_focus_enabled(false);
                        reconcile_widget_order(&settings_ui);
                    }
                    update_widget_lifecycles();
                }
                113 => {
                    let settings = settings::RavenSettings::load();
                    let is_topmost = settings::is_widget_always_on_top(&settings, "calendar_focus");
                    let next_topmost = !is_topmost;
                    settings::set_widget_always_on_top("calendar_focus", next_topmost);
                    unsafe {
                        crate::widgets::apply_widget_topmost_state(hwnd, next_topmost);
                    }
                    update_widget_lifecycles();
                }
                _ => {}
            }

            if let Some(widget) = widget_weak.upgrade() {
                update_calendar_focus_widget_properties(&widget, &runtime);
            }
        });
    }
}

fn show_start_context_menu(
    hwnd: windows::Win32::Foundation::HWND,
    settings_ui_weak: slint::Weak<SettingsWindow>,
    ui_weak: slint::Weak<crate::Pill>,
    motion_state: std::rc::Rc<std::cell::RefCell<crate::MotionState>>,
) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_LWIN};

    unsafe {
        let menu = CreatePopupMenu().unwrap();

        let _ = AppendMenuW(menu, MF_STRING, 1, windows::core::w!("Search Apps"));
        let _ = AppendMenuW(menu, MF_STRING, 2, windows::core::w!("Open File Manager"));
        let _ = AppendMenuW(menu, MF_STRING, 3, windows::core::w!("Open Settings"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, 4, windows::core::w!("Sleep"));
        let _ = AppendMenuW(menu, MF_STRING, 5, windows::core::w!("Restart"));
        let _ = AppendMenuW(menu, MF_STRING, 6, windows::core::w!("Shut Down"));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);

        if cmd.0 != 0 {
            let command = cmd.0;
            slint::Timer::single_shot(std::time::Duration::from_millis(10), move || {
                match command {
                    1 => {
                        // Search Apps: Simulate Win keypress to open Start/Search
                        unsafe {
                            keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
                            keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
                        }
                    }
                    2 => {
                        // Open File Manager
                        let _ = std::process::Command::new("explorer").spawn();
                    }
                    3 => {
                        // Open Settings
                        if let Some(s_ui) = settings_ui_weak.upgrade() {
                            s_ui.hide().unwrap();
                            center_settings_window(&s_ui);
                            SETTINGS_WINDOW_OPEN.store(true, Ordering::SeqCst);
                            s_ui.show().unwrap();
                        }
                        if let Some(ui) = ui_weak.upgrade() {
                            begin_notch_motion(&ui, &motion_state, false);
                        }
                    }
                    4 => {
                        // Sleep: standard sleep via PowerShell API
                        let _ = std::process::Command::new("powershell")
                            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Application]::SetSuspendState([System.Windows.Forms.PowerState]::Suspend, $false, $false)"])
                            .spawn();
                    }
                    5 => {
                        // Restart
                        let _ = std::process::Command::new("shutdown")
                            .args(["/r", "/t", "0"])
                            .spawn();
                    }
                    6 => {
                        // Shut Down
                        let _ = std::process::Command::new("shutdown")
                            .args(["/s", "/t", "0"])
                            .spawn();
                    }
                    _ => {}
                }
            });
        }
    }
}

fn show_app_context_menu(
    hwnd: windows::Win32::Foundation::HWND,
    app_hwnd_val: isize,
) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::{HWND, POINT, WPARAM, LPARAM};

    let app_hwnd = HWND(app_hwnd_val);
    unsafe {
        let menu = CreatePopupMenu().unwrap();
        
        let is_minimized = IsIconic(app_hwnd).as_bool();
        let is_maximized = IsZoomed(app_hwnd).as_bool();

        // 1. Restore
        let mut restore_flags = MF_STRING;
        if !is_minimized && !is_maximized {
            restore_flags |= MF_GRAYED;
        }
        let _ = AppendMenuW(menu, restore_flags, 1, windows::core::w!("Restore"));

        // 2. Minimize
        let mut minimize_flags = MF_STRING;
        if is_minimized {
            minimize_flags |= MF_GRAYED;
        }
        let _ = AppendMenuW(menu, minimize_flags, 2, windows::core::w!("Minimize"));

        // 3. Maximize
        let mut maximize_flags = MF_STRING;
        if is_maximized {
            maximize_flags |= MF_GRAYED;
        }
        let _ = AppendMenuW(menu, maximize_flags, 3, windows::core::w!("Maximize"));

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

        // 4. Close
        let _ = AppendMenuW(menu, MF_STRING, 4, windows::core::w!("Close Window"));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);

        if cmd.0 != 0 {
            let command = cmd.0;
            slint::Timer::single_shot(std::time::Duration::from_millis(10), move || {
                match command {
                    1 => {
                        let _ = ShowWindow(app_hwnd, SW_RESTORE);
                        let _ = SetForegroundWindow(app_hwnd);
                    }
                    2 => {
                        let _ = ShowWindow(app_hwnd, SW_MINIMIZE);
                    }
                    3 => {
                        let _ = ShowWindow(app_hwnd, SW_MAXIMIZE);
                        let _ = SetForegroundWindow(app_hwnd);
                    }
                    4 => {
                        let _ = PostMessageW(app_hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                    _ => {}
                }
            });
        }
    }
}

fn show_native_context_menu(
    hwnd: windows::Win32::Foundation::HWND,
    widget_id: &str,
    clock_idx: Option<usize>,
    settings_ui_weak: slint::Weak<SettingsWindow>,
    update_widget_lifecycles: std::rc::Rc<dyn Fn()>,
    stats_widget: std::rc::Rc<std::cell::RefCell<Vec<StatsWidgetWindow>>>,
) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::POINT;
    
    unsafe {
        let menu = CreatePopupMenu().unwrap();
        
        let flags = MF_STRING;
        let effective_widget_id = clock_idx
            .map(|idx| format!("clock_{}", idx))
            .unwrap_or_else(|| widget_id.to_string());
        
        let _ = AppendMenuW(menu, flags, 1, windows::core::w!("Configure Widget"));
        
        let settings = settings::RavenSettings::load();
        let lock_text = if settings.widgets.locked {
            "Unlock Position (Allow Dragging)"
        } else {
            "Lock Position (Prevent Dragging)"
        };
        let lock_wide: Vec<u16> = lock_text.encode_utf16().chain(Some(0)).collect();
        let _ = AppendMenuW(menu, flags, 2, windows::core::PCWSTR(lock_wide.as_ptr()));
        
        let ct_text = if settings.widgets.click_through {
            "Disable Click-Through"
        } else {
            "Enable Click-Through (Invisible to Clicks)"
        };
        let ct_wide: Vec<u16> = ct_text.encode_utf16().chain(Some(0)).collect();
        let _ = AppendMenuW(menu, flags, 5, windows::core::PCWSTR(ct_wide.as_ptr()));
        
        let _ = AppendMenuW(menu, flags, 4, windows::core::w!("Reset Position"));
        
        let is_topmost = settings::is_widget_always_on_top(&settings, &effective_widget_id);
        let top_text = if is_topmost {
            "Send to Back"
        } else {
            "Always on Top"
        };
        let top_wide: Vec<u16> = top_text.encode_utf16().chain(Some(0)).collect();
        let _ = AppendMenuW(menu, flags, 7, windows::core::PCWSTR(top_wide.as_ptr()));
        
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, flags, 3, windows::core::w!("Close Widget"));
        
        if widget_id == "picture" {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(menu, flags, 6, windows::core::w!("Remove Image"));
        } else if widget_id == "video" {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(menu, flags, 6, windows::core::w!("Remove Video"));
        }
        
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        
        if cmd.0 != 0 {
            let cmd_val = cmd.0;
            let widget_id_str = widget_id.to_string();
            let effective_widget_id_str = effective_widget_id.clone();
            let settings_ui_weak_c = settings_ui_weak.clone();
            let update_widget_lifecycles_c = update_widget_lifecycles.clone();
            let stats_widget_c = stats_widget.clone();
            
            slint::Timer::single_shot(std::time::Duration::from_millis(10), move || {
                let settings = settings::RavenSettings::load();
                match cmd_val {
                    6 => {
                        if widget_id_str == "video" {
                            settings::set_string(&["widgets", "video_path"], "");
                            if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                s_ui.set_video_selected_path("".into());
                            }
                        } else {
                            settings::set_string(&["widgets", "picture_path"], "");
                            if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                s_ui.set_picture_selected_path("".into());
                            }
                        }
                        update_widget_lifecycles_c();
                    }
                    1 => {
                        // Open Settings
                        if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                            let _ = s_ui.hide();
                            center_settings_window(&s_ui);
                            s_ui.set_active_tab("widgets".into());
                            s_ui.set_selected_widget_id(widget_id_str.into());
                            SETTINGS_WINDOW_OPEN.store(true, Ordering::SeqCst);
                            let _ = s_ui.show();
                        }
                    }
                    2 => {
                        // Toggle Lock Position
                        let new_val = !settings.widgets.locked;
                        settings::set_bool(&["widgets", "locked"], new_val);
                        
                        if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                            s_ui.set_widgets_locked(new_val);
                        }
                        
                        update_widget_lifecycles_c();
                    }
                    5 => {
                        // Toggle Click-Through
                        let new_val = !settings.widgets.click_through;
                        settings::set_bool(&["widgets", "click_through"], new_val);
                        
                        if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                            s_ui.set_widgets_click_through(new_val);
                        }
                        
                        update_widget_lifecycles_c();
                    }
                    3 => {
                        // Close Widget
                        if let Some(idx) = clock_idx {
                            println!("[WIDGET] show_native_context_menu Close Widget clock_idx: {}", idx);
                            // Step 1: Smoothly close and remove the specific window at idx
                            {
                                let mut stats_guard = stats_widget_c.borrow_mut();
                                if idx < stats_guard.len() {
                                    let w = stats_guard.remove(idx);
                                    let _ = w.hide();
                                }
                                if stats_guard.is_empty() {
                                    crate::window::STATS_WIDGET_HWND.store(0, std::sync::atomic::Ordering::SeqCst);
                                } else if idx == 0 {
                                    use raw_window_handle::HasWindowHandle;
                                    if let Ok(handle) = stats_guard[0].window().window_handle().window_handle() {
                                        if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                                            let hwnd = win32.hwnd.get() as isize;
                                            crate::window::STATS_WIDGET_HWND.store(hwnd, std::sync::atomic::Ordering::SeqCst);
                                        }
                                    }
                                }
                            }

                            // Step 2: Remove instance at idx and decrement count atomically in settings
                            let new_settings = settings::remove_clock_instance(idx);
                            let new_count = new_settings.widgets.clock_count as i32;

                            // Step 3: Sync UI state
                            if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                s_ui.set_widgets_clock_count(new_count);
                                if new_count == 0 {
                                    s_ui.set_widgets_stats_enabled(false);
                                    s_ui.set_selected_clock_index(0);
                                    sync_selected_clock_settings_to_ui(&s_ui, &new_settings, 0);
                                } else {
                                    sync_all_clocks_to_ui(&s_ui, &new_settings);
                                    let sel = s_ui.get_selected_clock_index().min(new_count - 1).max(0);
                                    s_ui.set_selected_clock_index(sel);
                                    sync_selected_clock_settings_to_ui(&s_ui, &new_settings, sel as usize);
                                }
                                reconcile_widget_order(&s_ui);
                            }
                        } else {
                            match widget_id_str.as_str() {
                                "year_progress" => {
                                    settings::set_bool(&["widgets", "year_journey_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_year_progress_enabled(false);
                                    }
                                }
                                "day_progress" => {
                                    settings::set_bool(&["widgets", "day_journey_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_day_progress_enabled(false);
                                    }
                                }
                                "month_progress" => {
                                    settings::set_bool(&["widgets", "month_journey_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_month_progress_enabled(false);
                                    }
                                }
                                "media" => {
                                    settings::set_bool(&["widgets", "media_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_media_enabled(false);
                                    }
                                }
                                "notes" => {
                                    settings::set_bool(&["widgets", "notes_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_notes_enabled(false);
                                    }
                                }
                                "todo" => {
                                    settings::set_bool(&["widgets", "todo_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_todo_enabled(false);
                                    }
                                }
                                "quotes" => {
                                    settings::set_bool(&["widgets", "quotes_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_quotes_enabled(false);
                                    }
                                }
                                "picture" => {
                                    settings::set_bool(&["widgets", "picture_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_picture_enabled(false);
                                    }
                                }
                                "video" => {
                                    settings::set_bool(&["widgets", "video_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_video_enabled(false);
                                    }
                                }
                                "battery" | "battery_widget" => {
                                    settings::set_bool(&["widgets", "battery_widget_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_battery_enabled(false);
                                    }
                                }
                                "calendar_focus" => {
                                    settings::set_bool(&["widgets", "calendar_focus_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_calendar_focus_enabled(false);
                                    }
                                }
                                "apps_container" => {
                                    settings::set_bool(&["widgets", "apps_container_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_apps_container_enabled(false);
                                    }
                                }
                                "focus_score" => {
                                    settings::set_bool(&["widgets", "focus_score_widget_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_focus_score_enabled(false);
                                    }
                                }
                                "streak" => {
                                    settings::set_bool(&["widgets", "streak_widget_enabled"], false);
                                    if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                        s_ui.set_widgets_streak_enabled(false);
                                    }
                                }
                                _ => {
                                    settings::update_widget_instance_visibility(&widget_id_str, false);
                                }
                            }
                            if let Some(s_ui) = settings_ui_weak_c.upgrade() {
                                reconcile_widget_order(&s_ui);
                            }
                        }
                        update_widget_lifecycles_c();
                    }
                    4 => {
                        // Reset Position
                        if let Some(idx) = clock_idx {
                            let default_x = 44 + (idx as i32 * 340);
                            let default_y = 86 + (idx as i32 * 20);
                            settings::update_clock_instance_setting(idx, |instance| {
                                instance.pos_x = default_x as f64;
                                instance.pos_y = default_y as f64;
                            });
                            update_widget_lifecycles_c();
                            return;
                        }
                        match widget_id_str.as_str() {
                            "clock" => {
                                settings::set_number(&["widgets", "clock_pos_x"], 44.0);
                                settings::set_number(&["widgets", "clock_pos_y"], 86.0);
                            }
                            "stats" | "battery" => {
                                settings::set_number(&["widgets", "stats_pos_x"], 44.0);
                                settings::set_number(&["widgets", "stats_pos_y"], 86.0);
                            }
                            "actions" => {
                                settings::set_number(&["widgets", "actions_pos_x"], 44.0);
                                settings::set_number(&["widgets", "actions_pos_y"], 86.0);
                            }
                            "year_progress" => {
                                settings::set_number(&["widgets", "year_journey_pos_x"], 40.0);
                                settings::set_number(&["widgets", "year_journey_pos_y"], 200.0);
                            }
                            "day_progress" => {
                                settings::set_number(&["widgets", "day_journey_pos_x"], 40.0);
                                settings::set_number(&["widgets", "day_journey_pos_y"], 360.0);
                            }
                            "month_progress" => {
                                settings::set_number(&["widgets", "month_journey_pos_x"], 40.0);
                                settings::set_number(&["widgets", "month_journey_pos_y"], 520.0);
                            }
                            "media" => {
                                settings::set_number(&["widgets", "media_pos_x"], 40.0);
                                settings::set_number(&["widgets", "media_pos_y"], 680.0);
                            }
                            "picture" => {
                                settings::set_number(&["widgets", "picture_pos_x"], 400.0);
                                settings::set_number(&["widgets", "picture_pos_y"], 520.0);
                            }
                            "video" => {
                                settings::set_number(&["widgets", "video_pos_x"], 400.0);
                                settings::set_number(&["widgets", "video_pos_y"], 680.0);
                            }
                            "battery_widget" => {
                                settings::set_number(&["widgets", "battery_widget_pos_x"], 720.0);
                                settings::set_number(&["widgets", "battery_widget_pos_y"], 200.0);
                            }
                            "calendar_focus" => {
                                settings::set_number(&["widgets", "calendar_focus_pos_x"], 720.0);
                                settings::set_number(&["widgets", "calendar_focus_pos_y"], 420.0);
                            }
                            "apps_container" => {
                                settings::set_number(&["widgets", "apps_container_pos_x"], 1040.0);
                                settings::set_number(&["widgets", "apps_container_pos_y"], 200.0);
                            }
                            "focus_score" => {
                                settings::set_number(&["widgets", "focus_score_widget_pos_x"], 760.0);
                                settings::set_number(&["widgets", "focus_score_widget_pos_y"], 420.0);
                            }
                            "streak" => {
                                settings::set_number(&["widgets", "streak_widget_pos_x"], 700.0);
                                settings::set_number(&["widgets", "streak_widget_pos_y"], 650.0);
                            }
                            _ => {}
                        }
                        update_widget_lifecycles_c();
                    }
                    7 => {
                        // Toggle Always on Top / Send to Back
                        let is_topmost = settings::is_widget_always_on_top(&settings::RavenSettings::load(), &effective_widget_id_str);
                        let next_topmost = !is_topmost;
                        settings::set_widget_always_on_top(&effective_widget_id_str, next_topmost);
                        unsafe {
                            crate::widgets::apply_widget_topmost_state(hwnd, next_topmost);
                        }
                        update_widget_lifecycles_c();
                    }
                    _ => {}
                }
            });
        }
    }
}

fn is_ny_dst(utc: &chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::Datelike;
    let year = utc.year();
    
    // Find second Sunday in March
    let mut march_sunday = 1;
    let mut sunday_count = 0;
    while sunday_count < 2 {
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, 3, march_sunday) {
            if date.weekday() == chrono::Weekday::Sun {
                sunday_count += 1;
                if sunday_count == 2 {
                    break;
                }
            }
        }
        march_sunday += 1;
    }
    
    // Find first Sunday in November
    let mut nov_sunday = 1;
    while let Some(date) = chrono::NaiveDate::from_ymd_opt(year, 11, nov_sunday) {
        if date.weekday() == chrono::Weekday::Sun {
            break;
        }
        nov_sunday += 1;
    }

    let start = chrono::NaiveDate::from_ymd_opt(year, 3, march_sunday).unwrap().and_hms_opt(7, 0, 0).unwrap();
    let end = chrono::NaiveDate::from_ymd_opt(year, 11, nov_sunday).unwrap().and_hms_opt(6, 0, 0).unwrap();

    let utc_naive = utc.naive_utc();
    utc_naive >= start && utc_naive < end
}

fn is_ldn_dst(utc: &chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::Datelike;
    let year = utc.year();
    
    // Find last Sunday in March
    let mut march_sunday = 31;
    while let Some(date) = chrono::NaiveDate::from_ymd_opt(year, 3, march_sunday) {
        if date.weekday() == chrono::Weekday::Sun {
            break;
        }
        march_sunday -= 1;
    }
    
    // Find last Sunday in October
    let mut oct_sunday = 31;
    while let Some(date) = chrono::NaiveDate::from_ymd_opt(year, 10, oct_sunday) {
        if date.weekday() == chrono::Weekday::Sun {
            break;
        }
        oct_sunday -= 1;
    }

    let start = chrono::NaiveDate::from_ymd_opt(year, 3, march_sunday).unwrap().and_hms_opt(1, 0, 0).unwrap();
    let end = chrono::NaiveDate::from_ymd_opt(year, 10, oct_sunday).unwrap().and_hms_opt(1, 0, 0).unwrap();

    let utc_naive = utc.naive_utc();
    utc_naive >= start && utc_naive < end
}

fn is_par_dst(utc: &chrono::DateTime<chrono::Utc>) -> bool {
    is_ldn_dst(utc)
}

fn is_syd_dst(utc: &chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::Datelike;
    let year = utc.year();
    
    // Sydney DST ends first Sunday in April, starts first Sunday in October.
    let mut apr_sunday = 1;
    while let Some(date) = chrono::NaiveDate::from_ymd_opt(year, 4, apr_sunday) {
        if date.weekday() == chrono::Weekday::Sun {
            break;
        }
        apr_sunday += 1;
    }
    
    let mut oct_sunday = 1;
    while let Some(date) = chrono::NaiveDate::from_ymd_opt(year, 10, oct_sunday) {
        if date.weekday() == chrono::Weekday::Sun {
            break;
        }
        oct_sunday += 1;
    }
    
    let apr_end = chrono::NaiveDate::from_ymd_opt(year, 4, apr_sunday).unwrap().and_hms_opt(16, 0, 0).unwrap();
    let oct_start = chrono::NaiveDate::from_ymd_opt(year, 10, oct_sunday).unwrap().and_hms_opt(16, 0, 0).unwrap();
    
    let utc_naive = utc.naive_utc();
    utc_naive < apr_end || utc_naive >= oct_start
}

const DEFAULT_QUOTES: &[(&str, &str)] = &[
    ("The only limit to our realization of tomorrow is our doubts of today.", "Franklin D. Roosevelt"),
    ("The greatest glory in living lies not in never falling, but in rising every time we fall.", "Nelson Mandela"),
    ("He who has a why to live for can bear almost any how.", "Friedrich Nietzsche"),
    ("Be the change that you wish to see in the world.", "Mahatma Gandhi"),
    ("Act as if what you do makes a difference. It does.", "William James"),
    ("Do not pray for an easy life, pray for the strength to endure a difficult one.", "Bruce Lee"),
    ("We make a living by what we get, but we make a life by what we give.", "Winston Churchill"),
    ("The only way to do great work is to love what you do.", "Steve Jobs"),
    ("It is during our darkest moments that we must focus to see the light.", "Aristotle Onassis"),
    ("Knowing yourself is the beginning of all wisdom.", "Aristotle"),
    ("Waste no more time arguing about what a good man should be. Be one.", "Marcus Aurelius"),
    ("The man who moves a mountain begins by carrying away small stones.", "Confucius"),
];

fn is_gif_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("gif"))
        .unwrap_or(false)
}

fn read_live_battery_status() -> Option<(f32, bool)> {
    unsafe {
        let mut status = windows::Win32::System::Power::SYSTEM_POWER_STATUS::default();
        windows::Win32::System::Power::GetSystemPowerStatus(&mut status).ok()?;
        let raw_pct = status.BatteryLifePercent;
        let pct = if raw_pct == 255 { 100.0 } else { raw_pct.min(100) as f32 };
        Some((pct, status.ACLineStatus == 1))
    }
}

fn render_battery_progress_ring(percent: f32) -> slint::Image {
    let size = 168u32;
    let center = size as f32 / 2.0;
    let radius = 68.0f32;
    let half_thickness = 7.0f32;
    let progress = percent.clamp(0.0, 100.0) / 100.0;
    let progress_color = if percent <= 20.0 {
        (255u8, 59u8, 48u8)
    } else if percent < 50.0 {
        (255u8, 204u8, 0u8)
    } else {
        (52u8, 199u8, 89u8)
    };

    let start_angle = -std::f32::consts::FRAC_PI_2;
    let end_angle = start_angle + progress * std::f32::consts::TAU;
    let start_point = (
        center + radius * start_angle.cos(),
        center + radius * start_angle.sin(),
    );
    let end_point = (
        center + radius * end_angle.cos(),
        center + radius * end_angle.sin(),
    );

    let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(size, size);
    for (index, pixel) in buffer.make_mut_slice().iter_mut().enumerate() {
        let x = (index as u32 % size) as f32 + 0.5;
        let y = (index as u32 / size) as f32 + 0.5;
        let dx = x - center;
        let dy = y - center;
        let distance = (dx * dx + dy * dy).sqrt();
        let on_track = (distance - radius).abs() <= half_thickness;

        let mut angle = dy.atan2(dx) - start_angle;
        if angle < 0.0 {
            angle += std::f32::consts::TAU;
        }
        let angle_progress = angle / std::f32::consts::TAU;
        let on_progress_arc = on_track && progress > 0.0 && angle_progress <= progress;
        let start_cap = progress > 0.0
            && ((x - start_point.0).powi(2) + (y - start_point.1).powi(2)).sqrt()
                <= half_thickness;
        let end_cap = progress > 0.0
            && ((x - end_point.0).powi(2) + (y - end_point.1).powi(2)).sqrt()
                <= half_thickness;

        *pixel = if on_progress_arc || start_cap || end_cap {
            slint::Rgba8Pixel::new(
                progress_color.0,
                progress_color.1,
                progress_color.2,
                255,
            )
        } else if on_track {
            slint::Rgba8Pixel::new(255, 255, 255, 31)
        } else {
            slint::Rgba8Pixel::new(0, 0, 0, 0)
        };
    }

    slint::Image::from_rgba8(buffer)
}

fn render_focus_progress_ring(percent: f32) -> slint::Image {
    let size = 184u32;
    let center = size as f32 / 2.0;
    let radius = 74.0f32;
    let half_thickness = 8.0f32;
    let progress = percent.clamp(0.0, 100.0) / 100.0;
    let start_angle = -std::f32::consts::FRAC_PI_2;
    let end_angle = start_angle + progress * std::f32::consts::TAU;
    let start_point = (
        center + radius * start_angle.cos(),
        center + radius * start_angle.sin(),
    );
    let end_point = (
        center + radius * end_angle.cos(),
        center + radius * end_angle.sin(),
    );

    let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(size, size);
    for (index, pixel) in buffer.make_mut_slice().iter_mut().enumerate() {
        let x = (index as u32 % size) as f32 + 0.5;
        let y = (index as u32 / size) as f32 + 0.5;
        let dx = x - center;
        let dy = y - center;
        let distance = (dx * dx + dy * dy).sqrt();
        let on_track = (distance - radius).abs() <= half_thickness;
        let mut angle = dy.atan2(dx) - start_angle;
        if angle < 0.0 {
            angle += std::f32::consts::TAU;
        }
        let on_progress_arc =
            on_track && progress > 0.0 && angle / std::f32::consts::TAU <= progress;
        let start_cap = progress > 0.0
            && ((x - start_point.0).powi(2) + (y - start_point.1).powi(2)).sqrt()
                <= half_thickness;
        let end_cap = progress > 0.0
            && ((x - end_point.0).powi(2) + (y - end_point.1).powi(2)).sqrt()
                <= half_thickness;

        *pixel = if on_progress_arc || start_cap || end_cap {
            slint::Rgba8Pixel::new(30, 144, 255, 255)
        } else if on_track {
            slint::Rgba8Pixel::new(24, 80, 130, 210)
        } else {
            slint::Rgba8Pixel::new(0, 0, 0, 0)
        };
    }
    slint::Image::from_rgba8(buffer)
}

fn update_calendar_focus_widget_properties(
    widget: &CalendarFocusWidgetWindow,
    runtime: &FocusTimerRuntime,
) {
    use chrono::Datelike;

    let now = chrono::Local::now();
    widget.set_weekday_str(now.format("%a").to_string().into());
    widget.set_month_str(now.format("%b").to_string().into());
    widget.set_day_str(now.day().to_string().into());

    let remaining = runtime.remaining_secs.get();
    let timer_text = if remaining >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            remaining / 3600,
            (remaining % 3600) / 60,
            remaining % 60
        )
    } else {
        format!("{:02}:{:02}", remaining / 60, remaining % 60)
    };
    let duration = runtime.duration_secs.get().max(1);
    let progress = remaining as f32 / duration as f32 * 100.0;
    widget.set_timer_str(timer_text.into());
    widget.set_timer_running(runtime.running.get());
    widget.set_progress_ring_img(render_focus_progress_ring(progress));
}

fn configure_video_widget_media(
    widget: &VideoFrameWidgetWindow,
    path: &str,
    alias: &str,
    gif_timer_cell: &std::rc::Rc<std::cell::RefCell<Option<slint::Timer>>>,
    video_stop_cell: &std::rc::Rc<std::cell::RefCell<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>>>,
) {
    if let Some(timer) = gif_timer_cell.borrow_mut().take() {
        timer.stop();
    }
    if let Some(stop_flag) = video_stop_cell.borrow_mut().take() {
        stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    stop_mci_video(alias);

    widget.set_video_path(path.into());
    widget.set_has_video(!path.is_empty());
    widget.set_is_gif(is_gif_path(path));

    if path.is_empty() {
        return;
    }

    if is_gif_path(path) {
        if let Some(frames) = load_gif_frame_images(path) {
            if frames.is_empty() {
                widget.set_has_video(false);
                return;
            }

            widget.set_video_frame_img(frames[0].clone());
            let frames = std::rc::Rc::new(frames);
            let frame_idx = std::rc::Rc::new(std::cell::Cell::new(0usize));
            let weak = widget.as_weak();
            let timer = slint::Timer::default();
            let frames_c = frames.clone();
            let frame_idx_c = frame_idx.clone();
            timer.start(
                slint::TimerMode::Repeated,
                std::time::Duration::from_millis(80),
                move || {
                    if let Some(w) = weak.upgrade() {
                        let next = (frame_idx_c.get() + 1) % frames_c.len();
                        frame_idx_c.set(next);
                        w.set_video_frame_img(frames_c[next].clone());
                    }
                },
            );
            *gif_timer_cell.borrow_mut() = Some(timer);
        } else {
            widget.set_has_video(false);
        }
    } else {
        let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        *video_stop_cell.borrow_mut() = Some(stop_flag.clone());
        start_ffmpeg_video_frames(widget.as_weak(), path.to_string(), stop_flag);
    }
}

fn find_ffmpeg_exe() -> Option<String> {
    use std::os::windows::process::CommandExt;

    std::process::Command::new("ffmpeg")
        .arg("-version")
        .creation_flags(0x08000000)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "ffmpeg".to_string())
}

fn start_ffmpeg_video_frames(
    widget_weak: slint::Weak<VideoFrameWidgetWindow>,
    path: String,
    stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let Some(ffmpeg) = find_ffmpeg_exe() else {
        println!("[WIDGET-DEBUG] ffmpeg not found; Video Frame cannot decode {}", path);
        return;
    };

    std::thread::spawn(move || {
        use std::io::Read;
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        let width = 320u32;
        let height = 150u32;
        let frame_len = (width * height * 4) as usize;
        let filter = format!(
            "fps=15,scale={}:{}:force_original_aspect_ratio=increase,crop={}:{}",
            width, height, width, height
        );

        let mut child = match Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-stream_loop",
                "-1",
                "-re",
                "-i",
                &path,
                "-an",
                "-vf",
                &filter,
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                println!("[WIDGET-DEBUG] failed to start ffmpeg for Video Frame: {}", err);
                return;
            }
        };

        let Some(mut stdout) = child.stdout.take() else {
            let _ = child.kill();
            return;
        };

        let mut frame = vec![0u8; frame_len];
        while !stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
            if stdout.read_exact(&mut frame).is_err() {
                break;
            }

            let bytes = frame.clone();
            let weak = widget_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(widget) = weak.upgrade() {
                    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                        &bytes,
                        width,
                        height,
                    );
                    widget.set_video_frame_img(slint::Image::from_rgba8(buffer));
                    widget.set_has_video(true);
                    widget.set_is_gif(true);
                }
            });
        }

        let _ = child.kill();
        let _ = child.wait();
    });
}

fn load_gif_frame_images(path: &str) -> Option<Vec<slint::Image>> {
    use image::AnimationDecoder;
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    let decoder = image::codecs::gif::GifDecoder::new(reader).ok()?;
    let frames = decoder.into_frames().collect_frames().ok()?;
    let mut images = Vec::with_capacity(frames.len());

    for frame in frames {
        let rgba = frame.into_buffer();
        let (width, height) = rgba.dimensions();
        let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            rgba.as_raw(),
            width,
            height,
        );
        images.push(slint::Image::from_rgba8(buffer));
    }

    Some(images)
}

fn mci_send(command: &str) -> u32 {
    let command_wide = wide(command);
    unsafe {
        mciSendStringW(
            command_wide.as_ptr(),
            std::ptr::null_mut(),
            0,
            windows::Win32::Foundation::HWND(0),
        ) as u32
    }
}

fn mci_quote_path(path: &str) -> String {
    format!("\"{}\"", path.replace('"', "\"\""))
}

fn stop_mci_video(alias: &str) {
    if !alias.is_empty() {
        let _ = mci_send(&format!("stop {}", alias));
        let _ = mci_send(&format!("close {}", alias));
    }
}

fn play_mci_video(hwnd: windows::Win32::Foundation::HWND, alias: &str, path: &str) {
    if alias.is_empty() || path.is_empty() || is_gif_path(path) {
        return;
    }

    stop_mci_video(alias);
    let path = mci_quote_path(path);
    let open_cmd = format!(
        "open {} type mpegvideo alias {} parent {} style child",
        path, alias, hwnd.0
    );
    let open_result = mci_send(&open_cmd);
    if open_result != 0 {
        println!("[WIDGET-DEBUG] MCI open failed for Video Frame with code {}", open_result);
        return;
    }
    let _ = mci_send(&format!("put {} window at 0 0 320 150", alias));
    let _ = mci_send(&format!("play {} repeat", alias));
}

fn select_image_file() -> Option<String> {
    use std::os::windows::process::CommandExt;
    let script = r#"
        Add-Type -AssemblyName System.Windows.Forms;
        $f = New-Object System.Windows.Forms.OpenFileDialog;
        $f.Filter = "Image Files|*.png;*.jpg;*.jpeg;*.gif;*.bmp;*.webp;*.ico";
        $f.Title = "Select Picture Frame Image";
        if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $f.FileName }
    "#;
    
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .ok()?;
        
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

fn select_video_file() -> Option<String> {
    use std::os::windows::process::CommandExt;
    let script = r#"
        Add-Type -AssemblyName System.Windows.Forms;
        $f = New-Object System.Windows.Forms.OpenFileDialog;
        $f.Filter = "GIF or Video Files|*.gif;*.mp4;*.webm;*.mov;*.avi;*.mkv;*.wmv;*.m4v|All Files|*.*";
        $f.Title = "Select Video Frame GIF or Video";
        if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $f.FileName }
    "#;
    
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .creation_flags(0x08000000)
        .output()
        .ok()?;
        
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

struct QueryCandidate {
    title: String,
    artist: String,
}

fn clean_query_part(value: &str) -> String {
    let re1 = regex::Regex::new(r"\[[^\]]*\]").unwrap();
    let re2 = regex::Regex::new(r"(?i)\([^)]*(official|video|audio|lyrics|lyric|visualizer|remaster|remastered|sped\s+up|slowed|nightcore|edit|full|hd|4k)[^)]*\)").unwrap();
    let re3 = regex::Regex::new(r"\([^)]*\)").unwrap();
    let re4 = regex::Regex::new(r"(?i)\b(official|music|video|audio|lyrics|lyric|visualizer|remaster|remastered|sped\s+up|slowed|nightcore|full\s+album|hd|4k)\b").unwrap();
    let re5 = regex::Regex::new(r"(?i)\b(ft|feat|featuring)\.?\s+.+$").unwrap();
    let re6 = regex::Regex::new(r"(?i)\s+[-|:]\s+(topic|official|lyrics?).*$").unwrap();
    let re_spaces = regex::Regex::new(r"\s+").unwrap();

    let s = re1.replace_all(value, " ");
    let s = re2.replace_all(&s, " ");
    let s = re3.replace_all(&s, " ");
    let s = re4.replace_all(&s, " ");
    let s = re5.replace_all(&s, " ");
    let s = re6.replace_all(&s, " ");
    let s = re_spaces.replace_all(&s, " ");
    
    s.trim().to_string()
}

fn get_query_candidates(title: &str, artist: &str) -> Vec<QueryCandidate> {
    let raw_title = title.trim();
    let raw_artist = artist.trim();
    
    let clean_title = clean_query_part(raw_title);
    let clean_artist = clean_query_part(raw_artist);
    
    let mut candidates = vec![
        QueryCandidate {
            title: if clean_title.is_empty() { raw_title.to_string() } else { clean_title },
            artist: if clean_artist.is_empty() { raw_artist.to_string() } else { clean_artist },
        },
        QueryCandidate {
            title: raw_title.to_string(),
            artist: raw_artist.to_string(),
        }
    ];

    let separators = [" - ", " – ", " — ", " | "];
    for &sep in &separators {
        if raw_title.contains(sep) {
            let parts: Vec<&str> = raw_title.split(sep).collect();
            if parts.len() >= 2 {
                let maybe_artist = parts[0].trim();
                let maybe_title = parts[1].trim();
                if !maybe_artist.is_empty() && !maybe_title.is_empty() {
                    candidates.insert(0, QueryCandidate {
                        title: clean_query_part(maybe_title),
                        artist: clean_query_part(maybe_artist),
                    });
                }
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for cand in candidates {
        let key = format!("{}::{}", cand.title, cand.artist).to_lowercase();
        if !cand.title.is_empty() && seen.insert(key) {
            deduped.push(cand);
        }
    }
    deduped
}

fn is_renderable_lyric(text: &str) -> bool {
    let value = text.trim();
    if value.is_empty() {
        return false;
    }
    
    let is_symbols = value.chars().all(|c| {
        c.is_whitespace() || 
        c == '.' || 
        c == '…' || 
        c == '♪' || 
        c == '♫' || 
        c == '♬' || 
        c == '·' || 
        c == '•' || 
        c == '-' ||
        c == '—' ||
        c == '_'
    });
    if is_symbols {
        return false;
    }
    
    let lower = value.to_lowercase();
    if lower == "instrumental" || lower == "music" || lower == "musical interlude" {
        return false;
    }
    
    true
}

fn parse_lrc_time(time_str: &str) -> Option<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 2 {
        let mins: f64 = parts[0].parse().ok()?;
        let secs: f64 = parts[1].parse().ok()?;
        Some(mins * 60.0 + secs)
    } else {
        None
    }
}

fn parse_synced_lyrics(synced: &str) -> Vec<(f64, String)> {
    let mut parsed = Vec::new();
    for line in synced.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            if let Some(end_bracket) = line.find(']') {
                let time_str = &line[1..end_bracket];
                let text = line[end_bracket + 1..].trim();
                if let Some(time_sec) = parse_lrc_time(time_str) {
                    if is_renderable_lyric(text) {
                        parsed.push((time_sec, text.to_string()));
                    }
                }
            }
        }
    }
    parsed
}

fn parse_plain_lyrics(plain: &str) -> Vec<(f64, String)> {
    let mut parsed = Vec::new();
    let mut index = 0;
    for line in plain.lines() {
        let text = line.trim();
        if is_renderable_lyric(text) {
            parsed.push((index as f64 * 5.0, text.to_string()));
            index += 1;
        }
    }
    parsed
}

struct IPv4OnlyResolver;

impl ureq::Resolver for IPv4OnlyResolver {
    fn resolve(&self, netloc: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
        use std::net::ToSocketAddrs;
        println!("[RESOLVER-DEBUG] Resolving netloc: '{}'", netloc);
        match netloc.to_socket_addrs() {
            Ok(addrs) => {
                let all: Vec<_> = addrs.collect();
                println!("[RESOLVER-DEBUG] Resolved all addresses: {:?}", all);
                let ipv4_addrs: Vec<_> = all.into_iter().filter(|addr| addr.is_ipv4()).collect();
                println!("[RESOLVER-DEBUG] Filtered IPv4 addresses: {:?}", ipv4_addrs);
                if ipv4_addrs.is_empty() {
                    println!("[RESOLVER-DEBUG] No IPv4 addresses found, returning AddrNotAvailable!");
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrNotAvailable,
                        "No IPv4 addresses found",
                    ));
                }
                Ok(ipv4_addrs)
            }
            Err(e) => {
                println!("[RESOLVER-DEBUG] DNS resolution error: {:?}", e);
                Err(e)
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct VersionManifest {
    version: String,
    channel: String,
    url: String,
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse_parts = |s: &str| -> Vec<u32> {
        s.trim_start_matches('v')
            .split('.')
            .map(|part| part.split('-').next().unwrap_or(part))
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect()
    };
    
    let current_parts = parse_parts(current);
    let latest_parts = parse_parts(latest);
    
    for i in 0..std::cmp::max(current_parts.len(), latest_parts.len()) {
        let cur = current_parts.get(i).cloned().unwrap_or(0);
        let lat = latest_parts.get(i).cloned().unwrap_or(0);
        if lat > cur {
            return true;
        } else if cur > lat {
            return false;
        }
    }
    false
}

fn fetch_latest_version() -> Result<VersionManifest, String> {
    let agent = ureq::AgentBuilder::new()
        .resolver(IPv4OnlyResolver)
        .timeout(std::time::Duration::from_secs(5))
        .build();
        
    let response = agent.get("https://ravennotch.me/version.json")
        .call()
        .map_err(|e| e.to_string())?;
        
    let manifest: VersionManifest = response.into_json()
        .map_err(|e| e.to_string())?;
        
    Ok(manifest)
}

#[derive(serde::Deserialize, Debug, Clone)]
struct LrcLibTrack {
    #[serde(rename = "trackName")]
    track_name: Option<String>,
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

struct LyricsEngine {
    cache: std::sync::Mutex<std::collections::HashMap<String, Vec<(f64, String)>>>,
    fetching: std::sync::Mutex<std::collections::HashSet<String>>,
}

static LYRICS_ENGINE: std::sync::OnceLock<std::sync::Arc<LyricsEngine>> = std::sync::OnceLock::new();

fn get_lyrics_engine() -> &'static std::sync::Arc<LyricsEngine> {
    LYRICS_ENGINE.get_or_init(|| {
        std::sync::Arc::new(LyricsEngine {
            cache: std::sync::Mutex::new(std::collections::HashMap::new()),
            fetching: std::sync::Mutex::new(std::collections::HashSet::new()),
        })
    })
}

fn async_fetch_lyrics(title: String, artist: String) {
    let key = format!("{}::{}", title, artist).to_lowercase();
    let engine = get_lyrics_engine().clone();
    
    std::thread::spawn(move || {
        println!("[LYRICS-DEBUG] Starting async fetch for '{}' by '{}'", title, artist);
        let clean_title = clean_query_part(&title);
        let clean_artist = clean_query_part(&artist);
        
        let mut urls = Vec::new();
        
        // 1. Precise get (cleaned)
        if !clean_title.is_empty() && !clean_artist.is_empty() {
            urls.push(format!(
                "https://lrclib.net/api/get?track_name={}&artist_name={}",
                urlencoding::encode(&clean_title),
                urlencoding::encode(&clean_artist)
            ));
        }
        
        // 2. Precise get (raw)
        if title != clean_title || artist != clean_artist {
            urls.push(format!(
                "https://lrclib.net/api/get?track_name={}&artist_name={}",
                urlencoding::encode(&title),
                urlencoding::encode(&artist)
            ));
        }

        // 3. Search (cleaned)
        if !clean_title.is_empty() && !clean_artist.is_empty() {
            let query = format!("{} {}", clean_title, clean_artist);
            urls.push(format!("https://lrclib.net/api/search?q={}", urlencoding::encode(&query)));
        }

        // 4. Search (raw)
        let query_raw = format!("{} {}", title, artist);
        urls.push(format!("https://lrclib.net/api/search?q={}", urlencoding::encode(&query_raw)));

        // 5. Check if title contains separators
        let separators = [" - ", " – ", " — ", " | "];
        for &sep in &separators {
            if title.contains(sep) {
                let parts: Vec<&str> = title.split(sep).collect();
                if parts.len() >= 2 {
                    let maybe_artist = parts[0].trim();
                    let maybe_title = parts[1].trim();
                    if !maybe_artist.is_empty() && !maybe_title.is_empty() {
                        let c_title = clean_query_part(maybe_title);
                        let c_artist = clean_query_part(maybe_artist);
                        urls.push(format!(
                            "https://lrclib.net/api/get?track_name={}&artist_name={}",
                            urlencoding::encode(&c_title),
                            urlencoding::encode(&c_artist)
                        ));
                        let query_sep = format!("{} {}", c_title, c_artist);
                        urls.push(format!("https://lrclib.net/api/search?q={}", urlencoding::encode(&query_sep)));
                    }
                }
            }
        }
        
        // Deduplicate urls
        let mut seen = std::collections::HashSet::new();
        urls.retain(|url| seen.insert(url.clone()));
        println!("[LYRICS-DEBUG] Generated {} candidate URLs to try.", urls.len());

        if urls.is_empty() {
            println!("[LYRICS-DEBUG] No candidate URLs generated. Inserting empty lyrics in cache.");
            let mut cache = engine.cache.lock().unwrap();
            cache.insert(key.clone(), vec![]);
            let mut fetching = engine.fetching.lock().unwrap();
            fetching.remove(&key);
            return;
        }

        let mut candidates = Vec::new();
        let title_lower = title.to_lowercase();
        let wants_translation = title_lower.contains("translation")
            || title_lower.contains("translated")
            || title_lower.contains("english version")
            || title_lower.contains("romanized");
        let is_translation_track = |track: &LrcLibTrack| -> bool {
            if let Some(name) = &track.track_name {
                let name_lower = name.to_lowercase();
                (name_lower.contains("translation")
                    || name_lower.contains("translated")
                    || name_lower.contains("english version")
                    || name_lower.contains("romanized"))
                    && !wants_translation
            } else {
                false
            }
        };

        let parse_json = |url: &str, json_str: &str| -> Option<(Vec<(f64, String)>, bool)> {
            if let Ok(list) = serde_json::from_str::<Vec<LrcLibTrack>>(json_str) {
                println!("[LYRICS-DEBUG] Successfully parsed JSON as Vec<LrcLibTrack>, count = {}", list.len());

                // Pass 1: prefer original lyrics before translated/romanized entries.
                for track in &list {
                    if !is_translation_track(track) {
                        if let Some(synced) = &track.synced_lyrics {
                            let parsed = parse_synced_lyrics(synced);
                            if !parsed.is_empty() {
                                println!("[LYRICS-DEBUG] Successfully parsed {} synced lyrics lines (original track priority)", parsed.len());
                                return Some((parsed, false));
                            }
                        }
                        if let Some(plain) = &track.plain_lyrics {
                            let parsed = parse_plain_lyrics(plain);
                            if !parsed.is_empty() {
                                println!("[LYRICS-DEBUG] Successfully parsed {} plain lyrics lines (original track priority)", parsed.len());
                                return Some((parsed, false));
                            }
                        }
                    }
                }

                for track in list {
                    if let Some(synced) = track.synced_lyrics {
                        let parsed = parse_synced_lyrics(&synced);
                        if !parsed.is_empty() {
                            println!("[LYRICS-DEBUG] Successfully parsed {} synced lyrics lines (fallback)", parsed.len());
                            return Some((parsed, true));
                        }
                    }
                    if let Some(plain) = track.plain_lyrics {
                        let parsed = parse_plain_lyrics(&plain);
                        if !parsed.is_empty() {
                            println!("[LYRICS-DEBUG] Successfully parsed {} plain lyrics lines (fallback)", parsed.len());
                            return Some((parsed, true));
                        }
                    }
                }
            } else {
                println!("[LYRICS-DEBUG] JSON for '{}' is not a Vec<LrcLibTrack>, trying single track", url);
            }

            if let Ok(track) = serde_json::from_str::<LrcLibTrack>(json_str) {
                println!("[LYRICS-DEBUG] Successfully parsed JSON as single LrcLibTrack");
                let is_translation = is_translation_track(&track);

                if let Some(synced) = track.synced_lyrics {
                    let parsed = parse_synced_lyrics(&synced);
                    if !parsed.is_empty() {
                        println!("[LYRICS-DEBUG] Successfully parsed {} synced lyrics lines (single track, translation={})", parsed.len(), is_translation);
                        return Some((parsed, is_translation));
                    }
                }
                if let Some(plain) = track.plain_lyrics {
                    let parsed = parse_plain_lyrics(&plain);
                    if !parsed.is_empty() {
                        println!("[LYRICS-DEBUG] Successfully parsed {} plain lyrics lines (single track, translation={})", parsed.len(), is_translation);
                        return Some((parsed, is_translation));
                    }
                }
            } else {
                println!("[LYRICS-DEBUG] Failed to parse JSON for '{}' as single LrcLibTrack", url);
            }
            None
        };

        for (index, url) in urls.iter().enumerate() {
            println!("[LYRICS-DEBUG] Fetching candidate URL natively {}/{}: '{}'", index + 1, urls.len(), url);
            let response = ureq::get(url)
                .set("User-Agent", "Raven-Notch/1.0")
                .timeout(std::time::Duration::from_secs(5))
                .call();

            match response {
                Ok(resp) => match resp.into_string() {
                    Ok(json_str) => {
                        println!("[LYRICS-DEBUG] Native fetch succeeded. JSON length: {}", json_str.len());
                        if let Some((lyrics, is_translation)) = parse_json(url, &json_str) {
                            println!("[LYRICS-DEBUG] Received candidate with {} lines (translation={})", lyrics.len(), is_translation);
                            candidates.push((lyrics, is_translation));
                        }
                    }
                    Err(err) => println!("[LYRICS-DEBUG] Native fetch body read failed for '{}': {}", url, err),
                },
                Err(err) => println!("[LYRICS-DEBUG] Native fetch failed for '{}': {}", url, err),
            }
        }
        
        let mut best_lyrics = None;
        let mut fallback_lyrics = None;
        for (lyrics, is_translation) in candidates {
            if !is_translation {
                if best_lyrics.is_none() {
                    best_lyrics = Some(lyrics);
                }
            } else {
                if fallback_lyrics.is_none() {
                    fallback_lyrics = Some(lyrics);
                }
            }
        }
        
        let final_lyrics = best_lyrics
            .or(fallback_lyrics)
            .unwrap_or_else(|| {
                println!("[LYRICS-DEBUG] All candidate threads finished, no lyrics found.");
                Vec::new()
            });
        
        let mut cache = engine.cache.lock().unwrap();
        cache.insert(key.clone(), final_lyrics);
        let mut fetching = engine.fetching.lock().unwrap();
        fetching.remove(&key);
        println!("[LYRICS-DEBUG] Finished caching results for '{}'", key);
    });
}

fn get_lyrics_for_track(title: &str, artist: &str, pos_sec: f64, _dur_sec: f64) -> (Vec<String>, i32, bool, String) {
    if title.is_empty() {
        return (
            vec![],
            0,
            false,
            "Ready to sync lyrics".to_string(),
        );
    }

    let key = format!("{}::{}", title, artist).to_lowercase();
    let engine = get_lyrics_engine();

    let lyrics_opt = {
        let cache = engine.cache.lock().unwrap();
        cache.get(&key).cloned()
    };

    if let Some(lyrics) = lyrics_opt {
        if lyrics.is_empty() {
            return (
                vec![],
                0,
                false,
                "No lyrics found.".to_string(),
            );
        }

        let pos = pos_sec + 0.5;

        let mut curr_idx = 0;
        for (i, &(time, _)) in lyrics.iter().enumerate() {
            if pos >= time {
                curr_idx = i;
            } else {
                break;
            }
        }

        let lines: Vec<String> = lyrics.iter().map(|&(_, ref text)| text.clone()).collect();
        (lines, curr_idx as i32, true, String::new())
    } else {
        let should_fetch = {
            let mut fetching = engine.fetching.lock().unwrap();
            fetching.insert(key.clone())
        };

        if should_fetch {
            async_fetch_lyrics(title.to_string(), artist.to_string());
        }

        (
            vec![],
            0,
            false,
            "Loading lyrics...".to_string(),
        )
    }
}

fn generate_svg_path(history: &std::collections::VecDeque<f32>) -> (String, String) {
    if history.is_empty() {
        return (String::new(), String::new());
    }

    let n = history.len();
    let mut path = String::new();
    
    // Line Path
    for (i, &val) in history.iter().enumerate() {
        let x = (i as f32 / (n - 1) as f32) * 100.0;
        let y = 95.0 - (val / 100.0) * 80.0;
        if i == 0 {
            path.push_str(&format!("M {} {}", x, y));
        } else {
            path.push_str(&format!(" L {} {}", x, y));
        }
    }

    // Fill Path (closes at the bottom)
    let mut fill = path.clone();
    fill.push_str(" L 100 100 L 0 100 Z");

    (path, fill)
}

fn update_calendar_ui(ui: &Pill, selected_day_idx: i32, master_events: &[services::CalendarEvent], force_days_rebuild: bool, center_date: chrono::NaiveDate) {
    let today_date = chrono::Local::now().date_naive();
    
    if force_days_rebuild {
        let mut dates = Vec::new();
        for offset in -180..=184 {
            let date = center_date.checked_add_signed(chrono::Duration::days(offset)).unwrap_or(center_date);
            dates.push(date);
        }
        
        let slint_days: Vec<SlintCalendarDay> = dates.iter().map(|&date| {
            let day_num = date.format("%d").to_string();
            let day_name = date.format("%a").to_string().to_uppercase();
            let month_name = date.format("%B").to_string();
            let year = date.format("%Y").to_string();
            let is_today = date == today_date;
            
            SlintCalendarDay {
                day_num: day_num.into(),
                day_name: day_name.into(),
                month_name: month_name.into(),
                year: year.into(),
                start_timestamp: 0,
                end_timestamp: 0,
                is_today,
            }
        }).collect();
        
        ui.set_calendar_days(std::rc::Rc::new(slint::VecModel::from(slint_days)).into());
    }
    
    // Calculate the target selected date based on offset from center date
    let selected_offset = (selected_day_idx - 180) as i64;
    let selected_date = center_date.checked_add_signed(chrono::Duration::days(selected_offset)).unwrap_or(center_date);
    
    // Update month and year text for the selected date
    ui.set_selected_month(selected_date.format("%B").to_string().into());
    ui.set_selected_year(selected_date.format("%Y").to_string().into());
    ui.set_selected_day_label(selected_date.format("%a %-d %b").to_string().into());
    
    // Automatically keep the mini/full calendar days updated
    use chrono::Datelike;
    let view_month = selected_date.month() as i32;
    let view_year = selected_date.year();
    ui.set_mini_calendar_view_month(view_month);
    ui.set_mini_calendar_view_year(view_year);
    let mini_days = generate_mini_calendar_days(view_year, view_month, selected_date);
    ui.set_mini_calendar_days(std::rc::Rc::new(slint::VecModel::from(mini_days)).into());
    
    // Filter events
    let filtered_events: Vec<SlintCalendarEvent> = master_events
        .iter()
        .filter(|event| {
            if let Some(event_dt) = chrono::DateTime::from_timestamp(event.timestamp, 0) {
                let local_dt = event_dt.with_timezone(&chrono::Local);
                local_dt.date_naive() == selected_date
            } else {
                false
            }
        })
        .map(|event| SlintCalendarEvent {
            title: event.title.clone().into(),
            date_str: event.date_str.clone().into(),
        })
        .collect();
        
    ui.set_filtered_calendar_events(std::rc::Rc::new(slint::VecModel::from(filtered_events)).into());
}

fn map_google_calendars(calendars: &[services::GoogleCalendarEntry], selected_ids: &[String]) -> Vec<SlintGoogleCalendarEntry> {
    let colors = [
        slint::Color::from_rgb_u8(0, 102, 255),   // Premium Blue
        slint::Color::from_rgb_u8(255, 69, 58),   // iOS Red
        slint::Color::from_rgb_u8(52, 199, 89),   // iOS Green
        slint::Color::from_rgb_u8(255, 159, 10),  // iOS Orange
        slint::Color::from_rgb_u8(175, 82, 222),  // iOS Purple
        slint::Color::from_rgb_u8(255, 214, 10),  // iOS Yellow
        slint::Color::from_rgb_u8(90, 200, 250),  // iOS Teal
        slint::Color::from_rgb_u8(255, 59, 128),  // iOS Pink
    ];
    calendars.iter().enumerate().map(|(idx, entry)| {
        let is_selected = if selected_ids.is_empty() {
            entry.primary.unwrap_or(false)
        } else {
            selected_ids.contains(&entry.id)
        };
        let color = colors[idx % colors.len()];
        SlintGoogleCalendarEntry {
            id: entry.id.clone().into(),
            summary: entry.summary.clone().into(),
            selected: is_selected,
            primary: entry.primary.unwrap_or(false),
            color,
        }
    }).collect()
}

fn generate_mini_calendar_days(year: i32, month: i32, selected_date: chrono::NaiveDate) -> Vec<SlintMiniCalendarDay> {
    use chrono::Datelike;
    let mut mini_days = Vec::with_capacity(42);
    
    if let Some(first_day) = chrono::NaiveDate::from_ymd_opt(year, month as u32, 1) {
        let start_weekday_idx = match first_day.weekday() {
            chrono::Weekday::Sun => 0,
            chrono::Weekday::Mon => 1,
            chrono::Weekday::Tue => 2,
            chrono::Weekday::Wed => 3,
            chrono::Weekday::Thu => 4,
            chrono::Weekday::Fri => 5,
            chrono::Weekday::Sat => 6,
        };
        
        let next_month_year = if month == 12 { year + 1 } else { year };
        let next_month = if month == 12 { 1 } else { month + 1 };
        let first_of_next = chrono::NaiveDate::from_ymd_opt(next_month_year, next_month as u32, 1).unwrap();
        let num_days = first_of_next.signed_duration_since(first_day).num_days() as usize;
        
        let today = chrono::Local::now().date_naive();
        
        for i in 0..42 {
            if i < start_weekday_idx || i >= start_weekday_idx + num_days {
                mini_days.push(SlintMiniCalendarDay {
                    day_num: "".into(),
                    is_current_month: false,
                    is_today: false,
                    is_selected: false,
                });
            } else {
                let day_of_month = (i - start_weekday_idx + 1) as u32;
                if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month as u32, day_of_month) {
                    mini_days.push(SlintMiniCalendarDay {
                        day_num: day_of_month.to_string().into(),
                        is_current_month: true,
                        is_today: date == today,
                        is_selected: date == selected_date,
                    });
                } else {
                    mini_days.push(SlintMiniCalendarDay {
                        day_num: "".into(),
                        is_current_month: false,
                        is_today: false,
                        is_selected: false,
                    });
                }
            }
        }
    } else {
        for _ in 0..42 {
            mini_days.push(SlintMiniCalendarDay {
                day_num: "".into(),
                is_current_month: false,
                is_today: false,
                is_selected: false,
            });
        }
    }
    
    mini_days
}

fn extract_accent_color(path: &str) -> Option<slint::Color> {
    let img = image::open(path).ok()?;
    let thumb = image::imageops::thumbnail(&img, 32, 32);
    
    let mut buckets = std::collections::HashMap::new();
    
    for pixel in thumb.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < 128 { continue; }
        
        // Quantize RGB values into 8 bins per channel
        let r_bin = r / 32;
        let g_bin = g / 32;
        let b_bin = b / 32;
        let key = (r_bin, g_bin, b_bin);
        
        let entry = buckets.entry(key).or_insert((0u64, 0u64, 0u64, 0u32));
        entry.0 += r as u64;
        entry.1 += g as u64;
        entry.2 += b as u64;
        entry.3 += 1;
    }
    
    let mut best_color = None;
    let mut best_score = -1.0f32;
    
    let mut fallback_color = None;
    let mut max_fallback_count = 0;
    
    for (_key, &(sum_r, sum_g, sum_b, count)) in &buckets {
        let avg_r = (sum_r / count as u64) as u8;
        let avg_g = (sum_g / count as u64) as u8;
        let avg_b = (sum_b / count as u64) as u8;
        
        let rf = avg_r as f32 / 255.0;
        let gf = avg_g as f32 / 255.0;
        let bf = avg_b as f32 / 255.0;
        
        let max = rf.max(gf).max(bf);
        let min = rf.min(gf).min(bf);
        
        let l = (max + min) / 2.0;
        let s = if l > 0.0 && l < 1.0 {
            (max - min) / (1.0 - (2.0 * l - 1.0).abs())
        } else {
            0.0
        };
        
        // Score is primarily based on area (pixel count)
        let mut score = count as f32;
        if s < 0.05 || l < 0.08 || l > 0.95 {
            score *= 0.01;
        }
        
        if score > best_score {
            best_score = score;
            best_color = Some((avg_r, avg_g, avg_b));
        }
        
        if count > max_fallback_count {
            max_fallback_count = count;
            fallback_color = Some((avg_r, avg_g, avg_b));
        }
    }
    
    if let Some((mut r, mut g, mut b)) = best_color.or(fallback_color) {
        let rf = r as f32 / 255.0;
        let gf = g as f32 / 255.0;
        let bf = b as f32 / 255.0;
        
        let max = rf.max(gf).max(bf);
        let min = rf.min(gf).min(bf);
        let delta = max - min;
        
        let mut h = if delta == 0.0 {
            0.0
        } else if max == rf {
            60.0 * (((gf - bf) / delta) % 6.0)
        } else if max == gf {
            60.0 * (((bf - rf) / delta) + 2.0)
        } else {
            60.0 * (((rf - gf) / delta) + 4.0)
        };
        if h < 0.0 { h += 360.0; }
        
        let mut s = if max == 0.0 { 0.0 } else { delta / max };
        let mut v = max;
        
        // BOOST SATURATION AND VALUE FOR GLOW
        s = (s * 1.5).clamp(0.5, 1.0); // Ensure it's colorful enough
        v = (v * 1.5).clamp(0.7, 1.0); // Ensure it's bright enough
        
        // Convert back to RGB
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        
        let (r_prime, g_prime, b_prime) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        
        r = ((r_prime + m) * 255.0).round() as u8;
        g = ((g_prime + m) * 255.0).round() as u8;
        b = ((b_prime + m) * 255.0).round() as u8;
        
        Some(slint::Color::from_rgb_u8(r, g, b))
    } else {
        Some(slint::Color::from_rgb_u8(128, 128, 128))
    }
}

#[derive(Debug)]
struct FullscreenDebounceState {
    initialized: bool,
    observed: bool,
    stable: bool,
    observed_since: std::time::Instant,
}

fn debounce_fullscreen_state(observed: bool) -> bool {
    static STATE: std::sync::OnceLock<std::sync::Mutex<FullscreenDebounceState>> =
        std::sync::OnceLock::new();
    let state = STATE.get_or_init(|| {
        std::sync::Mutex::new(FullscreenDebounceState {
            initialized: false,
            observed: false,
            stable: false,
            observed_since: std::time::Instant::now(),
        })
    });
    let Ok(mut state) = state.lock() else {
        return observed;
    };

    let now = std::time::Instant::now();
    if !state.initialized {
        state.initialized = true;
        state.observed = observed;
        state.stable = observed;
        state.observed_since = now;
    } else if observed != state.observed {
        state.observed = observed;
        state.observed_since = now;
    } else if state.stable != observed
        && now.saturating_duration_since(state.observed_since)
            >= std::time::Duration::from_millis(120)
    {
        state.stable = observed;
    }

    state.stable
}

fn is_foreground_window_fullscreen() -> bool {
    use std::sync::atomic::Ordering;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
    };
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetClassNameW, GetForegroundWindow, GetWindowRect,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed, GA_ROOT,
    };

    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.0 == 0 {
            return crate::window::IS_FOREGROUND_FULLSCREEN.load(Ordering::SeqCst);
        }

        let root = GetAncestor(foreground, GA_ROOT);
        let hwnd = if root.0 != 0 { root } else { foreground };

        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return debounce_fullscreen_state(false);
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 && pid == std::process::id() {
            let settings_hwnd = crate::window::SETTINGS_HWND.load(Ordering::SeqCst);
            if hwnd.0 != settings_hwnd || settings_hwnd == 0 {
                return crate::window::IS_FOREGROUND_FULLSCREEN.load(Ordering::SeqCst);
            }
        }

        let mut class_buf = [0u16; 256];
        let class_len = GetClassNameW(hwnd, &mut class_buf) as usize;
        let class_name = String::from_utf16_lossy(&class_buf[..class_len]);
        let shell_classes = [
            "Progman",
            "WorkerW",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
            "DV2ControlHost",
            "MsgrIMEWindowClass",
            "SysShadow",
        ];
        if shell_classes.iter().any(|class| class_name == *class) {
            return debounce_fullscreen_state(false);
        }

        let mut cloaked = 0u32;
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
        {
            return debounce_fullscreen_state(false);
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return debounce_fullscreen_state(false);
        }

        let mut rect = RECT::default();
        let has_dwm_bounds = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_ok();
        if !has_dwm_bounds && GetWindowRect(hwnd, &mut rect).is_err() {
            return debounce_fullscreen_state(false);
        }

        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96);
        let tolerance = monitor_math::fullscreen_tolerance(dpi);
        let window_bounds = monitor_math::MonitorBounds {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        };
        let monitor_bounds = monitor_math::MonitorBounds {
            left: info.rcMonitor.left,
            top: info.rcMonitor.top,
            right: info.rcMonitor.right,
            bottom: info.rcMonitor.bottom,
        };
        let work_area_bounds = monitor_math::MonitorBounds {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        };
        let is_maximized = IsZoomed(hwnd).as_bool();

        let raw_fullscreen = monitor_math::rect_is_fullscreen_like(
            window_bounds,
            monitor_bounds,
            work_area_bounds,
            tolerance,
            is_maximized,
        );
        let stable_fullscreen = debounce_fullscreen_state(raw_fullscreen);
        let state_code = (raw_fullscreen as u8) | ((stable_fullscreen as u8) << 1);
        static LAST_LOGGED_STATE: std::sync::atomic::AtomicU8 =
            std::sync::atomic::AtomicU8::new(u8::MAX);
        if LAST_LOGGED_STATE.swap(state_code, Ordering::SeqCst) != state_code {
            crate::diagnostics::log(
                "FULLSCREEN-PROBE",
                &format!(
                    "hwnd=0x{:X} class={} dpi={} dwm_bounds={} zoomed={} window=({}, {}, {}, {}) monitor=({}, {}, {}, {}) work=({}, {}, {}, {}) tolerance={} raw={} stable={}",
                    hwnd.0,
                    class_name,
                    dpi,
                    has_dwm_bounds,
                    is_maximized,
                    rect.left,
                    rect.top,
                    rect.right,
                    rect.bottom,
                    info.rcMonitor.left,
                    info.rcMonitor.top,
                    info.rcMonitor.right,
                    info.rcMonitor.bottom,
                    info.rcWork.left,
                    info.rcWork.top,
                    info.rcWork.right,
                    info.rcWork.bottom,
                    tolerance,
                    raw_fullscreen,
                    stable_fullscreen
                ),
            );
        }
        stable_fullscreen
    }
}





