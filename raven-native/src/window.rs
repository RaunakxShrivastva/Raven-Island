use crate::renderer::NativeRenderer;
use crate::services::{CaptureRegion, ServiceRegistry};
use crate::settings::RavenSettings;
use crate::widgets::NativeAction;
use std::cell::RefCell;
use std::ffi::c_void;
use thiserror::Error;
use windows::core::{Error as WindowsError, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, POINTL, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetMonitorInfoW,
    InvalidateRect, MonitorFromPoint, Rectangle, SelectObject, HGDIOBJ, MONITORINFO,
    PAINTSTRUCT, PS_SOLID,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, ReleaseCapture, SetCapture, UnregisterHotKey, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
    VK_ESCAPE, HOT_KEY_MODIFIERS,
};

const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_MOUSEWHEEL: u32 = 0x020A;
const WM_MOUSEHWHEEL: u32 = 0x020E;
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, Shell_NotifyIconW, HDROP, NIF_ICON, NIF_MESSAGE,
    NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
    SHAppBarMessage, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, GetWindowRect, LoadCursorW,
    LoadIconW, PostQuitMessage, RegisterClassW, SetLayeredWindowAttributes, SetTimer,
    SetWindowPos, ShowWindow, IDI_APPLICATION, CreatePopupMenu, InsertMenuW, TrackPopupMenu, DestroyMenu, SetForegroundWindow,
    MF_BYPOSITION, MF_STRING, TPM_RETURNCMD, TPM_NONOTIFY, TPM_BOTTOMALIGN,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HMENU, IDC_ARROW,
    KillTimer, LWA_ALPHA, LWA_COLORKEY, MSG, SW_SHOW, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    WM_CREATE, WM_DESTROY, WM_DROPFILES, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCCREATE, WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

pub type WindowResult<T> = Result<T, WindowError>;

#[derive(Debug, Error)]
pub enum WindowError {
    #[error("Win32 call failed: {0}")]
    Win32(String),
}

pub struct NativeWindow {
    pub hwnd: HWND,
    pub motion_bridge: crate::motion::SharedMotionBridge,
}

struct WindowState {
    settings: RefCell<RavenSettings>,
    services: ServiceRegistry,
    renderer: RefCell<NativeRenderer>,
    motion_bridge: crate::motion::SharedMotionBridge,
}

struct RegionOverlayState {
    origin: POINT,
    start: Option<POINT>,
    current: POINT,
    selected: Option<CaptureRegion>,
    done: bool,
}

const NOTCH_ANIMATION_TIMER: usize = 1;
const CLOCK_REFRESH_TIMER: usize = 2;
const STATS_REFRESH_TIMER: usize = 3;
const MEDIA_REFRESH_TIMER: usize = 4;
const CLOCK_WIDGET_TIMER: usize = 5;
const NOTIFICATIONS_REFRESH_TIMER: usize = 6;
const CALENDAR_REFRESH_TIMER: usize = 7;
const HOTKEY_HOME: i32 = 100;
const HOTKEY_MEDIA: i32 = 101;
const HOTKEY_CALENDAR: i32 = 102;
const HOTKEY_CLOCK: i32 = 103;
const HOTKEY_DROP: i32 = 104;
const HOTKEY_NOTIFICATIONS: i32 = 105;
const HOTKEY_CAPTURE: i32 = 106;
const HOTKEY_STATS: i32 = 107;
const HOTKEY_SETTINGS: i32 = 108;
const HOTKEY_CLIPBOARD: i32 = 109;
const HOTKEY_QUICK_SCREENSHOT: i32 = 200;
const HOTKEY_TOGGLE_RAVEN: i32 = 201;
const HOTKEY_TOGGLE_FREEZE: i32 = 202;
const HOTKEY_MEDIA_PLAY: i32 = 203;
const HOTKEY_MEDIA_NEXT: i32 = 204;
const HOTKEY_MEDIA_PREV: i32 = 205;
const HOTKEY_QUICK_RECORD_TOGGLE: i32 = 206;
const HOTKEY_RESTART_RAVEN: i32 = 207;
const HOTKEY_QUIT_RAVEN: i32 = 208;
const HOTKEY_TOPBAR_STATS: i32 = 209;
const HOTKEY_TOPBAR_VOLUME: i32 = 210;
const HOTKEY_TOPBAR_WIFI: i32 = 211;
const HOTKEY_TOPBAR_TIMER: i32 = 212;
const HOTKEY_TOPBAR_CALENDAR: i32 = 213;
const TRAY_ICON_ID: u32 = 1;
const WM_TRAYICON: u32 = 0x8000 + 1;
const FRAME_MS: u32 = 16;
const CLOCK_REFRESH_MS: u32 = 30_000;
const STATS_REFRESH_MS: u32 = 10_000;
const MEDIA_REFRESH_MS: u32 = 5_000;
const CLOCK_WIDGET_MS: u32 = 5_000;
const NOTIFICATIONS_REFRESH_MS: u32 = 15_000;
const CALENDAR_REFRESH_MS: u32 = 300_000;

impl NativeWindow {
    pub fn create(
        settings: RavenSettings,
        services: ServiceRegistry,
        renderer: NativeRenderer,
    ) -> WindowResult<Self> {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let class_name = wide("RavenNativeOverlay");
        let title = wide("Raven Native");
        let instance = unsafe { GetModuleHandleW(None).map_err(|e| WindowError::Win32(e.to_string()))? };

        let window_class = WNDCLASSW {
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
            hInstance: instance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassW(&window_class) };
        let _ = atom;

        let width = settings.appearance.idle_width.max(10.0).round() as i32;
        let height = settings.appearance.idle_height.max(10.0).round() as i32;
        let state = Box::new(WindowState {
            settings: RefCell::new(settings.clone()),
            services,
            renderer: RefCell::new(renderer),
            motion_bridge: crate::motion::SharedMotionBridge::new(),
        });

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_POPUP,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                HWND(0),
                HMENU(0),
                instance,
                Some(Box::into_raw(state) as *const c_void),
            )
        };

        if hwnd.0 == 0 {
            return Err(WindowError::Win32(format!(
                "CreateWindowExW failed with {}",
                WindowsError::from_win32()
            )));
        }

        PILL_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

        unsafe {
            let _ = SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), 255, LWA_COLORKEY);
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
            DragAcceptFiles(hwnd, true);
            position_at_top_center(hwnd, width, height, settings.appearance.pill_offset, settings.appearance.pill_y_offset);
            ShowWindow(hwnd, SW_SHOW);
        }

        Ok(Self { hwnd, motion_bridge: crate::motion::SharedMotionBridge::new() })
    }

    pub fn create_hidden(
        settings: RavenSettings,
        services: ServiceRegistry,
    ) -> WindowResult<Self> {
        let class_name = wide("RavenNativeHidden");
        let title = wide("Raven Native Hidden");
        let instance = unsafe { GetModuleHandleW(None).map_err(|e| WindowError::Win32(e.to_string()))? };

        let window_class = WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            lpfnWndProc: Some(window_proc),
            ..Default::default()
        };

        unsafe { let _ = RegisterClassW(&window_class); }

        let bridge = crate::motion::SharedMotionBridge::new();
        let state = Box::new(WindowState {
            settings: RefCell::new(settings),
            services,
            renderer: RefCell::new(crate::renderer::NativeRenderer::new(crate::settings::RavenSettings::load())),
            motion_bridge: bridge.clone(),
        });

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::WS_POPUP,
                0,
                0,
                0,
                0,
                HWND(0),
                HMENU(0),
                instance,
                Some(Box::into_raw(state) as *const c_void),
            )
        };

        if hwnd.0 == 0 {
            return Err(WindowError::Win32(format!("CreateWindowExW failed for hidden window")));
        }

        HIDDEN_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

        Ok(Self { hwnd, motion_bridge: bridge })
    }

    pub fn run_message_loop(&self) -> WindowResult<()> {
        unsafe {
            let mut message = MSG::default();
            while GetMessageW(&mut message, HWND(0), 0, 0).into() {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(())
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        // Custom message to show settings from a second instance (0x8000 + 1337)
        0x8539 => {
            if let Some(state) = state(hwnd) {
                state.services.event_bus().emit(crate::events::RavenEvent::ShowSettings);
            }
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_COPYDATA => {
            let cds = lparam.0 as *const windows::Win32::System::DataExchange::COPYDATASTRUCT;
            if !cds.is_null() && (*cds).dwData == 0x1337 {
                let len = (*cds).cbData as usize;
                let ptr = (*cds).lpData as *const u8;
                let data = std::slice::from_raw_parts(ptr, len);
                if let Ok(token_str) = String::from_utf8(data.to_vec()) {
                    println!("[WM_COPYDATA] Received sign-in token from second instance!");
                    if let Some(state) = state(hwnd) {
                        state.services.event_bus().emit(crate::events::RavenEvent::AccountTokenReceived(token_str));
                    }
                }
            }
            LRESULT(1)
        }
        WM_NCCREATE => {
            let createstruct = lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
            let state = (*createstruct).lpCreateParams as *mut WindowState;
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            LRESULT(1)
        }
        WM_CREATE => {
            SetTimer(hwnd, CLOCK_REFRESH_TIMER, CLOCK_REFRESH_MS, None);
            SetTimer(hwnd, STATS_REFRESH_TIMER, STATS_REFRESH_MS, None);
            SetTimer(hwnd, MEDIA_REFRESH_TIMER, MEDIA_REFRESH_MS, None);
            SetTimer(hwnd, CLOCK_WIDGET_TIMER, CLOCK_WIDGET_MS, None);
            SetTimer(hwnd, NOTIFICATIONS_REFRESH_TIMER, NOTIFICATIONS_REFRESH_MS, None);
            SetTimer(hwnd, CALENDAR_REFRESH_TIMER, CALENDAR_REFRESH_MS, None);
            let loaded_settings = if let Some(state) = state(hwnd) {
                state.settings.borrow().clone()
            } else {
                crate::settings::RavenSettings::load()
            };
            register_raven_hotkeys(hwnd, &loaded_settings);
            add_tray_icon(hwnd);
            LRESULT(0)
        }
        WM_PAINT => {
            if let Some(state) = state(hwnd) {
                state.renderer.borrow_mut().render(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if let Some(state) = state(hwnd) {
                let (x, y) = lparam_point(lparam);
                let outcome = state.renderer.borrow_mut().handle_click(x, y);
                if let Some(action) = outcome.action {
                    run_native_action(state, action);
                }
                apply_renderer_geometry(hwnd, state);
                InvalidateRect(hwnd, None, false);
                if outcome.animate {
                    SetTimer(hwnd, NOTCH_ANIMATION_TIMER, FRAME_MS, None);
                }
            }
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            if let Some(state) = state(hwnd) {
                if state.renderer.borrow().phase() != crate::motion::NotchPhase::Closed {
                    state.renderer.borrow_mut().toggle();
                    SetTimer(hwnd, NOTCH_ANIMATION_TIMER, FRAME_MS, None);
                }
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if let Some(state) = state(hwnd) {
                if crate::HOVER_ENABLED.load(std::sync::atomic::Ordering::SeqCst) && state.renderer.borrow().phase() == crate::motion::NotchPhase::Closed {
                    state.renderer.borrow_mut().toggle();
                    SetTimer(hwnd, NOTCH_ANIMATION_TIMER, FRAME_MS, None);
                    
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    unsafe { let _ = TrackMouseEvent(&mut tme); }
                }
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if let Some(state) = state(hwnd) {
                let mut pt = POINT::default();
                unsafe { GetCursorPos(&mut pt).unwrap_or_default() };
                let mut rect = RECT::default();
                unsafe { let _ = GetWindowRect(hwnd, &mut rect); }
                
                if pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.bottom {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    unsafe { let _ = TrackMouseEvent(&mut tme); }
                } else if crate::HOVER_ENABLED.load(std::sync::atomic::Ordering::SeqCst) && state.renderer.borrow().phase() != crate::motion::NotchPhase::Closed {
                    state.renderer.borrow_mut().toggle();
                    SetTimer(hwnd, NOTCH_ANIMATION_TIMER, FRAME_MS, None);
                }
            }
            LRESULT(0)
        }
        WM_DROPFILES => {
            if let Some(state) = state(hwnd) {
                let paths = dropped_paths(HDROP(wparam.0 as isize));
                let snapshot = state.services.add_shelf_paths(paths);
                state.renderer.borrow_mut().select_tab(crate::widgets::NativeTab::Drop);
                state.renderer.borrow_mut().set_snapshot(snapshot);
                InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            handle_hotkey(hwnd, wparam.0 as i32);
            LRESULT(0)
        }
        0x0319 /* WM_APPCOMMAND */ => {
            let command = ((lparam.0 >> 16) & 0x7fff) as i32;
            match command {
                8 => show_volume_hud_from_key(true),  // APPCOMMAND_VOLUME_MUTE
                9 => show_volume_hud_from_key(false), // APPCOMMAND_VOLUME_DOWN
                10 => show_volume_hud_from_key(false), // APPCOMMAND_VOLUME_UP
                _ => {}
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_TRAYICON => {
            if let Some(state) = state(hwnd) {
                let event = (lparam.0 & 0xFFFF) as u32;
                match event {
                    WM_LBUTTONUP => {
                        state.renderer.borrow_mut().toggle();
                        apply_renderer_geometry(hwnd, state);
                        InvalidateRect(hwnd, None, false);
                        SetTimer(hwnd, NOTCH_ANIMATION_TIMER, FRAME_MS, None);
                    }
                    WM_RBUTTONUP | 0x007B /* WM_CONTEXTMENU */ => {
                        let mut pt = POINT::default();
                        unsafe {
                            let _ = GetCursorPos(&mut pt);
                            let menu = CreatePopupMenu().unwrap_or_default();
                            if !menu.is_invalid() {
                                let _ = InsertMenuW(menu, 0, MF_BYPOSITION | MF_STRING, 1, PCWSTR(wide("Settings").as_ptr()));
                                let _ = InsertMenuW(menu, 1, MF_BYPOSITION | MF_STRING, 2, PCWSTR(wide("Restart").as_ptr()));
                                let _ = InsertMenuW(menu, 2, MF_BYPOSITION | MF_STRING, 3, PCWSTR(wide("Quit Raven Notch").as_ptr()));
                                
                                SetForegroundWindow(hwnd);
                                let cmd = TrackPopupMenu(
                                    menu,
                                    TPM_RETURNCMD | TPM_NONOTIFY | TPM_BOTTOMALIGN,
                                    pt.x,
                                    pt.y,
                                    0,
                                    hwnd,
                                    None,
                                );
                                let _ = DestroyMenu(menu);
                                
                                match cmd.0 {
                                    1 => {
                                        state.services.event_bus().emit(crate::events::RavenEvent::ShowSettings);
                                    }
                                    2 => {
                                        // Restart
                                        let _ = DestroyWindow(hwnd);
                                        if let Ok(exe_path) = std::env::current_exe() {
                                            let _ = std::process::Command::new(exe_path).spawn();
                                        }
                                        std::process::exit(0);
                                    }
                                    3 => {
                                        // Quit
                                        let _ = DestroyWindow(hwnd);
                                        std::process::exit(0);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == NOTCH_ANIMATION_TIMER {
                if let Some(state) = state(hwnd) {
                    let keep_animating = state.renderer.borrow_mut().tick();
                    apply_renderer_geometry(hwnd, state);
                    InvalidateRect(hwnd, None, false);

                    // Phase 5: Push physics snapshot to Slint bridge
                    let motion = state.renderer.borrow();
                    let geo = motion.geometry();
                    let is_open = motion.phase() != crate::motion::NotchPhase::Closed;
                    let phase = motion.phase();
                    let panel_ready = matches!(
                        phase,
                        crate::motion::NotchPhase::OpenContentStaging | crate::motion::NotchPhase::Open | crate::motion::NotchPhase::ClosingContent
                    );
                    state.motion_bridge.write(crate::motion::MotionSnapshot {
                        content_opacity: motion.content_opacity(),
                        border_radius: geo.radius,
                        width: geo.width,
                        height: geo.height,
                        is_open,
                        phase,
                        panel_ready,
                    });

                    if !keep_animating {
                        let _ = KillTimer(hwnd, NOTCH_ANIMATION_TIMER);
                    }
                }
                return LRESULT(0);
            }
            if wparam.0 == CLOCK_REFRESH_TIMER {
                if let Some(state) = state(hwnd) {
                    let snapshot = state.services.refresh_clock();
                    state.renderer.borrow_mut().set_snapshot(snapshot);
                    InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            if wparam.0 == STATS_REFRESH_TIMER {
                if let Some(state) = state(hwnd) {
                    if state.renderer.borrow().phase() == crate::motion::NotchPhase::Closed {
                        return LRESULT(0);
                    }
                    let snapshot = state.services.refresh_stats();
                    state.renderer.borrow_mut().set_snapshot(snapshot);
                    InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            if wparam.0 == MEDIA_REFRESH_TIMER {
                if let Some(state) = state(hwnd) {
                    if state.renderer.borrow().phase() == crate::motion::NotchPhase::Closed {
                        return LRESULT(0);
                    }
                    let snapshot = state.services.refresh_media();
                    state.renderer.borrow_mut().set_snapshot(snapshot);
                    InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            if wparam.0 == CLOCK_WIDGET_TIMER {
                if let Some(state) = state(hwnd) {
                    if state.renderer.borrow().phase() == crate::motion::NotchPhase::Closed {
                        return LRESULT(0);
                    }
                    let snapshot = state.services.refresh_clock();
                    state.renderer.borrow_mut().set_snapshot(snapshot);
                    InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            if wparam.0 == NOTIFICATIONS_REFRESH_TIMER {
                if let Some(state) = state(hwnd) {
                    if state.renderer.borrow().phase() == crate::motion::NotchPhase::Closed {
                        return LRESULT(0);
                    }
                    let snapshot = state.services.refresh_notifications();
                    state.renderer.borrow_mut().set_snapshot(snapshot);
                    InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            if wparam.0 == CALENDAR_REFRESH_TIMER {
                if let Some(state) = state(hwnd) {
                    if state.renderer.borrow().phase() == crate::motion::NotchPhase::Closed {
                        return LRESULT(0);
                    }
                    let snapshot = state.services.refresh_calendar();
                    state.renderer.borrow_mut().set_snapshot(snapshot);
                    InvalidateRect(hwnd, None, false);
                }
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            unregister_raven_hotkeys(hwnd);
            remove_tray_icon(hwnd);
            let ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
            if !ptr.is_null() {
                let _ = Box::from_raw(ptr);
                windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn select_capture_region() -> Option<CaptureRegion> {
    let class_name = wide("RavenCaptureRegionOverlay");
    let title = wide("Raven Region Capture");
    let instance = GetModuleHandleW(None).ok()?;
    let window_class = WNDCLASSW {
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hInstance: instance.into(),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(region_overlay_proc),
        ..Default::default()
    };
    let _ = RegisterClassW(&window_class);

    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    let monitor = MonitorFromPoint(
        cursor,
        windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
    );
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);
    let bounds = info.rcMonitor;
    let mut overlay = Box::new(RegionOverlayState {
        origin: POINT {
            x: bounds.left,
            y: bounds.top,
        },
        start: None,
        current: POINT::default(),
        selected: None,
        done: false,
    });
    let state_ptr = overlay.as_mut() as *mut RegionOverlayState;
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
        PCWSTR(class_name.as_ptr()),
        PCWSTR(title.as_ptr()),
        WS_POPUP,
        bounds.left,
        bounds.top,
        bounds.right - bounds.left,
        bounds.bottom - bounds.top,
        HWND(0),
        HMENU(0),
        instance,
        Some(state_ptr.cast()),
    );
    if hwnd.0 == 0 {
        return None;
    }
    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 210, LWA_ALPHA);
    ShowWindow(hwnd, SW_SHOW);
    let _ = SetCapture(hwnd);
    InvalidateRect(hwnd, None, true);

    let mut message = MSG::default();
    while !overlay.done && GetMessageW(&mut message, HWND(0), 0, 0).into() {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    let _ = ReleaseCapture();
    overlay.selected
}

unsafe extern "system" fn region_overlay_proc(
    hwnd: HWND,
    msg: u32,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let createstruct = lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
            let state = (*createstruct).lpCreateParams as *mut RegionOverlayState;
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            LRESULT(1)
        }
        WM_PAINT => {
            if let Some(state) = region_state(hwnd) {
                let mut paint = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut paint);
                let mut rect = RECT::default();
                let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
                let dim = CreateSolidBrush(COLORREF(0x000000));
                FillRect(hdc, &rect, dim);
                let _ = DeleteObject(HGDIOBJ(dim.0));

                if let Some(start) = state.start {
                    let selection = normalized_rect(start, state.current);
                    let border = CreatePen(PS_SOLID, 3, COLORREF(0x00FFFFFF));
                    let old = SelectObject(hdc, HGDIOBJ(border.0));
                    Rectangle(
                        hdc,
                        selection.left,
                        selection.top,
                        selection.right,
                        selection.bottom,
                    );
                    let _ = SelectObject(hdc, old);
                    let _ = DeleteObject(HGDIOBJ(border.0));
                }
                EndPaint(hwnd, &paint);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            if let Some(state) = region_state(hwnd) {
                let (x, y) = lparam_point(lparam);
                let point = POINT { x, y };
                state.start = Some(point);
                state.current = point;
                InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if let Some(state) = region_state(hwnd) {
                if state.start.is_some() {
                    let (x, y) = lparam_point(lparam);
                    state.current = POINT { x, y };
                    InvalidateRect(hwnd, None, true);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if let Some(state) = region_state(hwnd) {
                let (x, y) = lparam_point(lparam);
                state.current = POINT { x, y };
                if let Some(start) = state.start {
                    let rect = normalized_rect(start, state.current);
                    let width = (rect.right - rect.left).max(1) as u32;
                    let height = (rect.bottom - rect.top).max(1) as u32;
                    if width > 8 && height > 8 {
                        state.selected = Some(CaptureRegion {
                            x: (rect.left + state.origin.x).max(0) as u32,
                            y: (rect.top + state.origin.y).max(0) as u32,
                            width,
                            height,
                        });
                    }
                }
                state.done = true;
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if _wparam.0 as u32 == VK_ESCAPE.0 as u32 {
                if let Some(state) = region_state(hwnd) {
                    state.done = true;
                }
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = region_state(hwnd) {
                state.done = true;
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, _wparam, lparam),
    }
}

unsafe fn region_state<'a>(hwnd: HWND) -> Option<&'a mut RegionOverlayState> {
    let ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_USERDATA)
        as *mut RegionOverlayState;
    ptr.as_mut()
}

fn normalized_rect(a: POINT, b: POINT) -> RECT {
    RECT {
        left: a.x.min(b.x),
        top: a.y.min(b.y),
        right: a.x.max(b.x),
        bottom: a.y.max(b.y),
    }
}

unsafe fn tray_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: load_app_icon(),
        ..Default::default()
    };
    let tip = wide("Raven Native");
    for (index, unit) in tip.into_iter().take(data.szTip.len()).enumerate() {
        data.szTip[index] = unit;
    }
    data
}

unsafe fn load_app_icon() -> windows::Win32::UI::WindowsAndMessaging::HICON {
    let instance = GetModuleHandleW(None).unwrap_or_default();
    let handle = windows::Win32::UI::WindowsAndMessaging::LoadImageW(
        instance,
        PCWSTR(1 as *const u16), // Resource ID 1 (standard embedded app icon)
        windows::Win32::UI::WindowsAndMessaging::IMAGE_ICON,
        0,
        0,
        windows::Win32::UI::WindowsAndMessaging::LR_DEFAULTSIZE,
    ).unwrap_or_default();
    
    if handle.is_invalid() {
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    } else {
        windows::Win32::UI::WindowsAndMessaging::HICON(handle.0)
    }
}

unsafe fn add_tray_icon(hwnd: HWND) {
    let mut data = tray_icon_data(hwnd);
    let _ = Shell_NotifyIconW(NIM_ADD, &data);
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let data = tray_icon_data(hwnd);
    let _ = Shell_NotifyIconW(NIM_DELETE, &data);
}

fn parse_shortcut(shortcut: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = shortcut.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return None;
    }
    
    let mut modifiers = 0u32;
    let mut vk = None;
    
    for part in parts {
        let part_lower = part.to_lowercase();
        match part_lower.as_str() {
            "alt" => { modifiers |= 0x0001; } // MOD_ALT
            "control" | "ctrl" => { modifiers |= 0x0002; } // MOD_CONTROL
            "shift" | "⇧" => { modifiers |= 0x0004; } // MOD_SHIFT
            "super" | "win" | "command" | "⊞" => { modifiers |= 0x0008; } // MOD_WIN
            "space" => { vk = Some(0x20); } // VK_SPACE
            "comma" => { vk = Some(0xBC); } // VK_OEM_COMMA
            "period" => { vk = Some(0xBE); } // VK_OEM_PERIOD
            "slash" => { vk = Some(0xBF); } // VK_OEM_2
            "minus" => { vk = Some(0xBD); } // VK_OEM_MINUS
            "plus" => { vk = Some(0xBB); } // VK_OEM_PLUS
            "tab" => { vk = Some(0x09); } // VK_TAB
            "escape" | "esc" => { vk = Some(0x1B); } // VK_ESCAPE
            "enter" | "return" => { vk = Some(0x0D); } // VK_RETURN
            "backspace" => { vk = Some(0x08); } // VK_BACK
            "up" | "arrowup" => { vk = Some(0x26); } // VK_UP
            "down" | "arrowdown" => { vk = Some(0x28); } // VK_DOWN
            "left" | "arrowleft" => { vk = Some(0x25); } // VK_LEFT
            "right" | "arrowright" => { vk = Some(0x27); } // VK_RIGHT
            "home" => { vk = Some(0x24); } // VK_HOME
            "end" => { vk = Some(0x23); } // VK_END
            "pageup" | "pgup" | "page up" => { vk = Some(0x21); } // VK_PRIOR
            "pagedown" | "pgdn" | "page down" => { vk = Some(0x22); } // VK_NEXT
            "insert" | "ins" => { vk = Some(0x2D); } // VK_INSERT
            "delete" | "del" => { vk = Some(0x2E); } // VK_DELETE
            "semicolon" | ";" => { vk = Some(0xBA); } // VK_OEM_1
            "backtick" | "`" => { vk = Some(0xC0); } // VK_OEM_3
            "bracketleft" | "[" => { vk = Some(0xDB); } // VK_OEM_4
            "backslash" | "\\" => { vk = Some(0xDC); } // VK_OEM_5
            "bracketright" | "]" => { vk = Some(0xDD); } // VK_OEM_6
            "quote" | "'" => { vk = Some(0xDE); } // VK_OEM_7
            other => {
                if other.len() == 1 {
                    let c = other.chars().next().unwrap();
                    if c.is_alphanumeric() {
                        vk = Some(c.to_ascii_uppercase() as u32);
                    } else {
                        vk = match c {
                            ';' => Some(0xBA),
                            '=' => Some(0xBB),
                            '+' => Some(0xBB),
                            ',' => Some(0xBC),
                            '-' => Some(0xBD),
                            '.' => Some(0xBE),
                            '/' => Some(0xBF),
                            '`' => Some(0xC0),
                            '[' => Some(0xDB),
                            '\\' => Some(0xDC),
                            ']' => Some(0xDD),
                            '\'' => Some(0xDE),
                            _ => None,
                        };
                    }
                } else if other.starts_with('f') {
                    if let Ok(num) = other[1..].parse::<u32>() {
                        if num >= 1 && num <= 24 {
                            vk = Some(0x70 + num - 1); // VK_F1 is 0x70
                        }
                    }
                }
            }
        }
    }
    
    vk.map(|k| (modifiers, k))
}

static MAGIC_HOTKEY_IDS: std::sync::Mutex<Vec<i32>> = std::sync::Mutex::new(Vec::new());
static GLOBAL_MAGIC_HOOK: std::sync::Mutex<Option<windows::Win32::UI::WindowsAndMessaging::HHOOK>> = std::sync::Mutex::new(None);
static LAST_SYSTEM_HUD_KEY_MS: AtomicU64 = AtomicU64::new(0);
static BRIGHTNESS_HUD_WATCHER_STARTED: AtomicBool = AtomicBool::new(false);
static VOLUME_HUD_WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

fn current_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn show_volume_hud_from_key(muted: bool) {
    let now = current_millis();
    if now.saturating_sub(LAST_SYSTEM_HUD_KEY_MS.swap(now, Ordering::SeqCst)) < 45 {
        return;
    }
    show_system_hud("volume", crate::widgets::get_exact_volume(), muted);
}

fn show_brightness_hud_from_key(delta: i32) {
    let now = current_millis();
    if now.saturating_sub(LAST_SYSTEM_HUD_KEY_MS.swap(now, Ordering::SeqCst)) < 45 {
        return;
    }
    let value = crate::widgets::get_exact_brightness()
        .map(|brightness| (brightness + delta).clamp(0, 100))
        .unwrap_or_else(|| if delta < 0 { 0 } else { 100 });
    show_system_hud("brightness", value, false);
}

fn start_brightness_hud_watcher() {
    if BRIGHTNESS_HUD_WATCHER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        let mut last_value = crate::widgets::get_exact_brightness();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(8));
            let current = crate::widgets::get_exact_brightness();
            if let (Some(prev), Some(now)) = (last_value, current) {
                if now != prev {
                    last_value = Some(now);
                    show_system_hud("brightness", now, false);
                    continue;
                }
            } else {
                last_value = current;
            }
        }
    });
}

/// Polls the system master volume every 550 ms.  If it changed since the
/// last poll the volume HUD is shown — regardless of which window has focus.
/// This mirrors `start_brightness_hud_watcher` and guarantees the HUD works
/// even when the Settings window (or any other Slint window) has swallowed
/// the WM_APPCOMMAND / WH_KEYBOARD_LL events.
fn start_volume_hud_watcher() {
    if VOLUME_HUD_WATCHER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        let mut last_value = crate::widgets::get_exact_volume();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(550));
            let current = crate::widgets::get_exact_volume();
            if current != last_value {
                last_value = current;
                // Debounce: skip if a key-based HUD fired in the last 600 ms
                let now = current_millis();
                if now.saturating_sub(LAST_SYSTEM_HUD_KEY_MS.load(Ordering::SeqCst)) >= 600 {
                    show_system_hud("volume", current, false);
                }
            }
        }
    });
}

unsafe extern "system" fn global_magic_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let msg = wparam.0 as u32;
        if msg == windows::Win32::UI::WindowsAndMessaging::WM_KEYDOWN 
            || msg == windows::Win32::UI::WindowsAndMessaging::WM_SYSKEYDOWN 
        {
            let hook_struct = *(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT);
            let vk = hook_struct.vkCode;

            match vk {
                0xAD => show_volume_hud_from_key(true),  // VK_VOLUME_MUTE
                0xAE => show_volume_hud_from_key(false), // VK_VOLUME_DOWN
                0xAF => show_volume_hud_from_key(false), // VK_VOLUME_UP
                0x6F => show_brightness_hud_from_key(-10),
                _ => {}
            }
            
            if matches!(vk, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0..=0xA5) {
                let ctrl_down = (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
                let alt_down = (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU.0 as i32) as u16 & 0x8000) != 0;
                let shift_down = (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
                let win_down = (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_LWIN.0 as i32) as u16 & 0x8000) != 0 
                    || (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_RWIN.0 as i32) as u16 & 0x8000) != 0;
                
                if ctrl_down && alt_down && shift_down && win_down {
                    let ids = {
                        let lock = MAGIC_HOTKEY_IDS.lock().unwrap();
                        lock.clone()
                    };
                    
                    if !ids.is_empty() {
                        let main_hwnd_val = PILL_HWND.load(std::sync::atomic::Ordering::SeqCst);
                        if main_hwnd_val != 0 {
                            let hwnd = windows::Win32::Foundation::HWND(main_hwnd_val as _);
                            for id in ids {
                                handle_hotkey(hwnd, id);
                            }
                        }
                    }
                }
            }
        }
    }
    windows::Win32::UI::WindowsAndMessaging::CallNextHookEx(None, code, wparam, lparam)
}

pub unsafe fn update_global_magic_hook(ids: Vec<i32>) {
    start_brightness_hud_watcher();
    start_volume_hud_watcher();

    let mut magic_ids_lock = MAGIC_HOTKEY_IDS.lock().unwrap();
    *magic_ids_lock = ids;
    
    let mut hook_lock = GLOBAL_MAGIC_HOOK.lock().unwrap();
    if hook_lock.is_none() {
        let h_mod = GetModuleHandleW(None).ok();
        let h_instance = h_mod.map(|m| windows::Win32::Foundation::HINSTANCE(m.0)).unwrap_or_default();
        let hook = windows::Win32::UI::WindowsAndMessaging::SetWindowsHookExW(
            windows::Win32::UI::WindowsAndMessaging::WH_KEYBOARD_LL,
            Some(global_magic_hook_proc),
            h_instance,
            0,
        ).ok();
        if hook.is_some() {
            println!("[HOTKEY] Registered global HUD/magic hook");
        } else {
            println!("[HOTKEY] Failed to register global HUD/magic hook");
        }
        *hook_lock = hook;
    }
}

pub unsafe fn register_raven_hotkeys(hwnd: HWND, settings: &crate::settings::RavenSettings) {
    unregister_raven_hotkeys(hwnd);
    
    let shortcuts = &settings.shortcuts;
    
    let list = [
        (HOTKEY_HOME, &shortcuts.tab_home),
        (HOTKEY_MEDIA, &shortcuts.tab_media),
        (HOTKEY_CALENDAR, &shortcuts.tab_calendar),
        (HOTKEY_CLOCK, &shortcuts.tab_clock),
        (HOTKEY_DROP, &shortcuts.tab_drop),
        (HOTKEY_STATS, &shortcuts.tab_stats),
        (HOTKEY_CAPTURE, &shortcuts.tab_capture),
        (HOTKEY_SETTINGS, &shortcuts.open_settings),
        (HOTKEY_CLIPBOARD, &shortcuts.clipboard_history),
        (HOTKEY_QUICK_SCREENSHOT, &shortcuts.quick_screenshot),
        (HOTKEY_TOGGLE_RAVEN, &shortcuts.toggle_raven),
        (HOTKEY_TOGGLE_FREEZE, &shortcuts.toggle_freeze),
        (HOTKEY_MEDIA_PLAY, &shortcuts.media_play),
        (HOTKEY_MEDIA_NEXT, &shortcuts.media_next),
        (HOTKEY_MEDIA_PREV, &shortcuts.media_prev),
        (HOTKEY_QUICK_RECORD_TOGGLE, &shortcuts.quick_record_toggle),
        (HOTKEY_RESTART_RAVEN, &shortcuts.restart_raven),
        (HOTKEY_QUIT_RAVEN, &shortcuts.quit_raven),
        (HOTKEY_TOPBAR_STATS, &shortcuts.topbar_stats),
        (HOTKEY_TOPBAR_VOLUME, &shortcuts.topbar_volume),
        (HOTKEY_TOPBAR_WIFI, &shortcuts.topbar_wifi),
        (HOTKEY_TOPBAR_TIMER, &shortcuts.topbar_timer),
        (HOTKEY_TOPBAR_CALENDAR, &shortcuts.topbar_calendar),
    ];
    
    let mut magic_ids = Vec::new();
    for (id, val) in list {
        if !val.is_empty() {
            if val == "✦" {
                magic_ids.push(id);
            } else if let Some((mods, vk)) = parse_shortcut(val) {
                let _ = RegisterHotKey(hwnd, id, HOT_KEY_MODIFIERS(mods), vk);
            }
        }
    }
    
    update_global_magic_hook(magic_ids);
}

pub unsafe fn unregister_raven_hotkeys(hwnd: HWND) {
    update_global_magic_hook(Vec::new());
    for id in [
        HOTKEY_HOME,
        HOTKEY_MEDIA,
        HOTKEY_CALENDAR,
        HOTKEY_CLOCK,
        HOTKEY_DROP,
        HOTKEY_NOTIFICATIONS,
        HOTKEY_CAPTURE,
        HOTKEY_STATS,
        HOTKEY_SETTINGS,
        HOTKEY_CLIPBOARD,
        HOTKEY_QUICK_SCREENSHOT,
        HOTKEY_TOGGLE_RAVEN,
        HOTKEY_TOGGLE_FREEZE,
        HOTKEY_MEDIA_PLAY,
        HOTKEY_MEDIA_NEXT,
        HOTKEY_MEDIA_PREV,
        HOTKEY_QUICK_RECORD_TOGGLE,
        HOTKEY_RESTART_RAVEN,
        HOTKEY_QUIT_RAVEN,
        HOTKEY_TOPBAR_STATS,
        HOTKEY_TOPBAR_VOLUME,
        HOTKEY_TOPBAR_WIFI,
        HOTKEY_TOPBAR_TIMER,
        HOTKEY_TOPBAR_CALENDAR,
    ] {
        let _ = UnregisterHotKey(hwnd, id);
    }
}

pub unsafe fn handle_hotkey(hwnd: HWND, id: i32) {
    println!("[HOTKEY] Triggered hotkey ID: {}", id);

    if id == HOTKEY_MEDIA_PLAY {
        windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(0xB3, 0, windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0), 0);
        windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(0xB3, 0, windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(2), 0);
        return;
    }

    if id == HOTKEY_MEDIA_NEXT {
        windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(0xB0, 0, windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0), 0);
        windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(0xB0, 0, windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(2), 0);
        return;
    }

    if id == HOTKEY_MEDIA_PREV {
        windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(0xB1, 0, windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0), 0);
        windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(0xB1, 0, windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(2), 0);
        return;
    }

    if id == HOTKEY_RESTART_RAVEN {
        println!("[HOTKEY] Restarting Raven Notch...");
        if let Ok(exe_path) = std::env::current_exe() {
            let _ = std::process::Command::new(exe_path).spawn();
            remove_tray_icon(hwnd);
            std::process::exit(0);
        }
        return;
    }

    if id == HOTKEY_QUIT_RAVEN {
        println!("[HOTKEY] Quitting Raven Notch...");
        remove_tray_icon(hwnd);
        std::process::exit(0);
    }

    // Support Slint UI dispatching for visually interactive hotkeys
    if let Some(weak_ui) = crate::window::PILL_UI_WEAK.get() {
        let weak_cloned = weak_ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak_cloned.upgrade() {
                match id {
                    HOTKEY_HOME => {
                        let is_exp = ui.get_is_expanded();
                        let cur_tab = ui.get_active_tab().to_string();
                        if is_exp && cur_tab == "home" {
                            HOTKEY_OPENED.store(false, Ordering::SeqCst);
                            ui.invoke_request_notch_close();
                        } else {
                            HOTKEY_OPENED.store(true, Ordering::SeqCst);
                            ui.set_active_tab("home".into());
                            ui.invoke_request_notch_open();
                        }
                    }
                    HOTKEY_MEDIA => {
                        let is_exp = ui.get_is_expanded();
                        let cur_tab = ui.get_active_tab().to_string();
                        if is_exp && cur_tab == "media" {
                            HOTKEY_OPENED.store(false, Ordering::SeqCst);
                            ui.invoke_request_notch_close();
                        } else {
                            HOTKEY_OPENED.store(true, Ordering::SeqCst);
                            ui.set_active_tab("media".into());
                            ui.invoke_request_notch_open();
                        }
                    }
                    HOTKEY_CLOCK => {
                        let is_exp = ui.get_is_expanded();
                        let cur_tab = ui.get_active_tab().to_string();
                        if is_exp && cur_tab == "clock" {
                            HOTKEY_OPENED.store(false, Ordering::SeqCst);
                            ui.invoke_request_notch_close();
                        } else {
                            HOTKEY_OPENED.store(true, Ordering::SeqCst);
                            ui.set_active_tab("clock".into());
                            ui.invoke_request_notch_open();
                        }
                    }
                    HOTKEY_DROP => {
                        let is_exp = ui.get_is_expanded();
                        let cur_tab = ui.get_active_tab().to_string();
                        if is_exp && cur_tab == "drop" {
                            HOTKEY_OPENED.store(false, Ordering::SeqCst);
                            ui.invoke_request_notch_close();
                        } else {
                            HOTKEY_OPENED.store(true, Ordering::SeqCst);
                            ui.set_active_tab("drop".into());
                            ui.invoke_request_notch_open();
                        }
                    }
                    HOTKEY_STATS => {
                        let is_exp = ui.get_is_expanded();
                        let cur_tab = ui.get_active_tab().to_string();
                        if is_exp && cur_tab == "stats" {
                            HOTKEY_OPENED.store(false, Ordering::SeqCst);
                            ui.invoke_request_notch_close();
                        } else {
                            HOTKEY_OPENED.store(true, Ordering::SeqCst);
                            ui.set_active_tab("stats".into());
                            ui.invoke_request_notch_open();
                        }
                    }
                    HOTKEY_SETTINGS => {
                        ui.invoke_open_settings();
                    }
                    HOTKEY_CLIPBOARD => {
                        ui.invoke_toggle_clipboard_dropdown_rust();
                    }
                    HOTKEY_QUICK_SCREENSHOT => {
                        ui.invoke_trigger_screenshot();
                    }
                    HOTKEY_TOGGLE_RAVEN => {
                        if ui.get_is_expanded() {
                            HOTKEY_OPENED.store(false, Ordering::SeqCst);
                            ui.invoke_request_notch_close();
                        } else {
                            HOTKEY_OPENED.store(true, Ordering::SeqCst);
                            ui.invoke_request_notch_open();
                        }
                    }
                    HOTKEY_TOGGLE_FREEZE => {
                        ui.invoke_toggle_freeze();
                    }
                    HOTKEY_TOPBAR_STATS => {
                        ui.invoke_toggle_topbar_stats_rust();
                    }
                    HOTKEY_TOPBAR_VOLUME => {
                        ui.invoke_toggle_volume_dropdown_rust();
                    }
                    HOTKEY_TOPBAR_WIFI => {
                        ui.invoke_toggle_wifi_dropdown_rust();
                    }
                    HOTKEY_TOPBAR_TIMER => {
                        ui.invoke_toggle_timer_dropdown_rust();
                    }
                    HOTKEY_TOPBAR_CALENDAR => {
                        ui.invoke_toggle_calendar_dropdown_rust();
                    }
                    _ => {}
                }
            }
        });
    }
}

unsafe fn apply_renderer_geometry(hwnd: HWND, state: &WindowState) {
    let geometry = state.renderer.borrow().geometry();
    let settings = state.settings.borrow();

    // The OS Slint window is FIXED at composition dimensions and never resized during animation.
    // We only update the visible notch size atomics so WM_NCHITTEST can compute click-through bounds.
    let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
    let scale_factor = if scale_bits > 0 {
        f32::from_bits(scale_bits)
    } else {
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
        if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 }
    };

    let vis_w_phys = (geometry.width * scale_factor).round() as i32;
    let vis_h_phys = (geometry.height * scale_factor).round() as i32;
    PILL_VIS_WIDTH_PHYS.store(vis_w_phys, Ordering::SeqCst);
    PILL_VIS_HEIGHT_PHYS.store(vis_h_phys, Ordering::SeqCst);

    // Note: position is maintained by the subclass WM_WINDOWPOSCHANGING interceptor.
    // A single SetWindowPos for position-only is fine (SWP_NOSIZE keeps composition size).
    let _ = settings; // suppress unused warning
}

fn run_native_action(state: &WindowState, action: NativeAction) {
    match action {
        NativeAction::MediaPrevious => state.services.media.previous(),
        NativeAction::MediaPlayPause => state.services.media.play_pause(),
        NativeAction::MediaNext => state.services.media.next(),
        NativeAction::TimerToggle => {
            let snapshot = state.services.toggle_timer();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::TimerReset => {
            let snapshot = state.services.reset_timer();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::StopwatchToggle => {
            let snapshot = state.services.toggle_stopwatch();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::StopwatchReset => {
            let snapshot = state.services.reset_stopwatch();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::ShelfOpenFirst => {
            let snapshot = state.services.open_first_shelf_item();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::ShelfRevealFirst => {
            let snapshot = state.services.reveal_first_shelf_item();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::ShelfClear => {
            let snapshot = state.services.clear_shelf();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::CaptureScreenshot => {
            let snapshot = state.services.capture_screenshot();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::CaptureRegion => {
            let snapshot = if let Some(region) = unsafe { select_capture_region() } {
                state.services.capture_region_rect(region)
            } else {
                state.services.refresh_capture()
            };
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::CaptureOpenLast => {
            let snapshot = state.services.open_last_capture();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::CaptureOpenFolder => {
            let snapshot = state.services.open_capture_folder();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::CalendarRefresh => {
            let snapshot = state.services.refresh_calendar();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::CaffeineToggle => {
            let snapshot = state.services.toggle_caffeine();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::VolumeDown => {
            let snapshot = state.services.volume_down();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            crate::window::show_system_hud("volume", crate::widgets::get_exact_volume(), false);
            return;
        }
        NativeAction::VolumeMute => {
            let snapshot = state.services.volume_mute();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            crate::window::show_system_hud("volume", crate::widgets::get_exact_volume(), true);
            return;
        }
        NativeAction::VolumeUp => {
            let snapshot = state.services.volume_up();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            crate::window::show_system_hud("volume", crate::widgets::get_exact_volume(), false);
            return;
        }
        NativeAction::BrightnessDown => {
            let hud_value = crate::widgets::get_exact_brightness()
                .map(|value| (value - 10).clamp(0, 100))
                .unwrap_or(0);
            let snapshot = state.services.brightness_down();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            crate::window::show_system_hud("brightness", hud_value, false);
            return;
        }
        NativeAction::BrightnessUp => {
            let hud_value = crate::widgets::get_exact_brightness()
                .map(|value| (value + 10).clamp(0, 100))
                .unwrap_or(100);
            let snapshot = state.services.brightness_up();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            crate::window::show_system_hud("brightness", hud_value, false);
            return;
        }
        NativeAction::SettingsOpenFile => {
            let snapshot = state.services.open_settings_file();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
        NativeAction::SettingsWidthDown => {
            apply_settings_update(state, crate::settings::adjust_number(
                &["appearance", "idle_width"],
                -10.0,
                80.0,
                420.0,
            ));
            return;
        }
        NativeAction::SettingsWidthUp => {
            apply_settings_update(state, crate::settings::adjust_number(
                &["appearance", "idle_width"],
                10.0,
                80.0,
                420.0,
            ));
            return;
        }
        NativeAction::SettingsOpacityDown => {
            apply_settings_update(state, crate::settings::adjust_number(
                &["appearance", "notch_opacity"],
                -5.0,
                20.0,
                100.0,
            ));
            return;
        }
        NativeAction::SettingsOpacityUp => {
            apply_settings_update(state, crate::settings::adjust_number(
                &["appearance", "notch_opacity"],
                5.0,
                20.0,
                100.0,
            ));
            return;
        }
        NativeAction::SettingsHoverToggle => {
            apply_settings_update(state, crate::settings::toggle_bool(&["hover", "enabled"]));
            return;
        }
        NativeAction::OpenNotificationSettings => {
            let snapshot = state.services.open_notification_settings();
            state.renderer.borrow_mut().set_snapshot(snapshot);
            return;
        }
    }

    let snapshot = state.services.refresh_media();
    state.renderer.borrow_mut().set_snapshot(snapshot);
}

fn apply_settings_update(state: &WindowState, settings: RavenSettings) {
    *state.settings.borrow_mut() = settings.clone();
    state.renderer.borrow_mut().set_settings(settings);
}

unsafe fn state<'a>(hwnd: HWND) -> Option<&'a WindowState> {
    let ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
    ptr.as_ref()
}

use std::sync::atomic::{AtomicI32, AtomicU32, AtomicIsize, Ordering, AtomicBool, AtomicU64};
pub static PILL_HWND: AtomicIsize = AtomicIsize::new(0);
pub static SETTINGS_HWND: AtomicIsize = AtomicIsize::new(0);
pub static STATS_WIDGET_HWND: AtomicIsize = AtomicIsize::new(0);
pub static HIDDEN_HWND: AtomicIsize = AtomicIsize::new(0);
pub static HOTKEY_OPENED: AtomicBool = AtomicBool::new(false);
/// Set to true while a system HUD (volume/brightness) is being displayed.
/// Prevents the hover detection loop from hiding the notch mid-display.
pub static HUD_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static PILL_IS_SPLIT_LAYOUT: AtomicBool = AtomicBool::new(false);
/// Set to true by the 50 ms hover-polling loop when a fullscreen / maximized
/// window is in the foreground and auto_hide_on_fullscreen is enabled.
/// Read by update_pill_window_layout() to position the window correctly.
pub static IS_FOREGROUND_FULLSCREEN: AtomicBool = AtomicBool::new(false);
pub static PILL_FULL_WIDTH_BAR: AtomicBool = AtomicBool::new(false);
pub static PILL_TOP_BAR_WIDGETS: AtomicBool = AtomicBool::new(true);
pub static PILL_SHOW_CALENDAR_DROPDOWN: AtomicBool = AtomicBool::new(false);
pub static PILL_SHOW_TIMER_DROPDOWN: AtomicBool = AtomicBool::new(false);
pub static PILL_SHOW_VOLUME_DROPDOWN: AtomicBool = AtomicBool::new(false);
pub static PILL_SHOW_RAVEN_MENU: AtomicBool = AtomicBool::new(false);
pub static PILL_SHOW_WIFI_DROPDOWN: AtomicBool = AtomicBool::new(false);
pub static PILL_SHOW_TOPBAR_STATS_DROPDOWN: AtomicBool = AtomicBool::new(false);
pub static PILL_SHOW_CLIPBOARD_DROPDOWN: AtomicBool = AtomicBool::new(false);
pub static APPBAR_REGISTRATION_ACTIVE: AtomicBool = AtomicBool::new(false);

pub static PILL_LOGICAL_OFFSET_X: AtomicI32 = AtomicI32::new(0);
pub static PILL_LOGICAL_OFFSET_Y: AtomicI32 = AtomicI32::new(0);
pub static PILL_LOGICAL_IDLE_HEIGHT: AtomicI32 = AtomicI32::new(38);
/// Physical pixel width of the *visible* notch/pill rectangle (inner rect, not OS window).
/// Written by apply_renderer_geometry, read by slint_window_subclass_proc for WM_NCHITTEST.
pub static PILL_VIS_WIDTH_PHYS: AtomicI32 = AtomicI32::new(0);
/// Physical pixel height of the *visible* notch/pill rectangle.
pub static PILL_VIS_HEIGHT_PHYS: AtomicI32 = AtomicI32::new(0);
/// Physical pixel Y offset of the *visible* notch/pill rectangle relative to window top.
pub static PILL_VIS_Y_PHYS: AtomicI32 = AtomicI32::new(0);
/// Physical pixel width of the fixed-size OS window (composition size).
pub static PILL_WIN_WIDTH_PHYS: AtomicI32 = AtomicI32::new(0);
/// Physical pixel height of the fixed-size OS window (composition size).
pub static PILL_WIN_HEIGHT_PHYS: AtomicI32 = AtomicI32::new(0);
/// Physical top-left X for the fixed-size Slint OS window.
pub static PILL_TARGET_X_PHYS: AtomicI32 = AtomicI32::new(i32::MIN);
/// Physical top-left Y for the fixed-size Slint OS window.
pub static PILL_TARGET_Y_PHYS: AtomicI32 = AtomicI32::new(i32::MIN);
/// Physical scale factor computed dynamically from Slint window scale factor.
pub static PILL_SCALE_FACTOR: AtomicU32 = AtomicU32::new(0);
pub static PILL_UI_WEAK: std::sync::OnceLock<slint::Weak<crate::Pill>> = std::sync::OnceLock::new();
pub static DROP_CALLBACK: std::sync::OnceLock<Box<dyn Fn(Vec<String>, f32) + Send + Sync>> = std::sync::OnceLock::new();
pub static SETTINGS_UI_WEAK: std::sync::OnceLock<slint::Weak<crate::SettingsWindow>> = std::sync::OnceLock::new();

pub static TAB_SWITCH_CALLBACK: std::sync::OnceLock<Box<dyn Fn(bool) + Send + Sync>> = std::sync::OnceLock::new();
pub static ACTIVE_TAB: std::sync::OnceLock<std::sync::Mutex<String>> = std::sync::OnceLock::new();
pub static TAB_VISIBILITY: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, bool>>> = std::sync::OnceLock::new();

static DRAG_START_X: AtomicI32 = AtomicI32::new(0);
static DRAG_START_Y: AtomicI32 = AtomicI32::new(0);
static IS_DRAGGING: AtomicBool = AtomicBool::new(false);

static LAST_SWITCH_TIME: AtomicU64 = AtomicU64::new(0);
static LAST_SCROLL_TIME: AtomicU64 = AtomicU64::new(0);
static SCROLL_ACCUMULATOR_X: AtomicI32 = AtomicI32::new(0);
static SCROLL_ACCUMULATOR_Y: AtomicI32 = AtomicI32::new(0);

pub fn show_system_hud(kind: &str, value: i32, muted: bool) {
    if kind == "volume" || kind == "brightness" {
        let settings = crate::settings::RavenSettings::load();
        let enabled = settings.raven_alert.enabled
            && if kind == "volume" {
                settings.raven_alert.monitor_volume_hud
            } else {
                settings.raven_alert.monitor_brightness_hud
            };
        if !enabled {
            return;
        }
    }

    let hud_kind = kind.to_string();
    let token = LAST_SWITCH_TIME.fetch_add(1, Ordering::SeqCst) + 1;
    // Signal the hover loop immediately (before entering event loop) so it
    // keeps is_hovered=true even while the mouse is away from the notch.
    HUD_ACTIVE.store(true, Ordering::SeqCst);

    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = PILL_UI_WEAK.get().and_then(|weak| weak.upgrade()) {
            HOTKEY_OPENED.store(true, Ordering::SeqCst);
            // 1. Resize/reposition the OS window to HUD height and center it first to prevent 1-frame coordinate lag/flashing
            update_pill_window_layout();

            // 2. Set the Slint HUD properties
            ui.set_system_hud_kind(hud_kind.clone().into());
            ui.set_system_hud_value(value);
            ui.set_system_hud_muted(muted);
            ui.set_content_opacity(1.0);
            ui.set_is_hovered(true);
            ui.invoke_request_notch_open();

            slint::Timer::single_shot(std::time::Duration::from_secs(3), move || {
                if LAST_SWITCH_TIME.load(Ordering::SeqCst) == token {
                    HUD_ACTIVE.store(false, Ordering::SeqCst);
                    if let Some(ui) = PILL_UI_WEAK.get().and_then(|weak| weak.upgrade()) {
                        HOTKEY_OPENED.store(false, Ordering::SeqCst);
                        // Restore motion dimensions to idle so the normal spring
                        // physics takes back over after the HUD dismisses.
                        let idle_w = ui.get_idle_width();
                        let idle_h = ui.get_idle_height();
                        ui.set_motion_width(idle_w);
                        ui.set_motion_height(idle_h);
                        ui.set_system_hud_kind("".into());
                        ui.set_is_hovered(false);
                        ui.invoke_request_notch_close();
                        update_pill_window_layout();
                    }
                }
            });
        }
    });
}


pub fn set_window_interactive_mode(interactive: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, SetForegroundWindow,
        GWL_EXSTYLE, WS_EX_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_FRAMECHANGED,
        SWP_SHOWWINDOW, SWP_NOACTIVATE,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows::Win32::Foundation::HWND;

    let hwnd_val = PILL_HWND.load(Ordering::SeqCst);
    if hwnd_val != 0 {
        let hwnd = HWND(hwnd_val);
        unsafe {
            let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let original = ex_style;
            if interactive {
                ex_style &= !WS_EX_NOACTIVATE.0;
            } else {
                ex_style |= WS_EX_NOACTIVATE.0;
            }
            if ex_style != original {
                let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style as isize);
                let _ = SetWindowPos(
                    hwnd,
                    HWND(0),
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | if interactive { SWP_SHOWWINDOW } else { SWP_NOACTIVATE },
                );
            }
            if interactive {
                let _ = SetForegroundWindow(hwnd);
                let _ = SetFocus(hwnd);
            }
        }
    }
}

pub fn set_active_tab(tab: String) {
    if let Some(m) = ACTIVE_TAB.get() {
        if let Ok(mut active) = m.lock() {
            *active = tab;
        }
    } else {
        let _ = ACTIVE_TAB.set(std::sync::Mutex::new(tab));
    }
}

pub fn get_active_tab() -> String {
    ACTIVE_TAB.get()
        .and_then(|m| m.lock().ok())
        .map(|s| s.clone())
        .unwrap_or_else(|| "home".to_string())
}

pub fn set_tab_visibility(tab: String, visible: bool) {
    if let Some(m) = TAB_VISIBILITY.get() {
        if let Ok(mut map) = m.lock() {
            map.insert(tab, visible);
        }
    } else {
        let mut map = std::collections::HashMap::new();
        map.insert(tab, visible);
        let _ = TAB_VISIBILITY.set(std::sync::Mutex::new(map));
    }
}

pub fn is_tab_visible(tab: &str) -> bool {
    TAB_VISIBILITY.get()
        .and_then(|m| m.lock().ok())
        .and_then(|map| map.get(tab).copied())
        .unwrap_or(true)
}
pub unsafe fn remove_from_taskbar(hwnd: HWND) {
    let list: Result<windows::Win32::UI::Shell::ITaskbarList, _> = windows::Win32::System::Com::CoCreateInstance(
        &windows::Win32::UI::Shell::TaskbarList,
        None,
        windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
    );
    if let Ok(list) = list {
        let _ = list.HrInit();
        let _ = list.DeleteTab(hwnd);
    }
}


pub unsafe fn setup_settings_window_subclass(hwnd: HWND, ui: slint::Weak<crate::SettingsWindow>) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    SETTINGS_HWND.store(hwnd.0, Ordering::SeqCst);
    let _ = SETTINGS_UI_WEAK.set(ui);
    
    // Explicitly clear any owner to make sure it's a standalone window in taskbar
    let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0);

    // Set the official icon for taskbar and window
    let hicon = load_app_icon();
    if !hicon.is_invalid() {
        let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(hicon.0));
        let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_SMALL as usize), LPARAM(hicon.0));
    }

    // Force style recalculation and taskbar update
    let _ = SetWindowPos(
        hwnd,
        HWND(0),
        0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
    );

    let _ = windows::Win32::UI::Shell::SetWindowSubclass(
        hwnd,
        Some(settings_window_subclass_proc),
        8484,
        0,
    );
}

pub unsafe extern "system" fn settings_window_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    _dwrefdata: usize,
) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::UI::Shell::DefSubclassProc;

    // ── Size / position lifecycle logging ────────────────────────────────
    if msg == WM_SIZE || msg == WM_WINDOWPOSCHANGED {
        let start_total = std::time::Instant::now();
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let win_w = rect.right - rect.left;
        let win_h = rect.bottom - rect.top;

        let mut client_rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut client_rect);
        let client_w = client_rect.right - client_rect.left;
        let client_h = client_rect.bottom - client_rect.top;

        let mut layout_w = 0.0_f32;
        let mut layout_h = 0.0_f32;
        let mut content_w = 0.0_f32;
        let mut content_h = 0.0_f32;
        let mut scroll_viewport_w = 0.0_f32;
        let mut scroll_viewport_h = 0.0_f32;

        let start_query = std::time::Instant::now();
        if let Some(weak_ui) = SETTINGS_UI_WEAK.get() {
            if let Some(ui) = weak_ui.upgrade() {
                layout_w = ui.get_root_layout_width();
                layout_h = ui.get_root_layout_height();
                content_w = ui.get_content_width();
                content_h = ui.get_content_height();
                scroll_viewport_w = ui.get_scroll_viewport_width();
                scroll_viewport_h = ui.get_scroll_viewport_height();
            }
        }
        let query_elapsed = start_query.elapsed();
        let total_elapsed = start_total.elapsed();

        let log_msg = format!(
            "[SETTINGS-LIFECYCLE] Msg: {} | Win32: {}x{} pos({},{}) | Client: {}x{} | RootLayout: {}x{} | Content: {}x{} | Scroll: {}x{} | Q: {}ms Total: {}ms\n",
            if msg == WM_SIZE { "WM_SIZE" } else { "WM_WINDOWPOSCHANGED" },
            win_w, win_h, rect.left, rect.top, client_w, client_h,
            layout_w, layout_h, content_w, content_h, scroll_viewport_w, scroll_viewport_h,
            query_elapsed.as_millis(),
            total_elapsed.as_millis()
        );
        crate::diagnostics::log("SETTINGS-LIFECYCLE", log_msg.trim_end());
    }

    // ── Volume/Mute keys while Settings window is foreground ─────────────
    // When Settings has focus, Windows routes WM_APPCOMMAND here instead of
    // to the Pill window, so Volume HUD would not show. Forward it manually.
    if msg == 0x0319 /* WM_APPCOMMAND */ {
        let command = ((lparam.0 >> 16) & 0x7fff) as i32;
        match command {
            8  => show_volume_hud_from_key(true),  // APPCOMMAND_VOLUME_MUTE
            9  => show_volume_hud_from_key(false), // APPCOMMAND_VOLUME_DOWN
            10 => show_volume_hud_from_key(false), // APPCOMMAND_VOLUME_UP
            _ => {}
        }
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}


const HWND_TOPMOST_RAW: isize = -1;

pub fn store_slint_target_rect(x: i32, y: i32, w: i32, h: i32) {
    PILL_TARGET_X_PHYS.store(x, Ordering::SeqCst);
    PILL_TARGET_Y_PHYS.store(y, Ordering::SeqCst);
    PILL_WIN_WIDTH_PHYS.store(w, Ordering::SeqCst);
    PILL_WIN_HEIGHT_PHYS.store(h, Ordering::SeqCst);
}

pub fn store_slint_scale_factor(scale: f32) {
    PILL_SCALE_FACTOR.store(scale.to_bits(), Ordering::SeqCst);
}

fn load_slint_target_rect() -> Option<(i32, i32, i32, i32)> {
    let x = PILL_TARGET_X_PHYS.load(Ordering::SeqCst);
    let y = PILL_TARGET_Y_PHYS.load(Ordering::SeqCst);
    let w = PILL_WIN_WIDTH_PHYS.load(Ordering::SeqCst);
    let h = PILL_WIN_HEIGHT_PHYS.load(Ordering::SeqCst);

    if x == i32::MIN || y == i32::MIN || w <= 0 || h <= 0 {
        None
    } else {
        Some((x, y, w, h))
    }
}

pub unsafe fn setup_drag_drop_subclass_recursive(hwnd: HWND) {
    enable_drag_accept_recursive(hwnd);
    
    // Set subclass on parent
    let _ = windows::Win32::UI::Shell::SetWindowSubclass(
        hwnd,
        Some(slint_window_subclass_proc),
        4242,
        0,
    );
    
    // Enumerate children and set subclass
    let _ = windows::Win32::UI::WindowsAndMessaging::EnumChildWindows(
        hwnd,
        Some(enum_child_subclass),
        LPARAM(0),
    );
}

unsafe extern "system" fn enum_child_subclass(child_hwnd: HWND, _lparam: LPARAM) -> windows::Win32::Foundation::BOOL {
    let _ = windows::Win32::UI::Shell::SetWindowSubclass(
        child_hwnd,
        Some(slint_window_subclass_proc),
        4242,
        0,
    );
    windows::Win32::Foundation::BOOL(1)
}

unsafe fn enable_drag_accept_recursive(hwnd: HWND) {
    windows::Win32::UI::Shell::DragAcceptFiles(hwnd, true);
    
    let _ = windows::Win32::UI::WindowsAndMessaging::ChangeWindowMessageFilterEx(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::WM_DROPFILES,
        windows::Win32::UI::WindowsAndMessaging::MSGFLT_ALLOW,
        None,
    );
    let _ = windows::Win32::UI::WindowsAndMessaging::ChangeWindowMessageFilterEx(
        hwnd,
        0x0049, // WM_COPYGLOBALDATA
        windows::Win32::UI::WindowsAndMessaging::MSGFLT_ALLOW,
        None,
    );

    // Enumerate children and enable it there too
    let _ = windows::Win32::UI::WindowsAndMessaging::EnumChildWindows(
        hwnd,
        Some(enum_child_enable_drag),
        LPARAM(0),
    );
}

unsafe extern "system" fn enum_child_enable_drag(child_hwnd: HWND, _lparam: LPARAM) -> windows::Win32::Foundation::BOOL {
    windows::Win32::UI::Shell::DragAcceptFiles(child_hwnd, true);
    let _ = windows::Win32::UI::WindowsAndMessaging::ChangeWindowMessageFilterEx(
        child_hwnd,
        windows::Win32::UI::WindowsAndMessaging::WM_DROPFILES,
        windows::Win32::UI::WindowsAndMessaging::MSGFLT_ALLOW,
        None,
    );
    let _ = windows::Win32::UI::WindowsAndMessaging::ChangeWindowMessageFilterEx(
        child_hwnd,
        0x0049, // WM_COPYGLOBALDATA
        windows::Win32::UI::WindowsAndMessaging::MSGFLT_ALLOW,
        None,
    );
    windows::Win32::Foundation::BOOL(1)
}

pub unsafe fn apply_appbar_reservation(hwnd: HWND, enabled: bool, height: u32) -> Result<(), String> {
    let log = |msg: &str| {
        crate::diagnostics::log("APPBAR", msg);
    };

    log(&format!("apply_appbar_reservation called: enabled={}, height={}", enabled, height));

    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: 0,
        uEdge: ABE_TOP,
        rc: RECT::default(),
        lParam: LPARAM(0),
    };

    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowLongPtrW, GetWindowLongPtrW, GWLP_HWNDPARENT, GWL_EXSTYLE, GWL_STYLE,
        SetWindowPos, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_FRAMECHANGED,
        WS_EX_TOOLWINDOW,
    };
    use std::sync::atomic::Ordering;
    
    let hidden_hwnd = HIDDEN_HWND.load(Ordering::SeqCst);
    let current_ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;

    if !enabled {
        // First remove the AppBar registration
        let remove_res = SHAppBarMessage(ABM_REMOVE, &mut data);
        log(&format!("SHAppBarMessage ABM_REMOVE returned: {}", remove_res));

        // Restore owner and WS_EX_TOOLWINDOW
        if hidden_hwnd != 0 {
            let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, hidden_hwnd);
        }
        let restored_ex = current_ex_style | WS_EX_TOOLWINDOW.0;
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, restored_ex as isize);
        let _ = SetWindowPos(hwnd, HWND(0), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);
        log("AppBar removed, owner and TOOLWINDOW styles restored.");
        return Ok(());
    }

    // Always unregister first to avoid duplicate registration failure
    let pre_remove_res = SHAppBarMessage(ABM_REMOVE, &mut data);
    log(&format!("Pre-emptive ABM_REMOVE returned: {}", pre_remove_res));

    let current_style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    log(&format!("Before AppBar style adjustments: Style=0x{:X}, ExStyle=0x{:X}, Owner=0x{:X}", current_style, current_ex_style, GetWindowLongPtrW(hwnd, GWLP_HWNDPARENT) as usize));

    // AppBars cannot be owned or have WS_EX_TOOLWINDOW.
    // Remove the owner window and TOOLWINDOW style before registering.
    let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0);
    let stripped_ex = current_ex_style & !WS_EX_TOOLWINDOW.0;
    let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, stripped_ex as isize);
    let _ = SetWindowPos(hwnd, HWND(0), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);

    let post_style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let post_ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let post_owner = GetWindowLongPtrW(hwnd, GWLP_HWNDPARENT);
    log(&format!("After AppBar style adjustments: Style=0x{:X}, ExStyle=0x{:X}, Owner=0x{:X}", post_style, post_ex_style, post_owner as usize));

    data.uCallbackMessage = 0x0400 + 101; // WM_USER + 101

    let monitor = windows::Win32::Graphics::Gdi::MonitorFromWindow(
        hwnd,
        windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
    );
    let mut info = windows::Win32::Graphics::Gdi::MONITORINFO {
        cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
        ..Default::default()
    };
    if !windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info).as_bool() {
        // Restore styles before returning error
        if hidden_hwnd != 0 {
            let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, hidden_hwnd);
        }
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current_ex_style as isize);
        let _ = SetWindowPos(hwnd, HWND(0), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);
        log("Failed to get monitor info.");
        return Err("Failed to get monitor info".to_string());
    }

    let screen_w = info.rcMonitor.right - info.rcMonitor.left;
    let bar_height = height.clamp(1, 300) as i32;

    data.rc = RECT {
        left: info.rcMonitor.left,
        top: info.rcMonitor.top,
        right: info.rcMonitor.left + screen_w,
        bottom: info.rcMonitor.top + bar_height,
    };

    let new_res = SHAppBarMessage(ABM_NEW, &mut data);
    log(&format!("SHAppBarMessage ABM_NEW returned: {}", new_res));
    if new_res == 0 {
        // Restore owner and TOOLWINDOW style
        if hidden_hwnd != 0 {
            let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, hidden_hwnd);
        }
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current_ex_style as isize);
        let _ = SetWindowPos(hwnd, HWND(0), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);
        log("Windows rejected the appbar registration.");
        return Err("Windows rejected the appbar registration".to_string());
    }

    let query_res = SHAppBarMessage(ABM_QUERYPOS, &mut data);
    log(&format!("SHAppBarMessage ABM_QUERYPOS returned: {}, adjusted rc: {:?}", query_res, data.rc));

    data.rc.top = info.rcMonitor.top;
    data.rc.bottom = info.rcMonitor.top + bar_height;
    
    let setpos_res = SHAppBarMessage(ABM_SETPOS, &mut data);
    log(&format!("SHAppBarMessage ABM_SETPOS returned: {}, final rc: {:?}", setpos_res, data.rc));

    log(&format!("AppBar registered successfully at height={}.", bar_height));
    Ok(())
}

fn resolve_appbar_reserve_height(settings_height: u32) -> u32 {
    let scale_bits = PILL_SCALE_FACTOR.load(std::sync::atomic::Ordering::SeqCst);
    let scale = if scale_bits > 0 {
        f32::from_bits(scale_bits).max(0.5)
    } else {
        1.0
    };

    let ui_idle_height = PILL_UI_WEAK
        .get()
        .and_then(|weak| weak.upgrade())
        .map(|ui| ui.get_idle_height());
    let logical_idle_height = ui_idle_height.unwrap_or_else(|| {
        let stored = PILL_LOGICAL_IDLE_HEIGHT.load(std::sync::atomic::Ordering::SeqCst);
        if stored > 0 {
            stored as f32
        } else {
            settings_height as f32
        }
    });

    let measured_height = (logical_idle_height.max(10.0) * scale).round() as u32;
    if measured_height > 0 {
        measured_height.clamp(1, 300)
    } else {
        settings_height.clamp(1, 300)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct AppbarReservationState {
    hwnd: isize,
    enabled: bool,
    height: u32,
}

static APPBAR_RESERVATION_STATE: std::sync::OnceLock<std::sync::Mutex<Option<AppbarReservationState>>> =
    std::sync::OnceLock::new();

pub fn update_appbar_reservation() {
    let hwnd_val = PILL_HWND.load(std::sync::atomic::Ordering::SeqCst);
    if hwnd_val == 0 {
        return;
    }
    let hwnd = HWND(hwnd_val as _);
    let settings = crate::settings::RavenSettings::load();
    let effective_enabled = settings.advanced.reserve_top_area
        && !settings.appearance.auto_hide
        && !settings.appearance.auto_hide_on_fullscreen;
    if settings.advanced.reserve_top_area && !effective_enabled {
        crate::diagnostics::log(
            "APPBAR",
            "reserve_top_area ignored because auto-hide/fullscreen auto-hide is enabled",
        );
    }
    let reserve_height = resolve_appbar_reserve_height(settings.advanced.reserve_top_height);
    let desired_state = AppbarReservationState {
        hwnd: hwnd_val,
        enabled: effective_enabled,
        height: reserve_height,
    };

    let state_lock = APPBAR_RESERVATION_STATE.get_or_init(|| std::sync::Mutex::new(None));
    if let Ok(guard) = state_lock.lock() {
        if guard.as_ref().copied() == Some(desired_state) {
            return;
        }
    }

    unsafe {
        let res = apply_appbar_reservation(
            hwnd,
            effective_enabled,
            reserve_height,
        );
        if let Err(e) = res {
            if let Ok(mut guard) = state_lock.lock() {
                *guard = None;
            }
            println!("[APPBAR-ERROR] apply_appbar_reservation failed: {}", e);
        } else {
            if let Ok(mut guard) = state_lock.lock() {
                *guard = Some(desired_state);
            }
            println!("[APPBAR] apply_appbar_reservation succeeded!");
        }
    }
}

pub fn get_open_apps() -> Vec<(HWND, String)> {
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, IsWindowVisible, GetWindow, GetWindowTextW, GetClassNameW, GetWindowLongW, GetWindowThreadProcessId, GW_OWNER, GWL_EXSTYLE, WS_EX_TOOLWINDOW, WS_EX_APPWINDOW};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::Foundation::{HWND, LPARAM, BOOL};

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }

        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        );
        if hr.is_ok() && cloaked != 0 {
            return true.into();
        }

        let current_pid = windows::Win32::System::Threading::GetCurrentProcessId();
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == current_pid {
            return true.into();
        }

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 && (ex_style & WS_EX_APPWINDOW.0) == 0 {
            return true.into();
        }

        let owner = GetWindow(hwnd, GW_OWNER);
        if owner != HWND(0) && (ex_style & WS_EX_APPWINDOW.0) == 0 {
            return true.into();
        }

        let mut text = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut text);
        if len == 0 {
            return true.into();
        }
        let title = String::from_utf16_lossy(&text[..len as usize]);

        let mut class_name_buf = [0u16; 256];
        let class_len = GetClassNameW(hwnd, &mut class_name_buf);
        let class_name = String::from_utf16_lossy(&class_name_buf[..class_len as usize]);
        if class_name == "Progman" || class_name == "WorkerW" || class_name == "Shell_TrayWnd" {
            return true.into();
        }

        let list = &mut *(lparam.0 as *mut Vec<(HWND, String)>);
        list.push((hwnd, title));

        true.into()
    }

    let mut list = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

pub fn get_file_icon(path: &str) -> Option<slint::Image> {
    use windows::Win32::UI::WindowsAndMessaging::{PrivateExtractIconsW, HICON, LR_DEFAULTCOLOR, DestroyIcon};
    let path_wide = wide(path);
    let mut filename_arr = [0u16; 260];
    let len = path_wide.len().min(260);
    filename_arr[..len].copy_from_slice(&path_wide[..len]);
    
    let mut phicon = [HICON(0)];
    let mut piconid = 0u32;
    unsafe {
        let count = PrivateExtractIconsW(
            &filename_arr,
            0,
            24,
            24,
            Some(&mut phicon),
            Some(&mut piconid as *mut u32),
            LR_DEFAULTCOLOR.0,
        );
        if count > 0 && !phicon[0].is_invalid() {
            let img = hicon_to_slint_image(phicon[0]);
            let _ = DestroyIcon(phicon[0]);
            img
        } else {
            None
        }
    }
}

pub fn get_window_icon(hwnd: HWND) -> Option<slint::Image> {
    use windows::Win32::UI::WindowsAndMessaging::{
        SendMessageW, GetClassLongPtrW, PrivateExtractIconsW, HICON, WM_GETICON, ICON_SMALL, ICON_SMALL2, ICON_BIG,
        GCLP_HICONSM, GCLP_HICON, LR_DEFAULTCOLOR, DestroyIcon, GetWindowThreadProcessId,
    };
    use windows::Win32::Foundation::{WPARAM, LPARAM};

    unsafe {
        let mut hicon = HICON(SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_SMALL2 as usize), LPARAM(0)).0 as _);
        if hicon.is_invalid() {
            hicon = HICON(SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_SMALL as usize), LPARAM(0)).0 as _);
        }
        if hicon.is_invalid() {
            hicon = HICON(SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_BIG as usize), LPARAM(0)).0 as _);
        }

        if hicon.is_invalid() {
            hicon = HICON(GetClassLongPtrW(hwnd, GCLP_HICONSM) as _);
        }
        if hicon.is_invalid() {
            hicon = HICON(GetClassLongPtrW(hwnd, GCLP_HICON) as _);
        }

        let mut needs_destroy = false;
        if hicon.is_invalid() {
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid != 0 {
                use windows::Win32::System::Threading::*;
                if let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                    let mut exe_path = [0u16; 260];
                    let mut size = exe_path.len() as u32;
                    if QueryFullProcessImageNameW(process, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(exe_path.as_mut_ptr()), &mut size).is_ok() {
                        let mut phicon = [HICON(0)];
                        let mut piconid = 0u32;
                        let count = PrivateExtractIconsW(
                            &exe_path,
                            0,
                            24,
                            24,
                            Some(&mut phicon),
                            Some(&mut piconid as *mut u32),
                            LR_DEFAULTCOLOR.0,
                        );
                        if count > 0 && !phicon[0].is_invalid() {
                            hicon = phicon[0];
                            needs_destroy = true;
                        }
                    }
                }
            }
        }

        if hicon.is_invalid() {
            return None;
        }

        let img = hicon_to_slint_image(hicon);
        if needs_destroy {
            let _ = DestroyIcon(hicon);
        }
        img
    }
}

pub fn get_window_icon_raw(hwnd: HWND) -> Option<(u32, u32, Vec<u8>)> {
    use windows::Win32::UI::WindowsAndMessaging::{
        SendMessageW, GetClassLongPtrW, PrivateExtractIconsW, HICON, WM_GETICON, ICON_SMALL, ICON_SMALL2, ICON_BIG,
        GCLP_HICONSM, GCLP_HICON, LR_DEFAULTCOLOR, DestroyIcon, GetWindowThreadProcessId,
    };
    use windows::Win32::Foundation::{WPARAM, LPARAM};

    unsafe {
        let mut hicon = HICON(SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_SMALL2 as usize), LPARAM(0)).0 as _);
        if hicon.is_invalid() {
            hicon = HICON(SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_SMALL as usize), LPARAM(0)).0 as _);
        }
        if hicon.is_invalid() {
            hicon = HICON(SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_BIG as usize), LPARAM(0)).0 as _);
        }

        if hicon.is_invalid() {
            hicon = HICON(GetClassLongPtrW(hwnd, GCLP_HICONSM) as _);
        }
        if hicon.is_invalid() {
            hicon = HICON(GetClassLongPtrW(hwnd, GCLP_HICON) as _);
        }

        let mut needs_destroy = false;
        if hicon.is_invalid() {
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid != 0 {
                use windows::Win32::System::Threading::*;
                if let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                    let mut exe_path = [0u16; 260];
                    let mut size = exe_path.len() as u32;
                    if QueryFullProcessImageNameW(process, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(exe_path.as_mut_ptr()), &mut size).is_ok() {
                        let mut phicon = [HICON(0)];
                        let mut piconid = 0u32;
                        let count = PrivateExtractIconsW(
                            &exe_path,
                            0,
                            24,
                            24,
                            Some(&mut phicon),
                            Some(&mut piconid as *mut u32),
                            LR_DEFAULTCOLOR.0,
                        );
                        if count > 0 && !phicon[0].is_invalid() {
                            hicon = phicon[0];
                            needs_destroy = true;
                        }
                    }
                }
            }
        }

        if hicon.is_invalid() {
            return None;
        }

        let raw = hicon_to_raw_pixels(hicon);
        if needs_destroy {
            let _ = DestroyIcon(hicon);
        }
        raw
    }
}

unsafe fn hicon_to_raw_pixels(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<(u32, u32, Vec<u8>)> {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
    use windows::Win32::Foundation::HWND;

    let mut iconinfo = ICONINFO::default();
    if GetIconInfo(hicon, &mut iconinfo).is_err() {
        return None;
    }

    let _color_guard = BitmapGuard(iconinfo.hbmColor);
    let _mask_guard = BitmapGuard(iconinfo.hbmMask);

    let hdc = GetDC(HWND(0));
    let mem_dc = CreateCompatibleDC(hdc);
    let _dc_guard = DcGuard(mem_dc);

    let mut bmp = BITMAP::default();
    if GetObjectW(
        windows::Win32::Graphics::Gdi::HGDIOBJ(iconinfo.hbmColor.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bmp as *mut _ as *mut _),
    ) == 0 {
        return None;
    }

    let width = bmp.bmWidth;
    let height = bmp.bmHeight;

    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -height;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let prev = SelectObject(mem_dc, iconinfo.hbmColor);
    
    let res = GetDIBits(
        hdc,
        iconinfo.hbmColor,
        0,
        height as u32,
        Some(buffer.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );

    SelectObject(mem_dc, prev);
    ReleaseDC(HWND(0), hdc);

    if res == 0 {
        return None;
    }

    for chunk in buffer.chunks_exact_mut(4) {
        let b = chunk[0];
        let r = chunk[2];
        chunk[0] = r;
        chunk[2] = b;
    }

    Some((width as u32, height as u32, buffer))
}

struct BitmapGuard(windows::Win32::Graphics::Gdi::HBITMAP);
impl Drop for BitmapGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { windows::Win32::Graphics::Gdi::DeleteObject(self.0); }
        }
    }
}

struct DcGuard(windows::Win32::Graphics::Gdi::HDC);
impl Drop for DcGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { windows::Win32::Graphics::Gdi::DeleteDC(self.0); }
        }
    }
}

unsafe fn hicon_to_slint_image(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<slint::Image> {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
    use windows::Win32::Foundation::HWND;

    let mut iconinfo = ICONINFO::default();
    if GetIconInfo(hicon, &mut iconinfo).is_err() {
        return None;
    }

    let _color_guard = BitmapGuard(iconinfo.hbmColor);
    let _mask_guard = BitmapGuard(iconinfo.hbmMask);

    let hdc = GetDC(HWND(0));
    let mem_dc = CreateCompatibleDC(hdc);
    let _dc_guard = DcGuard(mem_dc);

    let mut bmp = BITMAP::default();
    if GetObjectW(
        windows::Win32::Graphics::Gdi::HGDIOBJ(iconinfo.hbmColor.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bmp as *mut _ as *mut _),
    ) == 0 {
        return None;
    }

    let width = bmp.bmWidth;
    let height = bmp.bmHeight;

    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -height;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let prev = SelectObject(mem_dc, iconinfo.hbmColor);
    
    let res = GetDIBits(
        hdc,
        iconinfo.hbmColor,
        0,
        height as u32,
        Some(buffer.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );

    SelectObject(mem_dc, prev);
    ReleaseDC(HWND(0), hdc);

    if res == 0 {
        return None;
    }

    for chunk in buffer.chunks_exact_mut(4) {
        let b = chunk[0];
        let r = chunk[2];
        chunk[0] = r;
        chunk[2] = b;
    }

    let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width as u32, height as u32);
    pixel_buffer.make_mut_slice().copy_from_slice(bytemuck_cast_or_convert(&buffer));
    
    Some(slint::Image::from_rgba8(pixel_buffer))
}

fn bytemuck_cast_or_convert(bytes: &[u8]) -> &[slint::Rgba8Pixel] {
    unsafe {
        std::slice::from_raw_parts(
            bytes.as_ptr() as *const slint::Rgba8Pixel,
            bytes.len() / 4
        )
    }
}

pub fn activate_window(hwnd_val: isize) {
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, ShowWindow, SetForegroundWindow, SW_RESTORE, SW_MINIMIZE, GetForegroundWindow};
    use windows::Win32::Foundation::HWND;

    let hwnd = HWND(hwnd_val);
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.0 == hwnd.0 {
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
        } else {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// Called whenever `full_width_bar` or `reserve_top_area` settings change.
/// Updates the atomics used for hit-testing and repositions/resizes the pill OS window.
/// Returns the primary monitor screen width in logical pixels (physical / scale),
/// and the DPI scale factor. Used to push `screen-width` into the Slint pill window.
pub fn get_primary_screen_logical_width() -> (f32, f32) {
    let hwnd_val = PILL_HWND.load(Ordering::SeqCst);
    let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
    let scale = if scale_bits > 0 {
        f32::from_bits(scale_bits)
    } else {
        1.0
    };

    if hwnd_val == 0 {
        // Fallback: use GetSystemMetrics
        let phys_w = unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN
            )
        };
        return (phys_w as f32 / scale, scale);
    }

    let hwnd = HWND(hwnd_val as _);
    unsafe {
        let monitor = windows::Win32::Graphics::Gdi::MonitorFromWindow(
            hwnd,
            windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
        );
        let mut info = windows::Win32::Graphics::Gdi::MONITORINFO {
            cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info);
        let phys_w = (info.rcMonitor.right - info.rcMonitor.left) as f32;
        (phys_w / scale, scale)
    }
}

pub fn update_pill_window_layout() {
    let hwnd_val = PILL_HWND.load(Ordering::SeqCst);
    if hwnd_val == 0 {
        return;
    }
    let hwnd = HWND(hwnd_val as _);
    let settings = crate::settings::RavenSettings::load();
    let layout_full_width = settings.advanced.full_width_bar;
    let full_width_bar_enabled = settings.advanced.full_width_bar;
    let top_bar_widgets = settings.advanced.full_width_bar && settings.advanced.top_bar_widgets;

    // Update atomics used by WM_NCHITTEST hit-testing
    PILL_FULL_WIDTH_BAR.store(full_width_bar_enabled, Ordering::SeqCst);
    PILL_TOP_BAR_WIDGETS.store(top_bar_widgets, Ordering::SeqCst);

    // Reposition/resize the OS-level window
    let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
    let scale = if scale_bits > 0 {
        f32::from_bits(scale_bits)
    } else {
        1.0
    };

    unsafe {
        let monitor = windows::Win32::Graphics::Gdi::MonitorFromWindow(
            hwnd,
            windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
        );
        let mut info = windows::Win32::Graphics::Gdi::MONITORINFO {
            cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info);
        let screen_width = info.rcMonitor.right - info.rcMonitor.left;

        let logical_offset_x = PILL_LOGICAL_OFFSET_X.load(Ordering::SeqCst) as f32;
        let logical_offset_y = PILL_LOGICAL_OFFSET_Y.load(Ordering::SeqCst) as f32;

        // Use persisted composition dimensions for non-full-width mode
        let logical_idle_w = (settings.appearance.idle_width).max(10.0);
        let logical_idle_h = (settings.appearance.idle_height).max(10.0);
        PILL_LOGICAL_IDLE_HEIGHT.store(logical_idle_h as i32, Ordering::SeqCst);
        if settings.advanced.reserve_top_area
            && !settings.appearance.auto_hide
            && !settings.appearance.auto_hide_on_fullscreen
        {
            update_appbar_reservation();
        }

        let comp_w = if layout_full_width {
            screen_width
        } else {
            ((f32::max(720.0, logical_idle_w) + 24.0) * scale).round() as i32
        };
        let (show_cal, show_timer, show_vol, show_raven, show_wifi, show_stats, show_clip) = if let Some(ui) = PILL_UI_WEAK.get().and_then(|w| w.upgrade()) {
            let cal = ui.get_show_calendar_dropdown();
            let timer = ui.get_show_timer_dropdown();
            let vol = ui.get_show_volume_dropdown();
            let raven = ui.get_show_raven_menu();
            let wifi = ui.get_show_wifi_dropdown();
            let stats = ui.get_show_topbar_stats_dropdown();
            let clip = ui.get_show_clipboard_dropdown();
            PILL_SHOW_CALENDAR_DROPDOWN.store(cal, Ordering::SeqCst);
            PILL_SHOW_TIMER_DROPDOWN.store(timer, Ordering::SeqCst);
            PILL_SHOW_VOLUME_DROPDOWN.store(vol, Ordering::SeqCst);
            PILL_SHOW_RAVEN_MENU.store(raven, Ordering::SeqCst);
            PILL_SHOW_WIFI_DROPDOWN.store(wifi, Ordering::SeqCst);
            PILL_SHOW_TOPBAR_STATS_DROPDOWN.store(stats, Ordering::SeqCst);
            PILL_SHOW_CLIPBOARD_DROPDOWN.store(clip, Ordering::SeqCst);
            (cal, timer, vol, raven, wifi, stats, clip)
        } else {
            (
                PILL_SHOW_CALENDAR_DROPDOWN.load(Ordering::SeqCst),
                PILL_SHOW_TIMER_DROPDOWN.load(Ordering::SeqCst),
                PILL_SHOW_VOLUME_DROPDOWN.load(Ordering::SeqCst),
                PILL_SHOW_RAVEN_MENU.load(Ordering::SeqCst),
                PILL_SHOW_WIFI_DROPDOWN.load(Ordering::SeqCst),
                PILL_SHOW_TOPBAR_STATS_DROPDOWN.load(Ordering::SeqCst),
                PILL_SHOW_CLIPBOARD_DROPDOWN.load(Ordering::SeqCst),
            )
        };

        let hud_active = HUD_ACTIVE.load(Ordering::SeqCst)
            || PILL_UI_WEAK.get()
                .and_then(|w| w.upgrade())
                .map(|ui| !ui.get_system_hud_kind().is_empty())
                .unwrap_or(false);

        let mut target_logical_h = if hud_active {
            f32::max(244.0, logical_idle_h)
        } else {
            logical_idle_h
        };
        if show_cal {
            target_logical_h = 560.0;
        } else if show_timer {
            target_logical_h = 300.0;
        } else if show_vol {
            target_logical_h = f32::max(244.0, 160.0 + logical_idle_h);
        } else if show_raven {
            target_logical_h = f32::max(244.0, 180.0 + logical_idle_h);
        } else if show_wifi {
            target_logical_h = f32::max(244.0, 364.0 + logical_idle_h);
        } else if show_stats {
            target_logical_h = f32::max(244.0, 228.0 + logical_idle_h);
        } else if show_clip {
            target_logical_h = f32::max(244.0, 336.0 + logical_idle_h);
        }
        let comp_h = (target_logical_h * scale).round() as i32;


        let x = if layout_full_width {
            info.rcMonitor.left
        } else {
            info.rcMonitor.left + (screen_width - comp_w) / 2 + (logical_offset_x * scale).round() as i32
        };

        // When auto-hide is active (either via the manual toggle OR via the
        // fullscreen/maximized detection) the window MUST sit flush at the top
        // of the monitor (y = rcMonitor.top) so the Slint "-self.height"
        // slide-up animation can physically exit the visible screen area.
        let auto_hide_active = settings.appearance.auto_hide
            || (settings.appearance.auto_hide_on_fullscreen && IS_FOREGROUND_FULLSCREEN.load(Ordering::SeqCst));
        let y = if auto_hide_active {
            info.rcMonitor.top   // always flush with screen top
        } else {
            info.rcMonitor.top + (logical_offset_y * scale).round() as i32
        };

        PILL_WIN_WIDTH_PHYS.store(comp_w, Ordering::SeqCst);
        PILL_WIN_HEIGHT_PHYS.store(comp_h, Ordering::SeqCst);
        store_slint_target_rect(x, y, comp_w, comp_h);

        let mut rect = windows::Win32::Foundation::RECT::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
        let current_w = rect.right - rect.left;
        let current_h = rect.bottom - rect.top;

        if rect.left != x || rect.top != y || current_w != comp_w || current_h != comp_h {
            use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_SHOWWINDOW, SWP_FRAMECHANGED};
            const HWND_TOPMOST_RAW: isize = -1;
            let _ = SetWindowPos(
                hwnd,
                HWND(HWND_TOPMOST_RAW),
                x,
                y,
                comp_w,
                comp_h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            );
            update_window_region(hwnd);
        }
    }
}

pub unsafe fn setup_slint_window_positioning(
    hwnd: HWND,
    logical_offset_x: f32,
    logical_offset_y: f32,
    logical_idle_width: f32,
    logical_idle_height: f32,
    scale: f32,
) {
    use windows::Win32::UI::WindowsAndMessaging::*;

    // Store logical offsets
    PILL_LOGICAL_OFFSET_X.store(logical_offset_x as i32, Ordering::SeqCst);
    PILL_LOGICAL_OFFSET_Y.store(logical_offset_y as i32, Ordering::SeqCst);
    PILL_LOGICAL_IDLE_HEIGHT.store(logical_idle_height as i32, Ordering::SeqCst);
    PILL_SCALE_FACTOR.store(scale.to_bits(), Ordering::SeqCst);
    update_appbar_reservation();

    if let Some(ui) = PILL_UI_WEAK.get().and_then(|w| w.upgrade()) {
        let (logical_screen_w, _) = get_primary_screen_logical_width();
        ui.set_screen_width(logical_screen_w);
    }



    // Explicitly delete from taskbar list to guarantee it never appears in Aero Peek previews or groupings
    remove_from_taskbar(hwnd);

    println!("[DIAGNOSTICS] --- BEFORE STYLE MODIFICATIONS ---");
    print_window_info(hwnd, "[TOP-LEVEL]");
    let _ = windows::Win32::UI::WindowsAndMessaging::EnumChildWindows(
        hwnd,
        Some(enum_child_print),
        LPARAM(0),
    );

    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    // Slint already sets TOPMOST and LAYERED (for transparency). We add TOOLWINDOW (no taskbar icon) and NOACTIVATE (no focus stealing)
    let new_ex_style = ex_style | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0;
    let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style as isize);

    // Explicitly enforce frameless and borderless popup style on the main window.
    // This strips out caption, thick frame, sysmenu, dlgframe, minimize/maximize boxes,
    // and forces WS_POPUP, ensuring Windows never paints standard OS title bars or borders
    // when focus changes, context menus are opened, or sounds are played.
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let new_style = (style | WS_POPUP.0) & !(WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0 | WS_DLGFRAME.0);
    let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);

    // Force style recalculation and taskbar update
    let _ = SetWindowPos(
        hwnd,
        HWND(0),
        0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
    );

    println!("[DIAGNOSTICS] --- AFTER STYLE MODIFICATIONS ---");
    print_window_info(hwnd, "[TOP-LEVEL]");
    let _ = windows::Win32::UI::WindowsAndMessaging::EnumChildWindows(
        hwnd,
        Some(enum_child_print),
        LPARAM(0),
    );

    // Recursively set up drag-accept files and subclass on the window and all child windows
    unsafe {
        setup_drag_drop_subclass_recursive(hwnd);
    }

    let settings = crate::settings::RavenSettings::load();
    let full_width = settings.advanced.full_width_bar;
    let top_bar_widgets_val = settings.advanced.top_bar_widgets;

    // Initialise hit-test atomics from settings so WM_NCHITTEST is correct from first paint
    PILL_FULL_WIDTH_BAR.store(full_width, Ordering::SeqCst);
    PILL_TOP_BAR_WIDGETS.store(top_bar_widgets_val, Ordering::SeqCst);

    // Calculate monitor coordinates and position/resize on startup
    let monitor = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST);
    let mut info = windows::Win32::Graphics::Gdi::MONITORINFO {
        cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info);
    let screen_width = info.rcMonitor.right - info.rcMonitor.left;

    // Fixed composition window: max(720, idle_w) + 24px (for inverse curves) x max(244, idle_h)
    let comp_w = if full_width {
        screen_width
    } else {
        ((f32::max(720.0, logical_idle_width) + 24.0) * scale).round() as i32
    };
    let comp_h = (f32::max(244.0, logical_idle_height) * scale).round() as i32;

    PILL_WIN_WIDTH_PHYS.store(comp_w, Ordering::SeqCst);
    PILL_WIN_HEIGHT_PHYS.store(comp_h, Ordering::SeqCst);
    PILL_VIS_WIDTH_PHYS.store((logical_idle_width * scale).round() as i32, Ordering::SeqCst);
    PILL_VIS_HEIGHT_PHYS.store((logical_idle_height * scale).round() as i32, Ordering::SeqCst);

    let x = if full_width {
        info.rcMonitor.left
    } else {
        info.rcMonitor.left + (screen_width - comp_w) / 2 + (logical_offset_x * scale).round() as i32
    };
    let y = info.rcMonitor.top + (logical_offset_y * scale).round() as i32;

    store_slint_target_rect(x, y, comp_w, comp_h);

    let _ = SetWindowPos(
        hwnd,
        HWND(HWND_TOPMOST_RAW),
        x,
        y,
        comp_w,
        comp_h,
        SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
    );
    update_window_region(hwnd);
}

pub unsafe fn snap_slint_window_to_top_center(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::*;

    let Some((x, y, w, h)) = load_slint_target_rect() else {
        return;
    };

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    let current_w = rect.right - rect.left;
    let current_h = rect.bottom - rect.top;

    if rect.left != x || rect.top != y || current_w != w || current_h != h {
        let _ = SetWindowPos(
            hwnd,
            HWND(HWND_TOPMOST_RAW),
            x,
            y,
            w,
            h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
    }
}

unsafe extern "system" fn slint_window_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    _dwrefdata: usize,
) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::UI::Shell::DefSubclassProc;

    let pill_hwnd = PILL_HWND.load(Ordering::SeqCst);
    if pill_hwnd != 0 && hwnd.0 != pill_hwnd {
        let root_hwnd = GetAncestor(hwnd, GA_ROOT);
        if root_hwnd.0 != pill_hwnd {
            return DefSubclassProc(hwnd, msg, wparam, lparam);
        }
    }

    // Gesture tab-switching handler
    let mut now_ms = 0;
    if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        now_ms = duration.as_millis() as u64;
    }

    match msg {
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            let last_time = LAST_SWITCH_TIME.load(Ordering::SeqCst);
            if now_ms - last_time >= 300 {
                let screen_x = (lparam.0 & 0xFFFF) as i16 as i32;
                let screen_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                
                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                let cx = screen_x - rect.left;
                let cy = screen_y - rect.top;
                
                let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
                let scale = if scale_bits > 0 {
                    f32::from_bits(scale_bits)
                } else {
                    let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
                    if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 }
                };
                let cx_logical = cx as f32 / scale;
                let cy_logical = cy as f32 / scale;

                let is_expanded = PILL_UI_WEAK.get()
                    .and_then(|w| w.upgrade())
                    .map(|ui| ui.get_is_expanded())
                    .unwrap_or(false);
                let show_apps = PILL_UI_WEAK.get()
                    .and_then(|w| w.upgrade())
                    .map(|ui| ui.get_top_bar_widget_apps() && !ui.get_active_apps_hidden() && ui.get_full_width_bar())
                    .unwrap_or(false);
                let current_idle_w = PILL_UI_WEAK.get()
                    .and_then(|w| w.upgrade())
                    .map(|ui| ui.get_idle_width())
                    .unwrap_or(260.0);
                let current_idle_h = PILL_UI_WEAK.get()
                    .and_then(|w| w.upgrade())
                    .map(|ui| ui.get_idle_height())
                    .unwrap_or(35.0);
                let win_w = rect.right - rect.left;
                let win_w_logical = win_w as f32 / scale;

                if !is_expanded && show_apps && cx_logical < (win_w_logical / 2.0 - current_idle_w / 2.0) && cy_logical <= current_idle_h {
                    if msg == WM_MOUSEWHEEL {
                        let delta = (wparam.0 >> 16) as i16;
                        let negated_delta = -delta;
                        let new_wparam = WPARAM(((negated_delta as u16 as usize) << 16) | (wparam.0 & 0xFFFF));
                        return DefSubclassProc(hwnd, WM_MOUSEHWHEEL, new_wparam, lparam);
                    }
                }
                
                let full_width_bar = PILL_FULL_WIDTH_BAR.load(Ordering::SeqCst);
                let is_split = PILL_IS_SPLIT_LAYOUT.load(Ordering::SeqCst);
                let cx_local = if full_width_bar {
                    if is_split {
                        cx_logical - (win_w_logical - 720.0) / 2.0
                    } else {
                        let vis_w = PILL_VIS_WIDTH_PHYS.load(Ordering::SeqCst) as f32;
                        let vis_w_logical = vis_w / scale;
                        cx_logical - (win_w_logical - vis_w_logical) / 2.0
                    }
                } else {
                    cx_logical
                };
                
                let active = get_active_tab();
                
                if active == "home" && cx_local >= 340.0 && cy_logical <= 48.0 {
                    let raw_delta = (wparam.0 >> 16) as i16 as i32;
                    let scroll_left = if msg == WM_MOUSEWHEEL {
                        raw_delta > 0
                    } else {
                        raw_delta < 0
                    };
                    
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = PILL_UI_WEAK.get().and_then(|w| w.upgrade()) {
                            let current_idx = ui.get_selected_day_index();
                            let count = slint::Model::row_count(&ui.get_calendar_days()) as i32;
                            if count > 0 {
                                let new_idx = if scroll_left {
                                    (current_idx - 1).max(0)
                                } else {
                                    (current_idx + 1).min(count - 1)
                                };
                                if new_idx != current_idx {
                                    ui.invoke_select_day(new_idx);
                                }
                            }
                        }
                    });
                    return LRESULT(0);
                }
                
                if active == "media" && cx_local < 130.0 {
                    let raw_delta = (wparam.0 >> 16) as i16 as i32;
                    let forward = if msg == WM_MOUSEWHEEL {
                        raw_delta < 0
                    } else {
                        raw_delta > 0
                    };
                    
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = PILL_UI_WEAK.get().and_then(|w| w.upgrade()) {
                            ui.invoke_cycle_media_source(forward);
                        }
                    });
                    return LRESULT(0);
                }
                
                // Allow scrolling to switch tabs only if we are not hovering over scrolling lists (like lyrics, shelf, or calendar events).
                let has_calendar = PILL_UI_WEAK.get()
                    .and_then(|w| w.upgrade())
                    .map(|ui| ui.get_tab_calendar())
                    .unwrap_or(true);
                
                let is_expanded = PILL_UI_WEAK.get()
                    .and_then(|w| w.upgrade())
                    .map(|ui| ui.get_is_expanded())
                    .unwrap_or(false);
                
                let allowed = is_expanded && (
                    if active == "home" {
                        !has_calendar || cx_local < 340.0
                    } else if active == "drop" {
                        if msg == WM_MOUSEHWHEEL {
                            !(cx_local >= 160.0 && cy_logical >= 130.0)
                        } else {
                            true
                        }
                    } else {
                        active == "clock" || active == "stats" || active == "media"
                    }
                );
                
                println!("[SCROLL-DEBUG] msg={:#x} active='{}' cx_logical={:.1} cx_local={:.1} has_calendar={} allowed={}", msg, active, cx_logical, cx_local, has_calendar, allowed);
                
                if allowed {
                    let raw_delta = (wparam.0 >> 16) as i16 as i32;
                    let accum_ref = if msg == WM_MOUSEHWHEEL { &SCROLL_ACCUMULATOR_X } else { &SCROLL_ACCUMULATOR_Y };
                    
                    // Reset scroll accumulators if user paused scrolling for > 500ms
                    let last_scroll = LAST_SCROLL_TIME.load(Ordering::SeqCst);
                    LAST_SCROLL_TIME.store(now_ms, Ordering::SeqCst);
                    if now_ms - last_scroll > 500 {
                        SCROLL_ACCUMULATOR_X.store(0, Ordering::SeqCst);
                        SCROLL_ACCUMULATOR_Y.store(0, Ordering::SeqCst);
                    }
                    
                    let new_accum = accum_ref.fetch_add(raw_delta, Ordering::SeqCst) + raw_delta;
                    
                    if new_accum.abs() >= 80 {
                        SCROLL_ACCUMULATOR_X.store(0, Ordering::SeqCst);
                        SCROLL_ACCUMULATOR_Y.store(0, Ordering::SeqCst);
                        LAST_SWITCH_TIME.store(now_ms, Ordering::SeqCst);
                        
                        let forward = if msg == WM_MOUSEWHEEL {
                            raw_delta < 0
                        } else {
                            raw_delta > 0
                        };
                        
                        if let Some(cb) = TAB_SWITCH_CALLBACK.get() {
                            cb(forward);
                        }
                    }
                }
            }
        }
        
        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            
            let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
            let scale = if scale_bits > 0 {
                f32::from_bits(scale_bits)
            } else {
                let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
                if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 }
            };
            let cx_logical = x as f32 / scale;
            let cy_logical = y as f32 / scale;
            
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            let full_width_bar = PILL_FULL_WIDTH_BAR.load(Ordering::SeqCst);
            let is_split = PILL_IS_SPLIT_LAYOUT.load(Ordering::SeqCst);
            let cx_local = if full_width_bar {
                let win_w = rect.right - rect.left;
                let win_w_logical = win_w as f32 / scale;
                if is_split {
                    cx_logical - (win_w_logical - 720.0) / 2.0
                } else {
                    let vis_w = PILL_VIS_WIDTH_PHYS.load(Ordering::SeqCst) as f32;
                    let vis_w_logical = vis_w / scale;
                    cx_logical - (win_w_logical - vis_w_logical) / 2.0
                }
            } else {
                cx_logical
            };
            
            let active = get_active_tab();
            
            let has_calendar = PILL_UI_WEAK.get()
                .and_then(|w| w.upgrade())
                .map(|ui| ui.get_tab_calendar())
                .unwrap_or(true);

            let is_expanded = PILL_UI_WEAK.get()
                .and_then(|w| w.upgrade())
                .map(|ui| ui.get_is_expanded())
                .unwrap_or(false);

            // Allow drag-swipe starting only outside sliders (bottom 130px) and vertical lists (right lyrics / file shelf / calendar list)
            let allowed = is_expanded && (
                (active == "home" && (!has_calendar || cx_local < 340.0))
                    || active == "clock"
                    || active == "stats"
                    || (active == "media" && cy_logical < 130.0)
                    || (active == "drop" && cx_local < 160.0 && cy_logical < 130.0)
            );
                
            if allowed {
                DRAG_START_X.store(x, Ordering::SeqCst);
                DRAG_START_Y.store(y, Ordering::SeqCst);
                IS_DRAGGING.store(true, Ordering::SeqCst);
            }
        }
        
        WM_LBUTTONUP => {
            if IS_DRAGGING.swap(false, Ordering::SeqCst) {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                
                let start_x = DRAG_START_X.load(Ordering::SeqCst);
                let start_y = DRAG_START_Y.load(Ordering::SeqCst);
                
                let delta_x = x - start_x;
                let delta_y = y - start_y;
                
                if delta_x.abs() >= 40 && delta_y.abs() <= 50 {
                    let last_time = LAST_SWITCH_TIME.load(Ordering::SeqCst);
                    if now_ms - last_time >= 300 {
                        LAST_SWITCH_TIME.store(now_ms, Ordering::SeqCst);
                        let forward = delta_x < 0;
                        if let Some(cb) = TAB_SWITCH_CALLBACK.get() {
                            cb(forward);
                        }
                    }
                }
            }
        }
        
        WM_MOUSEMOVE => {
            if IS_DRAGGING.load(Ordering::SeqCst) {
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                let start_y = DRAG_START_Y.load(Ordering::SeqCst);
                if (y - start_y).abs() > 60 {
                    IS_DRAGGING.store(false, Ordering::SeqCst);
                }
            }
        }
        WM_ACTIVATE => {
            let active = wparam.0 & 0xFFFF;
            if active == 0 { // WA_INACTIVE
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = PILL_UI_WEAK.get().and_then(|w| w.upgrade()) {
                        ui.invoke_close_transient_panels();
                        set_window_interactive_mode(false);
                        update_pill_window_layout();
                    }
                });
            }
        }
        _ => {}
    }

    if msg == WM_SHOWWINDOW || msg == WM_WINDOWPOSCHANGED {
        if hwnd.0 == pill_hwnd {
            remove_from_taskbar(hwnd);
        }
    }

    if msg == WM_HOTKEY {
        handle_hotkey(hwnd, wparam.0 as i32);
        return LRESULT(0);
    }

    if msg == WM_DROPFILES {
        let hdrop = windows::Win32::UI::Shell::HDROP(wparam.0 as isize);
        let mut pt = windows::Win32::Foundation::POINT::default();
        
        let mut cursor_pt = windows::Win32::Foundation::POINT::default();
        if GetCursorPos(&mut cursor_pt).is_ok() {
            let mut rect = RECT::default();
            let root_hwnd = GetAncestor(hwnd, GA_ROOT);
            if GetWindowRect(root_hwnd, &mut rect).is_ok() {
                pt.x = cursor_pt.x - rect.left;
                pt.y = cursor_pt.y - rect.top;
            }
        }

        let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
        let scale = if scale_bits > 0 {
            f32::from_bits(scale_bits)
        } else {
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
            if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 }
        };
        
        let cx_logical = pt.x as f32 / scale;
        let paths = dropped_paths(hdrop);
        
        if let Some(cb) = DROP_CALLBACK.get() {
            cb(paths, cx_logical);
        }
        return LRESULT(0);
    }

    if msg == WM_NCACTIVATE {
        // Prevent default title bar repainting when window gains/loses focus
        return DefSubclassProc(hwnd, msg, wparam, LPARAM(-1));
    }

    if msg == WM_NCPAINT {
        // Suppress non-client painting
        return LRESULT(0);
    }

    if msg == WM_WINDOWPOSCHANGING {
        let window_pos = lparam.0 as *mut WINDOWPOS;
        if let Some(window_pos) = window_pos.as_mut() {
            if let Some((tx, ty, tw, th)) = load_slint_target_rect() {
                window_pos.x = tx;
                window_pos.y = ty;
                window_pos.cx = tw;
                window_pos.cy = th;
                window_pos.flags.0 &= !SWP_NOSIZE.0;
                window_pos.flags.0 &= !SWP_NOMOVE.0;
                window_pos.flags.0 |= SWP_NOACTIVATE.0;
                window_pos.hwndInsertAfter = HWND(HWND_TOPMOST_RAW);
            } else {
                let center_x = window_pos.x + window_pos.cx / 2;
                let center_y = window_pos.y + window_pos.cy / 2;
                let target_point = windows::Win32::Foundation::POINT { x: center_x, y: center_y };
                let monitor = windows::Win32::Graphics::Gdi::MonitorFromPoint(
                    target_point,
                    windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
                );
                let mut info = windows::Win32::Graphics::Gdi::MONITORINFO {
                    cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
                    ..Default::default()
                };
                if windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info).as_bool() {
                    let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
                    let scale = if scale_bits > 0 {
                        f32::from_bits(scale_bits)
                    } else {
                        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
                        if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 }
                    };

                    let logical_offset_x = PILL_LOGICAL_OFFSET_X.load(Ordering::SeqCst) as f32;
                    let logical_offset_y = PILL_LOGICAL_OFFSET_Y.load(Ordering::SeqCst) as f32;

                    let offset_x_phys = (logical_offset_x * scale).round() as i32;
                    let offset_y_phys = (logical_offset_y * scale).round() as i32;

                    let target_w = PILL_WIN_WIDTH_PHYS.load(Ordering::SeqCst);
                    let target_h = PILL_WIN_HEIGHT_PHYS.load(Ordering::SeqCst);

                    if target_w > 0 && target_h > 0 {
                        window_pos.cx = target_w;
                        window_pos.cy = target_h;
                        window_pos.flags.0 &= !SWP_NOSIZE.0;
                    }

                    let w = window_pos.cx;

                    let screen_w = info.rcMonitor.right - info.rcMonitor.left;
                    let x = info.rcMonitor.left + (screen_w - w) / 2 + offset_x_phys;
                    let y = info.rcMonitor.top + offset_y_phys;

                    window_pos.x = x;
                    window_pos.y = y;
                    window_pos.hwndInsertAfter = HWND(HWND_TOPMOST_RAW);
                    
                    // Clear SWP_NOMOVE so our calculated coordinates are applied.
                    window_pos.flags = SET_WINDOW_POS_FLAGS(
                        (window_pos.flags.0 & !SWP_NOMOVE.0) | SWP_NOACTIVATE.0,
                    );
                }
            }
        }
        return DefSubclassProc(hwnd, msg, wparam, lparam);
    }

    // WM_NCHITTEST: return HTTRANSPARENT for cursor positions outside the visible pill/notch.
    // This makes clicks pass through the transparent margins of the fixed-size OS window.
    //
    // CRITICAL: When the left mouse button is held down (indicating a file drag from Explorer
    // or another app), we must NOT return HTTRANSPARENT. Otherwise, Windows skips our window
    // entirely and our OLE IDropTarget never receives DragEnter/DragOver/Drop events.
    // Drags from Explorer have LButton already pressed when the cursor enters our window,
    // while normal clicks on transparent areas happen with LButton NOT pressed.
    if msg == WM_NCHITTEST {
        // Check if a drag operation might be in progress (left mouse button held down)
        let lbutton_down = (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
            windows::Win32::UI::Input::KeyboardAndMouse::VK_LBUTTON.0 as i32,
        ) as u16 & 0x8000) != 0;

        let vis_w = PILL_VIS_WIDTH_PHYS.load(Ordering::SeqCst);
        let vis_h = PILL_VIS_HEIGHT_PHYS.load(Ordering::SeqCst);
        let vis_y = PILL_VIS_Y_PHYS.load(Ordering::SeqCst);

        // LPARAM low word = screen X, high word = screen Y
        let screen_x = (lparam.0 & 0xFFFF) as i16 as i32;
        let screen_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

        let root_hwnd = GetAncestor(hwnd, GA_ROOT);
        let check_hwnd = if root_hwnd.0 != 0 { root_hwnd } else { hwnd };

        let mut rect = RECT::default();
        let _ = GetWindowRect(check_hwnd, &mut rect);
        let win_w = rect.right - rect.left;
        let cx = screen_x - rect.left;  // client x relative to root
        let cy = screen_y - rect.top;   // client y relative to root

        let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
        let scale = if scale_bits > 0 {
            f32::from_bits(scale_bits)
        } else {
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(check_hwnd);
            if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 }
        };

        let is_split = PILL_IS_SPLIT_LAYOUT.load(Ordering::SeqCst);

        // Calculate if inside visible pill area or expanded panel area
        let inside_visible_pill = !is_split && {
            if vis_w > 0 && vis_h > 0 && win_w > 0 {
                let left  = (win_w - vis_w) / 2;
                let right = left + vis_w;
                let top = vis_y;
                let bottom = top + vis_h;
                cx >= left && cx < right && cy >= top && cy < bottom
            } else {
                false
            }
        };

        let full_width_bar = PILL_FULL_WIDTH_BAR.load(Ordering::SeqCst);
        let top_bar_widgets = PILL_TOP_BAR_WIDGETS.load(Ordering::SeqCst);

        // When full-width bar is active, the entire top strip (y: 0..bar_h, x: 0..win_w) is interactive
        let inside_full_width_bar = false;

        let inside_top_bar_widgets = full_width_bar && top_bar_widgets && {
            let logical_idle_h = PILL_LOGICAL_IDLE_HEIGHT.load(Ordering::SeqCst) as f32;
            let bar_h = (if is_split { logical_idle_h } else { vis_h as f32 / scale } * scale).round() as i32;
            if bar_h > 0 && win_w > 0 {
                let (left_w, right_w) = if let Some(ui) = PILL_UI_WEAK.get().and_then(|w| w.upgrade()) {
                    let mut lw = 16.0; // padding-left
                    if ui.get_top_bar_widget_raven() {
                        lw += 40.0;
                    }
                    let has_media = ui.get_top_bar_widget_media() && {
                        let title = ui.get_media_title();
                        !title.is_empty() && title != "No Media Playing"
                    };
                    if has_media {
                        lw += 262.0;
                    }
                    if ui.get_top_bar_widget_apps() {
                        let app_count = slint::Model::row_count(&ui.get_open_apps());
                        if !ui.get_active_apps_hidden() {
                            lw += app_count as f32 * 36.0;
                        }
                        lw += 32.0; // apps strip toggle
                    }

                    let mut rw = 16.0; // padding-right
                    if ui.get_top_bar_widget_stats() { rw += 30.0; }
                    if ui.get_top_bar_widget_clipboard() { rw += 30.0; }
                    if ui.get_top_bar_widget_volume() { rw += 28.0; }
                    if ui.get_top_bar_widget_wifi() { rw += 30.0; }
                    if ui.get_top_bar_widget_battery() { rw += 60.0; }
                    if ui.get_top_bar_widget_timer() { rw += 114.0; }
                    if ui.get_top_bar_widget_calendar() { rw += 104.0; }
                    (lw, rw)
                } else {
                    (480.0, 480.0) // Safe defaults if UI not loaded
                };

                let left_limit = ((left_w + 16.0) * scale).round() as i32;
                let right_limit = ((right_w + 16.0) * scale).round() as i32;

                cy >= 0 && cy < bar_h && (cx >= 0 && cx < left_limit || cx >= win_w - right_limit && cx < win_w)
            } else {
                false
            }
        };

        let mut in_header = false;
        let mut in_left_panel = false;
        let mut in_right_panel = false;

        if is_split {
            let panel_width = (720.0 * scale).round() as i32;
            let panel_left = (win_w - panel_width) / 2;
            let cx_rel = cx - panel_left;

            in_header = cx_rel >= (4.0 * scale).round() as i32 
                && cx_rel <= (692.0 * scale).round() as i32 
                && cy >= (10.0 * scale).round() as i32 
                && cy <= (38.0 * scale).round() as i32;

            in_left_panel = cx_rel >= (4.0 * scale).round() as i32 
                && cx_rel <= (348.0 * scale).round() as i32 
                && cy >= (50.0 * scale).round() as i32 
                && cy <= (216.0 * scale).round() as i32;

            in_right_panel = cx_rel >= (432.0 * scale).round() as i32 
                && cx_rel <= (692.0 * scale).round() as i32 
                && cy >= (50.0 * scale).round() as i32 
                && cy <= (216.0 * scale).round() as i32;
        }

        let show_cal = PILL_SHOW_CALENDAR_DROPDOWN.load(Ordering::SeqCst);
        let show_timer = PILL_SHOW_TIMER_DROPDOWN.load(Ordering::SeqCst);
        let show_vol = PILL_SHOW_VOLUME_DROPDOWN.load(Ordering::SeqCst);
        let show_raven = PILL_SHOW_RAVEN_MENU.load(Ordering::SeqCst);
        let show_wifi = PILL_SHOW_WIFI_DROPDOWN.load(Ordering::SeqCst);
        let show_stats = PILL_SHOW_TOPBAR_STATS_DROPDOWN.load(Ordering::SeqCst);
        let show_clip = PILL_SHOW_CLIPBOARD_DROPDOWN.load(Ordering::SeqCst);

        let idle_h_phys = (PILL_LOGICAL_IDLE_HEIGHT.load(Ordering::SeqCst) as f32 * scale).round() as i32;
        let idle_h_logical = PILL_LOGICAL_IDLE_HEIGHT.load(Ordering::SeqCst) as f32;

        let inside_dropdown = if show_cal {
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
            let right = (184.0 * scale).round() as i32;
            let top = ((idle_h_logical + 6.0) * scale).round() as i32;
            let bottom = ((idle_h_logical + 176.0) * scale).round() as i32;
            cx >= left && cx <= right && cy >= top && cy <= bottom
        } else if show_wifi {
            let left = win_w - (330.0 * scale).round() as i32;
            let right = win_w - (16.0 * scale).round() as i32;
            let top = idle_h_phys;
            let bottom = idle_h_phys + (364.0 * scale).round() as i32;
            cx >= left && cx <= right && cy >= top && cy <= bottom
        } else if show_stats {
            let left = win_w - (258.0 * scale).round() as i32;
            let right = win_w - (16.0 * scale).round() as i32;
            let top = idle_h_phys;
            let bottom = idle_h_phys + (228.0 * scale).round() as i32;
            cx >= left && cx <= right && cy >= top && cy <= bottom
        } else if show_clip {
            let left = win_w - (736.0 * scale).round() as i32;
            let right = win_w - (16.0 * scale).round() as i32;
            let top = idle_h_phys;
            let bottom = idle_h_phys + (336.0 * scale).round() as i32;
            cx >= left && cx <= right && cy >= top && cy <= bottom
        } else {
            false
        };

        let inside_expanded_panel = is_split && (in_header || in_left_panel || in_right_panel);

        // When a dropdown is open in full-width-bar mode, make the ENTIRE WINDOW interactive.
        // This guarantees Windows delivers ALL mouse events to Slint, which then routes them
        // internally through its own z-order (backdrop TouchArea + dropdown panel elements).
        // Coordinate-based guessing was unreliable; Slint's own hit testing handles the rest.
        let any_dropdown_open = show_cal || show_timer || show_vol || show_raven || show_wifi || show_stats || show_clip;


        // Determine subclass decision
        let final_result = if vis_w <= 0 || vis_h <= 0 {
            LRESULT(-1) // HTTRANSPARENT
        } else if lbutton_down {
            // Keep window interactive during drag/scroll/swipe gestures to maintain mouse capture
            LRESULT(1) // HTCLIENT
        } else if full_width_bar && any_dropdown_open {
            // Dropdown open: entire window is interactive, Slint routes internally
            LRESULT(1) // HTCLIENT
        } else if is_split {
            if inside_expanded_panel || inside_top_bar_widgets || inside_full_width_bar {
                LRESULT(1) // HTCLIENT
            } else {
                LRESULT(-1) // HTTRANSPARENT
            }
        } else {
            if inside_visible_pill || inside_top_bar_widgets || inside_full_width_bar || inside_dropdown {
                LRESULT(1) // HTCLIENT
            } else {
                LRESULT(-1) // HTTRANSPARENT
            }
        };

        // Query ex_style flags
        let mut ex_style = GetWindowLongPtrW(check_hwnd, GWL_EXSTYLE) as u32;
        let mut has_layered = (ex_style & WS_EX_LAYERED.0) != 0;
        
        // Enforce WS_EX_LAYERED if it was cleared by the UI/drawing backend
        if !has_layered {
            let target_ex = ex_style | WS_EX_LAYERED.0;
            let _ = SetWindowLongPtrW(check_hwnd, GWL_EXSTYLE, target_ex as isize);
            let _ = SetWindowPos(
                check_hwnd,
                HWND(0),
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
            );
            ex_style = target_ex;
            has_layered = true;
            println!("[DIAGNOSTICS] WS_EX_LAYERED was missing! Restored at runtime.");
        }

        // NOTE: WS_EX_TRANSPARENT is intentionally NOT used here.
        // WM_NCHITTEST returning HTTRANSPARENT is the correct and sufficient mechanism for
        // click-through behavior on specific window regions. Dynamically toggling WS_EX_TRANSPARENT
        // was causing a race: once set (when cursor crossed a transparent region), it made the
        // ENTIRE window pass-through, bypassing WM_NCHITTEST completely and making all dropdown
        // panels unclickable.

        
        let has_transparent = (ex_style & WS_EX_TRANSPARENT.0) != 0;
        let has_toolwindow = (ex_style & WS_EX_TOOLWINDOW.0) != 0;

        let result_str = if final_result.0 == 1 {
            "HTCLIENT"
        } else if final_result.0 == -1 {
            "HTTRANSPARENT"
        } else {
            "OTHER"
        };

        let mut msg_class = [0u16; 256];
        let msg_len = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut msg_class) as usize;
        let msg_class_str = String::from_utf16_lossy(&msg_class[..msg_len]);

        let mut root_class = [0u16; 256];
        let root_len = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(check_hwnd, &mut root_class) as usize;
        let root_class_str = String::from_utf16_lossy(&root_class[..root_len]);

        println!(
            "[HITTEST] x={} y={} result={} | msg_hwnd: {:p} (Class: {}) | root_hwnd: {:p} (Class: {}) | cursor_inside: visible_pill={} expanded_panel={} transparent_area={} | styles: WS_EX_TRANSPARENT={} WS_EX_LAYERED={} WS_EX_TOOLWINDOW={} | window_bounds: left={} top={} right={} bottom={} | interactive_bounds: split={} left={} right={} top={} bottom={}",
            screen_x, screen_y, result_str,
            hwnd.0 as *const std::ffi::c_void, msg_class_str,
            check_hwnd.0 as *const std::ffi::c_void, root_class_str,
            inside_visible_pill, inside_expanded_panel, (!inside_visible_pill && !inside_expanded_panel && !inside_top_bar_widgets && !inside_full_width_bar && !inside_dropdown),
            has_transparent, has_layered, has_toolwindow,
            rect.left, rect.top, rect.right, rect.bottom,
            is_split,
            if !is_split && vis_w > 0 && win_w > 0 { (win_w - vis_w) / 2 } else { 0 },
            if !is_split && vis_w > 0 && win_w > 0 { (win_w - vis_w) / 2 + vis_w } else { 0 },
            if !is_split && vis_w > 0 && win_w > 0 { vis_y } else { 0 },
            if !is_split && vis_w > 0 && win_w > 0 { vis_y + vis_h } else { 0 }
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());

        return final_result;
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub unsafe fn position_at_top_center(hwnd: HWND, width: i32, height: i32, offset_x: f32, offset_y: f32) {
    let monitor = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info);
    let screen_width = info.rcMonitor.right - info.rcMonitor.left;
    let x = info.rcMonitor.left + (screen_width - width) / 2 + offset_x as i32;
    let y = info.rcMonitor.top + offset_y as i32;
    let _ = SetWindowPos(hwnd, HWND(0), x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW);
}

pub unsafe fn center_window_top(hwnd: HWND, offset_x: f32, offset_y: f32) {
    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    let width = rect.right - rect.left;

    let monitor = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info);
    let screen_width = info.rcMonitor.right - info.rcMonitor.left;
    
    let x = info.rcMonitor.left + (screen_width - width) / 2 + offset_x as i32;
    let y = info.rcMonitor.top + offset_y as i32;
    
    let _ = SetWindowPos(hwnd, HWND(0), x, y, 0, 0, SWP_NOACTIVATE | SWP_SHOWWINDOW | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE);
}

pub fn get_top_center_position(width: i32, _height: i32, offset_x: f32, offset_y: f32) -> (i32, i32) {
    unsafe {
        let monitor = if PILL_HWND.load(Ordering::SeqCst) != 0 {
            windows::Win32::Graphics::Gdi::MonitorFromWindow(
                HWND(PILL_HWND.load(Ordering::SeqCst)),
                windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
            )
        } else {
            windows::Win32::Graphics::Gdi::MonitorFromPoint(
                POINT { x: 0, y: 0 },
                windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
            )
        };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut info);
        let screen_width = info.rcMonitor.right - info.rcMonitor.left;
        let x = info.rcMonitor.left + (screen_width - width) / 2 + offset_x as i32;
        let y = info.rcMonitor.top + offset_y as i32;
        (x, y)
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn lparam_point(lparam: LPARAM) -> (i32, i32) {
    let raw = lparam.0 as u32;
    let x = (raw & 0xffff) as i16 as i32;
    let y = ((raw >> 16) & 0xffff) as i16 as i32;
    (x, y)
}

unsafe fn dropped_paths(drop: HDROP) -> Vec<String> {
    let count = DragQueryFileW(drop, 0xFFFFFFFF, None);
    let mut paths = Vec::new();
    for index in 0..count {
        let len = DragQueryFileW(drop, index, None);
        if len == 0 {
            continue;
        }
        let mut buffer = vec![0u16; len as usize + 1];
        let written = DragQueryFileW(drop, index, Some(&mut buffer));
        if written > 0 {
            paths.push(String::from_utf16_lossy(&buffer[..written as usize]));
        }
    }
    DragFinish(drop);
    paths
}

// ── CUSTOM OLE DRAG & DROP SUPPORT ──

#[windows::core::implement(windows::Win32::System::Ole::IDropTarget)]
pub struct RavenDropTarget {
    ui_weak: slint::Weak<crate::Pill>,
    hwnd: HWND,
    shelf: crate::services::ShelfService,
}

impl RavenDropTarget {
    pub fn new(ui_weak: slint::Weak<crate::Pill>, hwnd: HWND, shelf: crate::services::ShelfService) -> Self {
        Self { ui_weak, hwnd, shelf }
    }

    fn get_logical_x(&self, pt: &POINTL) -> (bool, f32) {
        use std::sync::atomic::Ordering;

        let mut rect = RECT::default();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(self.hwnd, &mut rect);
        }
        let win_width = rect.right - rect.left;

        let scale_bits = PILL_SCALE_FACTOR.load(Ordering::SeqCst);
        let scale = if scale_bits > 0 {
            f32::from_bits(scale_bits)
        } else {
            1.0
        };

        let comp_w = ((720.0 + 24.0) * scale).round() as i32;
        let pill_left = (win_width - comp_w) / 2;

        let client_x = pt.x - rect.left - pill_left;
        let logical_x = client_x as f32 / scale;

        // Visual Share target is left area <= 176.0 logical px
        let left_target = logical_x <= 176.0;

        (left_target, logical_x)
    }

    fn update_hover_highlight(&self, pt: &POINTL) {
        let (left_target, _) = self.get_logical_x(pt);
        let ui_weak = self.ui_weak.clone();
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_share_active(left_target);
            ui.set_keep_active(!left_target);
        });
    }
}

impl windows::Win32::System::Ole::IDropTarget_Impl for RavenDropTarget {
    fn DragEnter(
        &self,
        _pdataobj: Option<&windows::Win32::System::Com::IDataObject>,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        println!("[IDropTarget] DragEnter at ({}, {})", pt.x, pt.y);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let settings = crate::settings::RavenSettings::load();
        if !settings.drop.enabled {
            unsafe {
                *pdweffect = windows::Win32::System::Ole::DROPEFFECT_NONE;
            }
            return Ok(());
        }

        if settings.drop.auto_expand {
            let ui_weak = self.ui_weak.clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.invoke_switch_tab("drop".into());
                ui.invoke_request_notch_open();
            });
        }

        self.update_hover_highlight(pt);

        unsafe {
            *pdweffect = windows::Win32::System::Ole::DROPEFFECT_COPY;
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        println!("[IDropTarget] DragOver at ({}, {})", pt.x, pt.y);
        let settings = crate::settings::RavenSettings::load();
        if !settings.drop.enabled {
            unsafe {
                *pdweffect = windows::Win32::System::Ole::DROPEFFECT_NONE;
            }
            return Ok(());
        }

        self.update_hover_highlight(pt);
        unsafe {
            *pdweffect = windows::Win32::System::Ole::DROPEFFECT_COPY;
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        println!("[IDropTarget] DragLeave");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let ui_weak = self.ui_weak.clone();
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_share_active(false);
            ui.set_keep_active(false);
        });
        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: Option<&windows::Win32::System::Com::IDataObject>,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        println!("[IDropTarget] Drop at ({}, {})", pt.x, pt.y);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        unsafe {
            *pdweffect = windows::Win32::System::Ole::DROPEFFECT_NONE;
        }

        let settings = crate::settings::RavenSettings::load();
        if !settings.drop.enabled {
            return Ok(());
        }

        let Some(pdataobj) = pdataobj else {
            println!("[IDropTarget] Drop: no data object");
            return Ok(());
        };

        let paths = match extract_paths_from_dataobject(pdataobj) {
            Ok(paths) => paths,
            Err(e) => {
                println!("[IDropTarget] Drop: extract_paths error: {:?}", e);
                return Ok(());
            }
        };

        println!("[IDropTarget] Drop: extracted {} paths: {:?}", paths.len(), paths);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        if paths.is_empty() {
            return Ok(());
        }

        let (left_target, _) = self.get_logical_x(pt);

        let ui_weak = self.ui_weak.clone();
        let shelf = self.shelf.clone();

        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_share_active(false);
            ui.set_keep_active(false);

            if left_target {
                if let Some(first_path) = paths.first() {
                    let provider_id = ui.get_share_provider_id().to_string();
                    println!("[DROP] COM Share file: {} with {}", first_path, provider_id);
                    ui.invoke_shelf_share_file(first_path.clone().into(), provider_id.into());
                }
            } else {
                println!("[DROP] COM Add files to shelf: {:?}", paths);
                shelf.add_paths(paths.clone());
                
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
        });

        unsafe {
            *pdweffect = windows::Win32::System::Ole::DROPEFFECT_COPY;
        }
        Ok(())
    }
}

fn extract_paths_from_dataobject(
    data_obj: &windows::Win32::System::Com::IDataObject,
) -> windows::core::Result<Vec<String>> {
    use windows::Win32::System::Com::{FORMATETC, TYMED_HGLOBAL, DVASPECT_CONTENT};
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
    use windows::Win32::System::Ole::CF_HDROP;

    let format_etc = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };

    unsafe {
        let medium = data_obj.GetData(&format_etc)?;
        let hdrop = HDROP(medium.u.hGlobal.0 as _);

        let file_count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
        let mut paths = Vec::new();

        for i in 0..file_count {
            let path_len = DragQueryFileW(hdrop, i, None);
            if path_len == 0 {
                continue;
            }
            let mut buffer = vec![0u16; path_len as usize + 1];
            let written = DragQueryFileW(hdrop, i, Some(&mut buffer));
            if written > 0 {
                paths.push(String::from_utf16_lossy(&buffer[..written as usize]));
            }
        }

        let _ = windows::Win32::System::Ole::ReleaseStgMedium(&medium as *const _ as *mut _);
        Ok(paths)
    }
}

pub unsafe fn register_custom_drop_target(
    hwnd: HWND,
    ui_weak: slint::Weak<crate::Pill>,
    shelf: crate::services::ShelfService,
) {
    let revoke_result = windows::Win32::System::Ole::RevokeDragDrop(hwnd);
    println!("[SETUP] RevokeDragDrop result: {:?}", revoke_result);
    
    let our_drop_target: windows::Win32::System::Ole::IDropTarget = RavenDropTarget::new(ui_weak, hwnd, shelf).into();
    let register_result = windows::Win32::System::Ole::RegisterDragDrop(hwnd, &our_drop_target);
    match &register_result {
        Ok(()) => println!("[SETUP] Custom OLE Drop Target registered SUCCESSFULLY on HWND: {:?}", hwnd),
        Err(e) => println!("[SETUP] RegisterDragDrop FAILED on HWND {:?}: {:?}", hwnd, e),
    }
    let _ = std::io::Write::flush(&mut std::io::stdout());

    // Keep the COM object alive by preventing implicit drop — COM AddRef was called by RegisterDragDrop
    std::mem::forget(our_drop_target);
}

#[windows::core::implement(windows::Win32::System::Ole::IDropTarget)]
pub struct AppsContainerDropTarget;

impl AppsContainerDropTarget {
    pub fn new() -> Self {
        Self
    }
}

impl windows::Win32::System::Ole::IDropTarget_Impl for AppsContainerDropTarget {
    fn DragEnter(
        &self,
        _pdataobj: Option<&windows::Win32::System::Com::IDataObject>,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            *pdweffect = windows::Win32::System::Ole::DROPEFFECT_COPY;
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            *pdweffect = windows::Win32::System::Ole::DROPEFFECT_COPY;
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: Option<&windows::Win32::System::Com::IDataObject>,
        _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            *pdweffect = windows::Win32::System::Ole::DROPEFFECT_NONE;
        }

        let Some(pdataobj) = pdataobj else {
            return Ok(());
        };

        if let Ok(paths) = extract_paths_from_dataobject(pdataobj) {
            if !paths.is_empty() {
                if let Some(cb) = crate::widgets::APPS_CONTAINER_DROP_CALLBACK.get() {
                    cb(paths);
                }
                unsafe {
                    *pdweffect = windows::Win32::System::Ole::DROPEFFECT_COPY;
                }
            }
        }

        Ok(())
    }
}

pub unsafe fn register_apps_container_drop_target(hwnd: HWND) {
    let _ = windows::Win32::System::Ole::RevokeDragDrop(hwnd);
    let target: windows::Win32::System::Ole::IDropTarget = AppsContainerDropTarget::new().into();
    let result = windows::Win32::System::Ole::RegisterDragDrop(hwnd, &target);
    match &result {
        Ok(()) => println!("[APPS-CONTAINER] OLE drop target registered on HWND: {:?}", hwnd),
        Err(e) => println!("[APPS-CONTAINER] OLE drop target failed on HWND {:?}: {:?}", hwnd, e),
    }
    std::mem::forget(target);
}

pub unsafe fn update_window_region(hwnd: HWND) {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::Foundation::BOOL;
    
    let root_hwnd = windows::Win32::UI::WindowsAndMessaging::GetAncestor(hwnd, windows::Win32::UI::WindowsAndMessaging::GA_ROOT);
    let target_hwnd = if root_hwnd.0 != 0 { root_hwnd } else { hwnd };
    
    // Clear any window region to restore full composition-based transparency and corners
    let _ = SetWindowRgn(target_hwnd, HRGN::default(), BOOL(1));
}

pub unsafe fn set_window_click_through(hwnd: HWND, click_through: bool) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    
    let root_hwnd = GetAncestor(hwnd, GA_ROOT);
    let target_hwnd = if root_hwnd.0 != 0 { root_hwnd } else { hwnd };
    
    let ex_style = GetWindowLongPtrW(target_hwnd, GWL_EXSTYLE) as u32;
    let target_style = if click_through {
        ex_style | WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0
    } else {
        (ex_style & !WS_EX_TRANSPARENT.0) | WS_EX_LAYERED.0
    };
    
    if ex_style != target_style {
        println!("[STYLE-CHANGE-BEFORE] HWND: {:?}, ClickThrough: {}, ExStyleBefore: 0x{:X}, WS_EX_TRANSPARENT: {}, WS_EX_LAYERED: {}", 
                 target_hwnd.0, click_through, ex_style, (ex_style & WS_EX_TRANSPARENT.0) != 0, (ex_style & WS_EX_LAYERED.0) != 0);
        let _ = SetWindowLongPtrW(target_hwnd, GWL_EXSTYLE, target_style as isize);
        let _ = SetWindowPos(
            target_hwnd,
            HWND(0),
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );
        let post_ex_style = GetWindowLongPtrW(target_hwnd, GWL_EXSTYLE) as u32;
        println!("[STYLE-CHANGE-AFTER] HWND: {:?}, ExStyleAfter: 0x{:X}, WS_EX_TRANSPARENT: {}, WS_EX_LAYERED: {}", 
                 target_hwnd.0, post_ex_style, (post_ex_style & WS_EX_TRANSPARENT.0) != 0, (post_ex_style & WS_EX_LAYERED.0) != 0);
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

pub unsafe fn print_window_info(hwnd: HWND, prefix: &str) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    
    let mut class_name = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut class_name) as usize;
    let class_str = String::from_utf16_lossy(&class_name[..len]);
    
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let parent = GetParent(hwnd);
    let owner = GetWindowLongPtrW(hwnd, GWLP_HWNDPARENT);
    
    println!(
        "{} HWND: {:p} | Class: {} | Style: 0x{:08X} | ExStyle: 0x{:08X} | Parent: {:p} | Owner: 0x{:X} | WS_EX_LAYERED: {} | WS_EX_TRANSPARENT: {} | WS_EX_TOOLWINDOW: {}",
        prefix,
        hwnd.0 as *const std::ffi::c_void,
        class_str,
        style,
        ex_style,
        parent.0 as *const std::ffi::c_void,
        owner,
        (ex_style & WS_EX_LAYERED.0) != 0,
        (ex_style & WS_EX_TRANSPARENT.0) != 0,
        (ex_style & WS_EX_TOOLWINDOW.0) != 0
    );
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

pub unsafe extern "system" fn enum_child_print(child_hwnd: HWND, _lparam: LPARAM) -> windows::Win32::Foundation::BOOL {
    print_window_info(child_hwnd, "  [CHILD]");
    windows::Win32::Foundation::BOOL(1)
}

pub fn set_run_on_startup(enabled: bool) -> Result<(), String> {
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };
    use windows::core::PCWSTR;

    let subkey = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value_name = wide("Raven Notch");

    let mut hkey = HKEY::default();
    let res = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        )
    };

    if let Err(e) = res {
        return Err(format!("Failed to open registry key: {:?}", e));
    }

    let result = if enabled {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current exe path: {}", e))?;
        let quoted_path = format!("\"{}\"", exe_path.to_string_lossy());
        let path_wide = wide(&quoted_path);
        let path_bytes = unsafe {
            std::slice::from_raw_parts(path_wide.as_ptr() as *const u8, path_wide.len() * 2)
        };

        let set_res = unsafe {
            RegSetValueExW(
                hkey,
                PCWSTR(value_name.as_ptr()),
                0,
                REG_SZ,
                Some(path_bytes),
            )
        };
        if let Err(e) = set_res {
            Err(format!("Failed to set registry value: {:?}", e))
        } else {
            Ok(())
        }
    } else {
        let del_res = unsafe {
            RegDeleteValueW(hkey, PCWSTR(value_name.as_ptr()))
        };
        if let Err(e) = del_res {
            let code = e.code().0;
            // 0x80070002 is HRESULT for ERROR_FILE_NOT_FOUND (2)
            if code != -2147024894 {
                Err(format!("Failed to delete registry value: {:?}", e))
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    };

    unsafe {
        let _ = RegCloseKey(hkey);
    }

    result
}



