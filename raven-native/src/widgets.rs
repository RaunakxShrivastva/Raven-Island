#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeTab {
    Home,
    Media,
    Clock,
    Drop,
    Capture,
    Calendar,
    Notifications,
    Stats,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeAction {
    MediaPrevious,
    MediaPlayPause,
    MediaNext,
    TimerToggle,
    TimerReset,
    StopwatchToggle,
    StopwatchReset,
    ShelfOpenFirst,
    ShelfRevealFirst,
    ShelfClear,
    CaptureScreenshot,
    CaptureRegion,
    CaptureOpenLast,
    CaptureOpenFolder,
    CalendarRefresh,
    CaffeineToggle,
    VolumeDown,
    VolumeMute,
    VolumeUp,
    BrightnessDown,
    BrightnessUp,
    SettingsOpenFile,
    SettingsWidthDown,
    SettingsWidthUp,
    SettingsOpacityDown,
    SettingsOpacityUp,
    SettingsHoverToggle,
    OpenNotificationSettings,
}

pub static APPS_CONTAINER_DROP_CALLBACK: std::sync::OnceLock<Box<dyn Fn(Vec<String>) + Send + Sync>> =
    std::sync::OnceLock::new();

unsafe fn widget_dropped_paths(drop: windows::Win32::UI::Shell::HDROP) -> Vec<String> {
    let count = windows::Win32::UI::Shell::DragQueryFileW(drop, 0xFFFF_FFFF, None) as u32;
    let mut paths = Vec::new();
    for index in 0..count {
        let len = windows::Win32::UI::Shell::DragQueryFileW(drop, index, None) as usize + 1;
        let mut buffer = vec![0u16; len];
        let copied = windows::Win32::UI::Shell::DragQueryFileW(drop, index, Some(&mut buffer));
        if copied > 0 {
            let value = String::from_utf16_lossy(&buffer[..copied as usize]);
            if !value.trim().is_empty() {
                paths.push(value);
            }
        }
    }
    windows::Win32::UI::Shell::DragFinish(drop);
    paths
}

unsafe fn widget_window_title(hwnd: windows::Win32::Foundation::HWND) -> String {
    let mut title_buf = [0u16; 512];
    let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut title_buf);
    String::from_utf16_lossy(&title_buf[..len as usize])
}

fn widget_id_from_window_title(
    title: &str,
    settings: &crate::settings::RavenSettings,
) -> Option<String> {
    if title.starts_with("Raven Clock Widget") {
        let idx = title["Raven Clock Widget".len()..]
            .trim()
            .parse::<usize>()
            .unwrap_or(0);
        return Some(format!("clock_{}", idx));
    }

    if let Some(id) = title.strip_prefix("Raven Generic Widget - ") {
        let id = id.trim();
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    let builtins = [
        ("Raven Year Progress Widget ", "year_progress"),
        ("Raven Day Progress Widget ", "day_progress"),
        ("Raven Month Progress Widget ", "month_progress"),
        ("Raven Media Widget ", "media"),
        ("Raven Notes Widget ", "notes"),
        ("Raven Todo Widget ", "todo"),
        ("Raven Quotes Widget ", "quotes"),
        ("Raven Picture Widget ", "picture"),
        ("Raven Video Frame Widget ", "video"),
        ("Raven Battery Percentage Widget ", "battery_widget"),
        ("Raven Calendar Focus Widget ", "calendar_focus"),
        ("Raven System Stats Widget ", "system_stats"),
        ("Raven Apps Container Widget ", "apps_container"),
        ("Raven Focus Score Widget ", "focus_score"),
        ("Raven Calendar Widget ", "streak"),
    ];

    for (prefix, builtin_id) in builtins {
        if let Some(suffix) = title.strip_prefix(prefix) {
            let suffix = suffix.trim();
            if !suffix.is_empty()
                && settings
                    .widgets
                    .instances
                    .iter()
                    .any(|instance| instance.id == suffix)
            {
                return Some(suffix.to_string());
            }
            return Some(builtin_id.to_string());
        }
    }

    None
}

unsafe fn widget_saved_topmost_state(hwnd: windows::Win32::Foundation::HWND) -> bool {
    let settings = crate::settings::RavenSettings::load();
    let title = widget_window_title(hwnd);
    widget_id_from_window_title(&title, &settings)
        .as_deref()
        .is_some_and(|id| crate::settings::is_widget_always_on_top(&settings, id))
}

impl NativeTab {
    pub const ALL: [NativeTab; 9] = [
        NativeTab::Home,
        NativeTab::Media,
        NativeTab::Clock,
        NativeTab::Drop,
        NativeTab::Capture,
        NativeTab::Calendar,
        NativeTab::Notifications,
        NativeTab::Stats,
        NativeTab::Settings,
    ];

    pub fn label(self) -> &'static str {
        self.descriptor().label
    }

    pub fn descriptor(self) -> PanelDescriptor {
        match self {
            NativeTab::Home => PanelDescriptor {
                label: "Home",
                title: "Home dashboard native port",
                detail: "Profile, agenda, quick actions, top-bar state",
            },
            NativeTab::Media => PanelDescriptor {
                label: "Media",
                title: "Media controls native port",
                detail: "Album art, title, artist, progress, playback buttons",
            },
            NativeTab::Clock => PanelDescriptor {
                label: "Clock",
                title: "Clock / timer native port",
                detail: "Clock, stopwatch, timer, laps",
            },
            NativeTab::Drop => PanelDescriptor {
                label: "Drop",
                title: "Drop shelf native port",
                detail: "Keep/share zones, previews, shelf items",
            },
            NativeTab::Capture => PanelDescriptor {
                label: "Capture",
                title: "Capture studio native port",
                detail: "Modes, recent captures, record status",
            },
            NativeTab::Calendar => PanelDescriptor {
                label: "Cal",
                title: "Calendar native port",
                detail: "ICS agenda, Google status, upcoming events",
            },
            NativeTab::Notifications => PanelDescriptor {
                label: "Alerts",
                title: "Notification hub native port",
                detail: "Windows notification access, recent alerts, app events",
            },
            NativeTab::Stats => PanelDescriptor {
                label: "Stats",
                title: "System stats native port",
                detail: "CPU, RAM, battery, volume, brightness",
            },
            NativeTab::Settings => PanelDescriptor {
                label: "Set",
                title: "Settings native port",
                detail: "Existing schema, user data paths, migration controls",
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PanelDescriptor {
    pub label: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
}

#[derive(Clone, Debug)]
pub struct WidgetModel {
    pub current_tab: NativeTab,
}

impl WidgetModel {
    pub fn new() -> Self {
        Self {
            current_tab: NativeTab::Home,
        }
    }

    pub fn select_tab_at(&mut self, x: i32, width: f32) {
        let tab_width = (width / NativeTab::ALL.len() as f32).max(1.0);
        let index = (x.max(0) as f32 / tab_width).floor() as usize;
        if let Some(tab) = NativeTab::ALL.get(index).copied() {
            self.current_tab = tab;
        }
    }
}

// ── DESKTOP WIDGET ENGINE RUST BACKEND ──────────────────────────────────
// Provides Win32 window pinning, styles configuration, and system helpers.

use windows::Win32::Foundation::{BOOL, HWND, LPARAM};

pub static WIDGET_DRAG_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Tracks HWNDs that have been explicitly positioned via position_widget_window_from_left().
/// Used by save_current_extra_widget_positions() to avoid overwriting saved positions
/// with the default startup position before the 50ms positioning timer fires.
pub static POSITIONED_HWNDS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<isize>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

static ZORDER_UNLOCK_HWNDS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<isize>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

fn widget_zorder_unlocked(hwnd: HWND) -> bool {
    ZORDER_UNLOCK_HWNDS
        .lock()
        .map(|set| set.contains(&hwnd.0))
        .unwrap_or(false)
}

fn with_widget_zorder_unlocked(hwnd: HWND, apply: impl FnOnce()) {
    {
        let mut set = ZORDER_UNLOCK_HWNDS.lock().unwrap();
        set.insert(hwnd.0);
    }
    apply();
    {
        let mut set = ZORDER_UNLOCK_HWNDS.lock().unwrap();
        set.remove(&hwnd.0);
    }
}

unsafe extern "system" fn find_desktop_worker_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetWindow};
    use windows::Win32::UI::WindowsAndMessaging::GW_HWNDNEXT;

    let shell_view = FindWindowExW(
        hwnd,
        HWND(0),
        windows::core::w!("SHELLDLL_DefView"),
        None,
    );
    if shell_view.0 != 0 {
        let worker = GetWindow(hwnd, GW_HWNDNEXT);
        if worker.0 != 0 {
            *(lparam.0 as *mut HWND) = worker;
            return BOOL(0);
        }
    }

    BOOL(1)
}

pub unsafe fn desktop_host_window() -> Option<HWND> {
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowW, SendMessageTimeoutW, SMTO_NORMAL,
    };

    let progman = FindWindowW(windows::core::w!("Progman"), None);
    if progman.0 != 0 {
        let mut _result = 0usize;
        let _ = SendMessageTimeoutW(
            progman,
            0x052C,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
            SMTO_NORMAL,
            1000,
            Some(&mut _result),
        );
    }

    let mut worker = HWND(0);
    let _ = EnumWindows(
        Some(find_desktop_worker_proc),
        LPARAM(&mut worker as *mut HWND as isize),
    );

    if worker.0 != 0 {
        Some(worker)
    } else if progman.0 != 0 {
        Some(progman)
    } else {
        None
    }
}


fn get_setting_number_by_key(settings: &crate::settings::RavenSettings, key: &str) -> f64 {
    match key {
        "year_journey_pos_x" => settings.widgets.year_journey_pos_x,
        "year_journey_pos_y" => settings.widgets.year_journey_pos_y,
        "day_journey_pos_x" => settings.widgets.day_journey_pos_x,
        "day_journey_pos_y" => settings.widgets.day_journey_pos_y,
        "month_journey_pos_x" => settings.widgets.month_journey_pos_x,
        "month_journey_pos_y" => settings.widgets.month_journey_pos_y,
        "media_pos_x" => settings.widgets.media_pos_x,
        "media_pos_y" => settings.widgets.media_pos_y,
        "notes_pos_x" => settings.widgets.notes_pos_x,
        "notes_pos_y" => settings.widgets.notes_pos_y,
        "todo_pos_x" => settings.widgets.todo_pos_x,
        "todo_pos_y" => settings.widgets.todo_pos_y,
        "quotes_pos_x" => settings.widgets.quotes_pos_x,
        "quotes_pos_y" => settings.widgets.quotes_pos_y,
        "picture_pos_x" => settings.widgets.picture_pos_x,
        "picture_pos_y" => settings.widgets.picture_pos_y,
        "video_pos_x" => settings.widgets.video_pos_x,
        "video_pos_y" => settings.widgets.video_pos_y,
        "battery_widget_pos_x" => settings.widgets.battery_widget_pos_x,
        "battery_widget_pos_y" => settings.widgets.battery_widget_pos_y,
        "calendar_focus_pos_x" => settings.widgets.calendar_focus_pos_x,
        "calendar_focus_pos_y" => settings.widgets.calendar_focus_pos_y,
        "system_stats_widget_pos_x" => settings.widgets.system_stats_widget_pos_x,
        "system_stats_widget_pos_y" => settings.widgets.system_stats_widget_pos_y,
        "apps_container_pos_x" => settings.widgets.apps_container_pos_x,
        "apps_container_pos_y" => settings.widgets.apps_container_pos_y,
        "focus_score_widget_pos_x" => settings.widgets.focus_score_widget_pos_x,
        "focus_score_widget_pos_y" => settings.widgets.focus_score_widget_pos_y,
        "streak_widget_pos_x" => settings.widgets.streak_widget_pos_x,
        "streak_widget_pos_y" => settings.widgets.streak_widget_pos_y,
        _ => 0.0,
    }
}

pub unsafe extern "system" fn widget_window_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _uidsubclass: usize,
    _dwrefdata: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::Foundation::RECT;

    if msg == 0x00A1 /* WM_NCLBUTTONDOWN */ && wparam.0 == 2 /* HTCAPTION */ {
        if WIDGET_DRAG_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
            return windows::Win32::Foundation::LRESULT(0);
        }
    }

    if msg == WM_DROPFILES {
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title_str = String::from_utf16_lossy(&title_buf[..len as usize]);
        if title_str.starts_with("Raven Apps Container Widget ") {
            let paths = widget_dropped_paths(windows::Win32::UI::Shell::HDROP(wparam.0 as isize));
            if let Some(cb) = APPS_CONTAINER_DROP_CALLBACK.get() {
                cb(paths);
            }
            return windows::Win32::Foundation::LRESULT(0);
        }
    }

    // Detailed logging for requested messages
    let mut log_msg = false;
    let msg_name = match msg {
        0x0010 => { log_msg = true; "WM_CLOSE" },
        0x0002 => { log_msg = true; "WM_DESTROY" },
        0x0082 => { log_msg = true; "WM_NCDESTROY" },
        0x0018 => { log_msg = true; "WM_SHOWWINDOW" },
        0x0046 => { log_msg = true; "WM_WINDOWPOSCHANGING" },
        0x0047 => { log_msg = true; "WM_WINDOWPOSCHANGED" },
        0x007D => { log_msg = true; "WM_STYLECHANGED" },
        0x0210 => { log_msg = true; "WM_PARENTNOTIFY" },
        _ => "",
    };

    if log_msg {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let parent = GetParent(hwnd);
        let owner = GetWindow(hwnd, GW_OWNER);
        let is_visible = IsWindowVisible(hwnd);
        
        let insert_after_str = if msg == 0x0046 || msg == 0x0047 {
            let winpos = &*(lparam.0 as *const WINDOWPOS);
            format!("InsertAfter: {:?}, Flags: 0x{:08X}", winpos.hwndInsertAfter, winpos.flags.0)
        } else {
            "N/A".to_string()
        };

        println!("[WIDGET-LOG] Subclass Msg: {} (0x{:04X}) | HWND: {:?} | Style: 0x{:08X} | ExStyle: 0x{:08X} | Parent: {:?} | Owner: {:?} | IsVisible: {:?} | {} | wparam: {} | lparam: {}",
                 msg_name, msg, hwnd, style, ex_style, parent, owner, is_visible, insert_after_str, wparam.0, lparam.0);
    }

    // ── NON-CLIENT AREA SUPPRESSION ──────────────────────────────────────────
    // WM_NCCALCSIZE (0x0083): When wparam=1, return 0 → client area = window area
    // This permanently kills the title bar geometry regardless of style resets.
    if msg == 0x0083 && wparam.0 != 0 {
        return windows::Win32::Foundation::LRESULT(0);
    }

    // WM_NCPAINT (0x0085): Suppress all non-client (title bar / border) painting
    if msg == 0x0085 {
        return windows::Win32::Foundation::LRESULT(0);
    }

    // WM_NCACTIVATE (0x0086): Return TRUE without calling DefWindowProc to prevent
    // the activation highlight frame from appearing on the title bar area
    if msg == 0x0086 {
        return windows::Win32::Foundation::LRESULT(1);
    }

    // WM_MOUSEACTIVATE (0x0021): Return MA_NOACTIVATE (3) to prevent the window
    // from gaining activation while ensuring mouse click messages are processed.
    if msg == 0x0021 {
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title_str = String::from_utf16_lossy(&title_buf[..len as usize]);
        let is_focusable = title_str.starts_with("Raven Notes Widget ") || title_str.starts_with("Raven Todo Widget ");
        if !is_focusable {
            return windows::Win32::Foundation::LRESULT(3);
        }
    }

    // WM_SYSCOMMAND (0x0112): Block SC_MINIMIZE, SC_MAXIMIZE, SC_RESTORE so that
    // keyboard shortcuts like Ctrl+D or Win+D cannot minimize/maximize the widget window.
    // SC_MINIMIZE = 0xF020, SC_MAXIMIZE = 0xF030, SC_RESTORE = 0xF120
    if msg == 0x0112 {
        let cmd = wparam.0 & 0xFFF0;
        if cmd == 0xF020 || cmd == 0xF030 || cmd == 0xF120 {
            return windows::Win32::Foundation::LRESULT(0);
        }
    }

    // WM_STYLECHANGED (0x007D): winit occasionally resets styles during redraws.
    // Re-enforce our popup + no-caption style immediately after any style change.
    if msg == 0x007D && !WIDGET_DRAG_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let target_style = (style & !WS_CHILD.0 & !WS_CAPTION.0 & !WS_THICKFRAME.0 & !WS_SYSMENU.0 & !WS_MINIMIZEBOX.0 & !WS_MAXIMIZEBOX.0) | WS_POPUP.0;
        if style != target_style {
            println!("[WIDGET-LOG] STYLE CHANGING: Style from 0x{:08X} to target 0x{:08X}", style, target_style);
            let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, target_style as isize);
        }
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title_str = String::from_utf16_lossy(&title_buf[..len as usize]);
        let is_focusable = title_str.starts_with("Raven Notes Widget ") || title_str.starts_with("Raven Todo Widget ");

        let mut target_ex = ex_style | WS_EX_TOOLWINDOW.0 | WS_EX_LAYERED.0;
        if is_focusable {
            target_ex &= !WS_EX_NOACTIVATE.0;
        } else {
            target_ex |= WS_EX_NOACTIVATE.0;
        }
        target_ex &= !WS_EX_APPWINDOW.0;

        if ex_style != target_ex {
            println!("[WIDGET-LOG] EXSTYLE CHANGING: ExStyle from 0x{:08X} to target 0x{:08X}", ex_style, target_ex);
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex as isize);
            crate::window::remove_from_taskbar(hwnd);
        }
    }

    // Strict Desktop Z-Order Bottom-Lock & Window Styles Enforcement
    if msg == WM_WINDOWPOSCHANGING {
        let winpos = &mut *(lparam.0 as *mut WINDOWPOS);
        let is_topmost_toggle = winpos.hwndInsertAfter == HWND_TOPMOST
            || winpos.hwndInsertAfter == HWND_NOTOPMOST;
        if !WIDGET_DRAG_ACTIVE.load(std::sync::atomic::Ordering::SeqCst)
            && !widget_zorder_unlocked(hwnd)
            && !is_topmost_toggle
        {
            winpos.flags = SET_WINDOW_POS_FLAGS(winpos.flags.0 | SWP_NOZORDER.0);
        }

        // Prevent Win+D / Show Desktop from minimizing the widget.
        // Windows moves minimized windows to the special position (-32000, -32000).
        // By adding SWP_NOMOVE | SWP_NOSIZE we keep the window in place.
        if (winpos.flags.0 & SWP_NOMOVE.0 == 0) && (winpos.x == -32000 || winpos.y == -32000) {
            println!("[WIDGET-LOG] WINDOWPOSCHANGING: blocking minimize attempt (x={}, y={})", winpos.x, winpos.y);
            winpos.flags = SET_WINDOW_POS_FLAGS(winpos.flags.0 | SWP_NOMOVE.0 | SWP_NOSIZE.0);
        }

        if !WIDGET_DRAG_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
            // Enforce Frameless Popup Style (No Titlebar / Caption / Borders / Child)
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let target_style = (style & !WS_CHILD.0 & !WS_CAPTION.0 & !WS_THICKFRAME.0 & !WS_SYSMENU.0) | WS_POPUP.0;
            if style != target_style {
                println!("[WIDGET-LOG] WINDOWPOSCHANGING Style from 0x{:08X} to target 0x{:08X}", style, target_style);
                let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, target_style as isize);
            }

            // Enforce WS_EX_TOOLWINDOW and WS_EX_NOACTIVATE to hide from Taskbar and avoid mouse focus
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title_str = String::from_utf16_lossy(&title_buf[..len as usize]);
            let is_focusable = title_str.starts_with("Raven Notes Widget ") || title_str.starts_with("Raven Todo Widget ");

            let mut target_ex = ex_style | WS_EX_TOOLWINDOW.0 | WS_EX_LAYERED.0;
            if is_focusable {
                target_ex &= !WS_EX_NOACTIVATE.0;
            } else {
                target_ex |= WS_EX_NOACTIVATE.0;
            }
            target_ex &= !WS_EX_APPWINDOW.0;

            if ex_style != target_ex {
                println!("[WIDGET-LOG] WINDOWPOSCHANGING ExStyle from 0x{:08X} to target 0x{:08X}", ex_style, target_ex);
                let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex as isize);
                crate::window::remove_from_taskbar(hwnd);
            }
        }
    }

    // WM_SIZE (0x0005): Failsafe — if the widget somehow gets minimized
    // (e.g. via Win+D Show Desktop bypassing WINDOWPOSCHANGING), restore it immediately.
    // SIZE_MINIMIZED = 1
    if msg == 0x0005 && wparam.0 == 1 {
        println!("[WIDGET-LOG] WM_SIZE SIZE_MINIMIZED detected — restoring widget window");
        let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::SW_RESTORE,
        );
        return windows::Win32::Foundation::LRESULT(0);
    }

    if msg == WM_ENTERSIZEMOVE {
        WIDGET_DRAG_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
    } else if msg == WM_EXITSIZEMOVE
        || msg == WM_NCDESTROY
        || msg == 0x001C /* WM_CAPTURECHANGED */
        || msg == 0x0202 /* WM_LBUTTONUP */
        || msg == 0x00A2 /* WM_NCLBUTTONUP */
    {
        WIDGET_DRAG_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    // Log drag-related messages for debugging
    if msg == WM_EXITSIZEMOVE || msg == 0x001C /* WM_CAPTURECHANGED */ || msg == 0x0202 /* WM_LBUTTONUP */ || msg == 0x00A2 /* WM_NCLBUTTONUP */ || msg == WM_MOVE || msg == 0x0047 /* WM_WINDOWPOSCHANGED */ {
        println!("[WIDGET-DEBUG] Subclass received message: msg=0x{:04X}, wparam={}, lparam={}", msg, wparam.0, lparam.0);
    }

    // ── FORCE REPAINT DURING DRAG (fixes WS_EX_LAYERED ghosting) ───────────
    // During the modal move loop, WS_EX_LAYERED windows stop compositing and
    // appear to "ghost"/disappear until the drag ends.  Forcing a full
    // redraw + update on every WM_MOVE keeps the surface fresh so the widget
    // stays visible while it is being dragged.  RDW_INVALIDATE | RDW_UPDATENOW
    // | RDW_ALLCHILDREN ensures both the non-client and client areas repaint
    // synchronously within this message handler.
    if msg == WM_MOVE && WIDGET_DRAG_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
        let _ = RedrawWindow(
            hwnd,
            None,
            None,
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN,
        );
    }

    let mut trigger_save = false;
    if msg == WM_EXITSIZEMOVE {
        trigger_save = true;
    }

    if trigger_save {
        let dpi = GetDpiForWindow(hwnd);
        let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
        
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let phys_width = rect.right - rect.left;
        let phys_height = rect.bottom - rect.top;
        
        // Ensure the window has non-zero width/height and is not minimized
        if phys_width > 0 && phys_height > 0 && rect.left > -10000 {
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(monitor, &mut info);
            let screen_width = info.rcMonitor.right - info.rcMonitor.left;
            let screen_height = info.rcMonitor.bottom - info.rcMonitor.top;
            
            // Query window title to determine coordinate system and target setting
            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title_str = String::from_utf16_lossy(&title_buf[..len as usize]);
            
            if title_str.starts_with("Raven Clock Widget") {
                // Clock widget: top-right coordinate system
                let idx_str = &title_str["Raven Clock Widget".len()..];
                let idx: i32 = idx_str.trim().parse().unwrap_or(0);

                let mut phys_pos_x = info.rcMonitor.left + screen_width - phys_width - rect.left;
                let mut phys_pos_y = rect.top - info.rcMonitor.top;
                
                let max_x = (screen_width - phys_width).max(0);
                let max_y = (screen_height - phys_height).max(0);
                phys_pos_x = phys_pos_x.clamp(0, max_x);
                phys_pos_y = phys_pos_y.clamp(0, max_y);

                let pos_x = (phys_pos_x as f32 / scale).round() as i32;
                let pos_y = (phys_pos_y as f32 / scale).round() as i32;
                
                let current = crate::settings::RavenSettings::load();
                let inst = current.widgets.get_clock_instance(idx as usize);
                if (inst.pos_x - pos_x as f64).abs() > 0.5 
                    || (inst.pos_y - pos_y as f64).abs() > 0.5 
                {
                    crate::settings::update_clock_instance_setting(idx as usize, |instance| {
                        instance.pos_x = pos_x as f64;
                        instance.pos_y = pos_y as f64;
                    });
                }
            } else {
                // All other widgets use top-left coordinate system
                let prefixes = [
                        ("Raven Year Progress Widget ", "year_journey_pos_x", "year_journey_pos_y"),
                        ("Raven Day Progress Widget ", "day_journey_pos_x", "day_journey_pos_y"),
                        ("Raven Month Progress Widget ", "month_journey_pos_x", "month_journey_pos_y"),
                        ("Raven Media Widget ", "media_pos_x", "media_pos_y"),
                        ("Raven Notes Widget ", "notes_pos_x", "notes_pos_y"),
                        ("Raven Todo Widget ", "todo_pos_x", "todo_pos_y"),
                        ("Raven Quotes Widget ", "quotes_pos_x", "quotes_pos_y"),
                        ("Raven Picture Widget ", "picture_pos_x", "picture_pos_y"),
                        ("Raven Video Frame Widget ", "video_pos_x", "video_pos_y"),
                        ("Raven Battery Percentage Widget ", "battery_widget_pos_x", "battery_widget_pos_y"),
                        ("Raven Calendar Focus Widget ", "calendar_focus_pos_x", "calendar_focus_pos_y"),
                        ("Raven System Stats Widget ", "system_stats_widget_pos_x", "system_stats_widget_pos_y"),
                        ("Raven Apps Container Widget ", "apps_container_pos_x", "apps_container_pos_y"),
                        ("Raven Focus Score Widget ", "focus_score_widget_pos_x", "focus_score_widget_pos_y"),
                        ("Raven Calendar Widget ", "streak_widget_pos_x", "streak_widget_pos_y"),
                        // Also support the legacy Generic Widget title prefix
                        ("Raven Generic Widget - ", "", ""),
                    ];

                    for &(prefix, x_key, y_key) in &prefixes {
                        if title_str.starts_with(prefix) {
                            let instance_id = &title_str[prefix.len()..];
                            
                            let mut phys_pos_x = rect.left - info.rcMonitor.left;
                            let mut phys_pos_y = rect.top - info.rcMonitor.top;
                            
                            let max_x = (screen_width - phys_width).max(0);
                            let max_y = (screen_height - phys_height).max(0);
                            phys_pos_x = phys_pos_x.clamp(0, max_x);
                            phys_pos_y = phys_pos_y.clamp(0, max_y);
                            
                            let pos_x = (phys_pos_x as f32 / scale).round() as f64;
                            let pos_y = (phys_pos_y as f32 / scale).round() as f64;

                            let current = crate::settings::RavenSettings::load();
                            // 1. Check if this is a copy widget instance in the instances list
                            if current.widgets.instances.iter().any(|inst| inst.id == instance_id) {
                                let pos_x_i32 = pos_x.round() as i32;
                                let pos_y_i32 = pos_y.round() as i32;
                                if let Some(instance) = current.widgets.instances.iter().find(|inst| inst.id == instance_id) {
                                    if instance.x != pos_x_i32 || instance.y != pos_y_i32 {
                                        crate::settings::update_widget_instance_position(instance_id, pos_x_i32, pos_y_i32);
                                        println!("[WIDGET-DEBUG] Drag finished: successfully saved copy/instance widget '{}' position to x={}, y={}", instance_id, pos_x_i32, pos_y_i32);
                                    }
                                }
                            } else if !x_key.is_empty() && !y_key.is_empty() {
                                 // 2. Otherwise, if it has keys, save as the main static widget
                                 let old_x = get_setting_number_by_key(&current, x_key);
                                 let old_y = get_setting_number_by_key(&current, y_key);
                                 if (old_x - pos_x).abs() > 0.5 || (old_y - pos_y).abs() > 0.5 {
                                     crate::settings::set_number(&["widgets", x_key], pos_x);
                                     crate::settings::set_number(&["widgets", y_key], pos_y);
                                     println!("[WIDGET-DEBUG] Drag finished: successfully saved main widget '{}' position to x={}, y={}", x_key, pos_x, pos_y);
                                 }
                            }
                            break;
                        }
                    }
            }
        }
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub unsafe fn setup_widget_window(hwnd: HWND, click_through: bool, focusable: bool) {
    use windows::Win32::UI::WindowsAndMessaging::*;

    {
        let mut set = POSITIONED_HWNDS.lock().unwrap();
        set.remove(&hwnd.0);
    }
    
    // Early return if the window is already fully set up with all customized styles
    // (no caption, WS_POPUP, toolwindow, layered, and bound to the desktop host).
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let owner = GetWindow(hwnd, GW_OWNER);
    
    let has_popup = (style & WS_POPUP.0) != 0;
    let no_caption = (style & WS_CAPTION.0) == 0;
    let has_toolwindow = (ex_style & WS_EX_TOOLWINDOW.0) != 0;
    let has_layered = (ex_style & WS_EX_LAYERED.0) != 0;
    
    let mut needs_owner_bind = false;
    if let Some(host) = desktop_host_window() {
        if owner != host {
            needs_owner_bind = true;
        }
    }
    
    if has_popup && no_caption && has_toolwindow && has_layered && !needs_owner_bind {
        set_widget_click_through(hwnd, click_through);
        apply_widget_topmost_state(hwnd, widget_saved_topmost_state(hwnd));
        return;
    }

    println!("[WIDGET-DEBUG] setup_widget_window invoked on HWND: {:?}", hwnd);
    let target_style = (style & !WS_CHILD.0 & !WS_CAPTION.0 & !WS_THICKFRAME.0 & !WS_SYSMENU.0 & !WS_MINIMIZEBOX.0 & !WS_MAXIMIZEBOX.0) | WS_POPUP.0;
    let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, target_style as isize);

    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    // WS_EX_TOOLWINDOW: Hide from Taskbar and Alt-Tab switcher
    // WS_EX_NOACTIVATE: Avoid taking mouse click focus (only if not focusable)
    // WS_EX_LAYERED: Required for per-pixel alpha compositing (Slint transparent background)
    let mut target_ex = ex_style | WS_EX_TOOLWINDOW.0 | WS_EX_LAYERED.0;
    if focusable {
        target_ex &= !WS_EX_NOACTIVATE.0;
    } else {
        target_ex |= WS_EX_NOACTIVATE.0;
    }
    target_ex &= !WS_EX_APPWINDOW.0; // Ensure WS_EX_APPWINDOW is off
    
    if click_through {
        target_ex |= WS_EX_TRANSPARENT.0;
    } else {
        target_ex &= !WS_EX_TRANSPARENT.0;
    }
    
    let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex as isize);
    windows::Win32::UI::Shell::DragAcceptFiles(hwnd, true);
    
    // Bind the widget window's Owner to the system desktop wallpaper host window (Progman / WorkerW).
    // This is the key fix that prevents Win+D / "Show Desktop" from minimizing the widget:
    // Windows excludes windows owned by the desktop shell from the minimize sweep.
    let mut owner_bound = false;
    if let Some(host) = desktop_host_window() {
        println!("[WIDGET-LOG] Binding GWLP_HWNDPARENT: HWND {:?} -> desktop host {:?}", hwnd, host);
        let old_owner = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, host.0 as isize);
        println!("[WIDGET-LOG] GWLP_HWNDPARENT set (old owner: 0x{:X})", old_owner);
        owner_bound = true;
    } else {
        println!("[WIDGET-LOG] desktop_host_window() returned None — Win+D protection not applied");
    }
    
    // Apply style changes. DIAGNOSTIC: Use SWP_NOZORDER instead of HWND_BOTTOM to prevent z-slip behind WorkerW
    log_window_info("BEFORE setup_widget_window SetWindowPos", hwnd, None);
    let _ = SetWindowPos(
        hwnd,
        HWND(0),
        0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
    );
    log_window_info("AFTER setup_widget_window SetWindowPos", hwnd, None);
    apply_widget_topmost_state(hwnd, widget_saved_topmost_state(hwnd));

    // Immediately attempt to remove from taskbar (best-effort, may race with registration).
    crate::window::remove_from_taskbar(hwnd);

    // Also schedule a delayed removal to handle the Win32 race condition where
    // the taskbar hasn't registered the window yet.
    let hwnd_raw = hwnd.0;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let _ = slint::invoke_from_event_loop(move || {
            unsafe {
                crate::window::remove_from_taskbar(HWND(hwnd_raw));
            }
        });
    });

    // Set subclass to intercept dragging finish
    let sub_ok = windows::Win32::UI::Shell::SetWindowSubclass(
        hwnd,
        Some(widget_window_subclass_proc),
        9595,
        0,
    );

    println!("[WIDGET-DEBUG] HWND: {:?} | Style: 0x{:08X} (target: 0x{:08X}) | ExStyle: 0x{:08X} (target: 0x{:08X}) | OwnerBound: {} | SubclassRegistered: {:?}",
             hwnd, style, target_style, ex_style, target_ex, owner_bound, sub_ok);
}

pub unsafe fn set_widget_click_through(hwnd: HWND, click_through: bool) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let mut target_ex = ex_style;
    if click_through {
        target_ex |= WS_EX_TRANSPARENT.0;
    } else {
        target_ex &= !WS_EX_TRANSPARENT.0;
    }
    
    if target_ex != ex_style {
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex as isize);
        log_window_info("BEFORE set_widget_click_through SetWindowPos", hwnd, Some(HWND(0)));
        let _ = SetWindowPos(
            hwnd,
            HWND(0),
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );
        log_window_info("AFTER set_widget_click_through SetWindowPos", hwnd, Some(HWND(0)));
    }
}

pub unsafe fn position_widget_window(hwnd: HWND, pos_x: i32, pos_y: i32, width: i32, height: i32) {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    
    let dpi = GetDpiForWindow(hwnd);
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
    
    let phys_width = (width as f32 * scale) as i32;
    let phys_height = (height as f32 * scale) as i32;
    let phys_pos_x = (pos_x as f32 * scale) as i32;
    let phys_pos_y = (pos_y as f32 * scale) as i32;
    
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);
    let screen_width = info.rcMonitor.right - info.rcMonitor.left;
    let screen_height = info.rcMonitor.bottom - info.rcMonitor.top;
    let max_pos_x = (screen_width - phys_width).max(0);
    let max_pos_y = (screen_height - phys_height).max(0);
    let phys_pos_x = phys_pos_x.clamp(0, max_pos_x);
    let phys_pos_y = phys_pos_y.clamp(0, max_pos_y);
    
    // Coordinate translation: pos_x and pos_y are offsets from the top-right of the primary screen
    let x = info.rcMonitor.left + screen_width - phys_width - phys_pos_x;
    let y = info.rcMonitor.top + phys_pos_y;

    println!("[LIFECYCLE-LOG] position_widget_window (right-aligned): HWND: {:?}, target (pos_x: {}, pos_y: {}), physical (x: {}, y: {}, w: {}, h: {})",
             hwnd, pos_x, pos_y, x, y, phys_width, phys_height);

    log_window_info("BEFORE position_widget_window SetWindowPos", hwnd, Some(HWND(0)));
    let _ = SetWindowPos(
        hwnd,
        HWND(0),
        x, y, phys_width, phys_height,
        SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
    );
    log_window_info("AFTER position_widget_window SetWindowPos", hwnd, Some(HWND(0)));
    if widget_saved_topmost_state(hwnd) {
        apply_widget_topmost_state(hwnd, true);
    }

    // Mark as positioned so the periodic save timer won't overwrite saved coords
    {
        let mut set = POSITIONED_HWNDS.lock().unwrap();
        set.insert(hwnd.0);
    }
}

pub unsafe fn apply_widget_topmost_state(hwnd: windows::Win32::Foundation::HWND, topmost: bool) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    const HWND_TOPMOST_RAW: isize = -1;
    const HWND_NOTOPMOST_RAW: isize = -2;
    let insert_after = if topmost { HWND(HWND_TOPMOST_RAW) } else { HWND(HWND_NOTOPMOST_RAW) };
    with_widget_zorder_unlocked(hwnd, || {
        let _ = SetWindowPos(
            hwnd,
            insert_after,
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    });
}

pub unsafe fn send_widget_to_back(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    const HWND_NOTOPMOST_RAW: isize = -2;
    const HWND_BOTTOM_RAW: isize = 1;
    with_widget_zorder_unlocked(hwnd, || {
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW;
        let _ = SetWindowPos(hwnd, HWND(HWND_NOTOPMOST_RAW), 0, 0, 0, 0, flags);
        let _ = SetWindowPos(hwnd, HWND(HWND_BOTTOM_RAW), 0, 0, 0, 0, flags);
    });
}

pub unsafe fn position_widget_window_from_left(hwnd: HWND, pos_x: i32, pos_y: i32, width: i32, height: i32) {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;

    let dpi = GetDpiForWindow(hwnd);
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };

    let phys_width = ((width.max(120)) as f32 * scale) as i32;
    let phys_height = ((height.max(80)) as f32 * scale) as i32;
    let phys_pos_x = (pos_x as f32 * scale) as i32;
    let phys_pos_y = (pos_y as f32 * scale) as i32;

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);

    let screen_width = info.rcMonitor.right - info.rcMonitor.left;
    let screen_height = info.rcMonitor.bottom - info.rcMonitor.top;
    let max_pos_x = (screen_width - phys_width).max(0);
    let max_pos_y = (screen_height - phys_height).max(0);
    let phys_pos_x = phys_pos_x.clamp(0, max_pos_x);
    let phys_pos_y = phys_pos_y.clamp(0, max_pos_y);

    let x = info.rcMonitor.left + phys_pos_x;
    let y = info.rcMonitor.top + phys_pos_y;

    // Skip SetWindowPos if already at the exact target bounds.
    // Prevents multi-widget tug-of-war: unconditional SetWindowPos every sync-tick
    // causes DWM to batch-recalculate all windows together, glitching adjacent widgets.
    let mut cur = windows::Win32::Foundation::RECT::default();
    let _ = GetWindowRect(hwnd, &mut cur);
    if cur.left == x && cur.top == y
        && (cur.right - cur.left) == phys_width
        && (cur.bottom - cur.top) == phys_height
    {
        if widget_saved_topmost_state(hwnd) {
            apply_widget_topmost_state(hwnd, true);
        }
        return;
    }

    let _ = SetWindowPos(
        hwnd,
        HWND(0),
        x, y, phys_width, phys_height,
        SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
    );
    if widget_saved_topmost_state(hwnd) {
        apply_widget_topmost_state(hwnd, true);
    }

    // Mark as positioned so the periodic save timer won't overwrite saved coords
    // with a pre-position default value.
    {
        let mut set = POSITIONED_HWNDS.lock().unwrap();
        set.insert(hwnd.0);
    }
}

/// Exhaustive diagnostic logging for widget window state
pub unsafe fn log_exhaustive_widget_state(label: &str, hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::Foundation::RECT;

    let is_window = IsWindow(hwnd);
    let is_visible = IsWindowVisible(hwnd);
    let is_iconic = IsIconic(hwnd);
    let is_zoomed = IsZoomed(hwnd);

    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);

    let parent = GetParent(hwnd);
    let owner = GetWindow(hwnd, GW_OWNER);
    let prev = GetWindow(hwnd, GW_HWNDPREV);
    let next = GetWindow(hwnd, GW_HWNDNEXT);

    let dpi = GetDpiForWindow(hwnd);
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut minfo = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut minfo);

    // Check DWM cloaked state
    let mut cloaked: u32 = 0;
    let hr = windows::Win32::Graphics::Dwm::DwmGetWindowAttribute(
        hwnd,
        windows::Win32::Graphics::Dwm::DWMWA_CLOAKED,
        &mut cloaked as *mut u32 as *mut _,
        std::mem::size_of::<u32>() as u32,
    );
    let cloaked_str = if hr.is_ok() {
        format!("0x{:08X}", cloaked)
    } else {
        format!("query failed: {:?}", hr)
    };

    // Style flags decoded
    let has_ws_visible = (style & WS_VISIBLE.0) != 0;
    let has_ws_popup = (style & WS_POPUP.0) != 0;
    let has_ws_child = (style & WS_CHILD.0) != 0;
    let has_ws_caption = (style & WS_CAPTION.0) != 0;
    let has_ex_layered = (ex_style & WS_EX_LAYERED.0) != 0;
    let has_ex_transparent = (ex_style & WS_EX_TRANSPARENT.0) != 0;
    let has_ex_toolwindow = (ex_style & WS_EX_TOOLWINDOW.0) != 0;
    let has_ex_noactivate = (ex_style & WS_EX_NOACTIVATE.0) != 0;
    let has_ex_appwindow = (ex_style & WS_EX_APPWINDOW.0) != 0;

    // Get prev/next window titles for z-order context
    let prev_title = get_window_title(prev);
    let next_title = get_window_title(next);

    println!("\n╔══════════════════════════════════════════════════════════════");
    println!("║ [DIAG-EXHAUSTIVE] {}", label);
    println!("╠══════════════════════════════════════════════════════════════");
    println!("║ HWND:           {:?} (0x{:X})", hwnd, hwnd.0);
    println!("║ IsWindow:       {:?}", is_window);
    println!("║ IsWindowVisible:{:?}", is_visible);
    println!("║ IsIconic:       {:?}", is_iconic);
    println!("║ IsZoomed:       {:?}", is_zoomed);
    println!("║ DWM Cloaked:    {}", cloaked_str);
    println!("║ DPI:            {} (scale: {:.2}x)", dpi, scale);
    println!("╠── Geometry ─────────────────────────────────────────────────");
    println!("║ WindowRect:     L={} T={} R={} B={} ({}x{})",
             rect.left, rect.top, rect.right, rect.bottom,
             rect.right - rect.left, rect.bottom - rect.top);
    println!("║ MonitorRect:    L={} T={} R={} B={} ({}x{})",
             minfo.rcMonitor.left, minfo.rcMonitor.top, minfo.rcMonitor.right, minfo.rcMonitor.bottom,
             minfo.rcMonitor.right - minfo.rcMonitor.left, minfo.rcMonitor.bottom - minfo.rcMonitor.top);
    println!("║ MonitorWork:    L={} T={} R={} B={}",
             minfo.rcWork.left, minfo.rcWork.top, minfo.rcWork.right, minfo.rcWork.bottom);
    println!("╠── Styles ──────────────────────────────────────────────────");
    println!("║ Style:          0x{:08X} [VISIBLE={} POPUP={} CHILD={} CAPTION={}]",
             style, has_ws_visible, has_ws_popup, has_ws_child, has_ws_caption);
    println!("║ ExStyle:        0x{:08X} [LAYERED={} TRANSPARENT={} TOOLWINDOW={} NOACTIVATE={} APPWINDOW={}]",
             ex_style, has_ex_layered, has_ex_transparent, has_ex_toolwindow, has_ex_noactivate, has_ex_appwindow);
    println!("╠── Hierarchy ───────────────────────────────────────────────");
    println!("║ Parent:         {:?} (0x{:X})", parent, parent.0);
    println!("║ Owner:          {:?} (0x{:X})", owner, owner.0);
    println!("║ Z-Prev:         {:?} (0x{:X}) [{}]", prev, prev.0, prev_title);
    println!("║ Z-Next:         {:?} (0x{:X}) [{}]", next, next.0, next_title);
    println!("╚══════════════════════════════════════════════════════════════\n");
}

unsafe fn get_window_title(hwnd: HWND) -> String {
    if hwnd.0 == 0 {
        return "(null)".to_string();
    }
    use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
    let mut buf = [0u16; 256];
    let len = GetWindowTextW(hwnd, &mut buf);
    if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        "(no title)".to_string()
    }
}

// Native Helpers for volume and brightness sliders inside widgets
pub fn set_exact_volume(value: i32) {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitialize(None);
        if let Ok(device_enumerator) = windows::Win32::System::Com::CoCreateInstance::<_, windows::Win32::Media::Audio::IMMDeviceEnumerator>(&windows::Win32::Media::Audio::MMDeviceEnumerator, None, windows::Win32::System::Com::CLSCTX_ALL) {
            if let Ok(default_device) = device_enumerator.GetDefaultAudioEndpoint(windows::Win32::Media::Audio::eRender, windows::Win32::Media::Audio::eConsole) {
                if let Ok(endpoint_volume) = default_device.Activate::<windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume>(windows::Win32::System::Com::CLSCTX_ALL, None) {
                    let scalar = (value as f32 / 100.0).clamp(0.0, 1.0);
                    let _ = endpoint_volume.SetMasterVolumeLevelScalar(scalar, std::ptr::null());
                }
            }
        }
    }
}

pub fn get_exact_volume() -> i32 {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitialize(None);
        if let Ok(device_enumerator) = windows::Win32::System::Com::CoCreateInstance::<_, windows::Win32::Media::Audio::IMMDeviceEnumerator>(&windows::Win32::Media::Audio::MMDeviceEnumerator, None, windows::Win32::System::Com::CLSCTX_ALL) {
            if let Ok(default_device) = device_enumerator.GetDefaultAudioEndpoint(windows::Win32::Media::Audio::eRender, windows::Win32::Media::Audio::eConsole) {
                if let Ok(endpoint_volume) = default_device.Activate::<windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume>(windows::Win32::System::Com::CLSCTX_ALL, None) {
                    if let Ok(scalar) = endpoint_volume.GetMasterVolumeLevelScalar() {
                        return (scalar * 100.0).round() as i32;
                    }
                }
            }
        }
    }
    50
}

static BRIGHTNESS_QUERY_IN_FLIGHT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
static BRIGHTNESS_CACHE_VALUE: std::sync::atomic::AtomicI32 =
    std::sync::atomic::AtomicI32::new(-1);
static BRIGHTNESS_CACHE_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

fn current_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn set_exact_brightness(value: i32) {
    BRIGHTNESS_CACHE_VALUE.store(value.clamp(0, 100), std::sync::atomic::Ordering::SeqCst);
    BRIGHTNESS_CACHE_MS.store(current_millis(), std::sync::atomic::Ordering::SeqCst);
    use std::os::windows::process::CommandExt;
    let script = format!(
        "Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue | ForEach-Object {{ $_.WmiSetBrightness(1, {}) | Out-Null }}",
        value
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn();
}

pub fn get_exact_brightness() -> Option<i32> {
    let now = current_millis();
    let cached_at = BRIGHTNESS_CACHE_MS.load(std::sync::atomic::Ordering::SeqCst);
    let cached = BRIGHTNESS_CACHE_VALUE.load(std::sync::atomic::Ordering::SeqCst);
    if cached >= 0 && now.saturating_sub(cached_at) < 5_000 {
        return Some(cached.clamp(0, 100));
    }

    if BRIGHTNESS_QUERY_IN_FLIGHT
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_err()
    {
        return (cached >= 0).then_some(cached.clamp(0, 100));
    }

    struct QueryGuard;
    impl Drop for QueryGuard {
        fn drop(&mut self) {
            BRIGHTNESS_QUERY_IN_FLIGHT.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let _guard = QueryGuard;

    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-Command",
            "(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty CurrentBrightness)",
        ])
        .creation_flags(0x08000000)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<i32>()
        .ok()
        .map(|value| value.clamp(0, 100));
    if let Some(value) = value {
        BRIGHTNESS_CACHE_VALUE.store(value, std::sync::atomic::Ordering::SeqCst);
        BRIGHTNESS_CACHE_MS.store(current_millis(), std::sync::atomic::Ordering::SeqCst);
    }
    value
}

pub unsafe fn log_window_info(label: &str, hwnd: HWND, insert_after: Option<HWND>) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let parent = GetParent(hwnd);
    let owner = GetWindow(hwnd, GW_OWNER);
    let is_visible = IsWindowVisible(hwnd);
    let mut rect = windows::Win32::Foundation::RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    
    let insert_str = if let Some(ins) = insert_after {
        format!("{:?}", ins)
    } else {
        "None (SWP_NOZORDER)".to_string()
    };
    
    println!(
        "[LIFECYCLE-LOG-Z] {} | HWND: {:?} | InsertAfter: {} | Owner: {:?} | Parent: {:?} | Style: 0x{:08X} | ExStyle: 0x{:08X} | IsVisible: {:?} | Rect: [{}, {}, {}, {}]",
        label, hwnd, insert_str, owner, parent, style, ex_style, is_visible, rect.left, rect.top, rect.right, rect.bottom
    );
}
