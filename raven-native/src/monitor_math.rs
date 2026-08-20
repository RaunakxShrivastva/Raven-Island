#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorBounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl MonitorBounds {
    pub fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    pub fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorPos {
    pub x: i32,
    pub y: i32,
}

/// Returns a physical-pixel tolerance suitable for comparing DWM/window bounds
/// with monitor bounds. Invisible resize borders grow with display DPI.
pub fn fullscreen_tolerance(dpi: u32) -> i32 {
    let dpi = dpi.max(96) as f32;
    ((8.0 * dpi / 96.0).round() as i32).clamp(8, 32)
}

/// Checks whether a physical window rectangle covers a physical monitor
/// rectangle, allowing for small compositor and DPI rounding differences.
pub fn rect_covers_monitor(
    window: MonitorBounds,
    monitor: MonitorBounds,
    tolerance: i32,
) -> bool {
    if window.width() <= 0 || window.height() <= 0 || monitor.width() <= 0 || monitor.height() <= 0 {
        return false;
    }

    let tolerance = tolerance.max(0);
    window.left <= monitor.left + tolerance
        && window.top <= monitor.top + tolerance
        && window.right >= monitor.right - tolerance
        && window.bottom >= monitor.bottom - tolerance
}

pub fn rect_is_fullscreen_like(
    window: MonitorBounds,
    monitor: MonitorBounds,
    work_area: MonitorBounds,
    tolerance: i32,
    is_maximized: bool,
) -> bool {
    rect_covers_monitor(window, monitor, tolerance)
        || rect_covers_monitor(window, work_area, tolerance)
        || is_maximized
}

pub fn check_hover(
    cursor: CursorPos,
    monitor: MonitorBounds,
    scale: f32,
    logical_idle_w: f32,
    logical_idle_h: f32,
    offset_x_phys: i32,
    offset_y_phys: i32,
    is_currently_hovered: bool,
    padding: i32,
    edge_hit: i32,
    full_width_bar: bool,
) -> bool {
    let screen_w = monitor.right - monitor.left;
    let idle_w_phys = (logical_idle_w * scale).round() as i32;
    let idle_h_phys = (logical_idle_h * scale).round() as i32;
    
    let in_x_range = if full_width_bar {
        cursor.x >= monitor.left && cursor.x <= monitor.right
    } else {
        let monitor_center = monitor.left + screen_w / 2;
        let notch_left = monitor_center - idle_w_phys / 2 + offset_x_phys;
        let notch_right = notch_left + idle_w_phys;
        cursor.x >= notch_left - padding && cursor.x <= notch_right + padding
    };
    
    if !is_currently_hovered {
        in_x_range
            && cursor.y >= monitor.top - 4
            && cursor.y <= monitor.top + edge_hit
    } else {
        in_x_range
            && cursor.y >= monitor.top - 4
            && cursor.y <= monitor.top + offset_y_phys + idle_h_phys + padding
    }
}

pub fn calculate_notch_center_pos(
    monitor: MonitorBounds,
    comp_w: i32,
    offset_x_phys: i32,
    offset_y_phys: i32,
) -> (i32, i32) {
    let screen_w = monitor.right - monitor.left;
    let x = monitor.left + (screen_w - comp_w) / 2 + offset_x_phys;
    let y = monitor.top + offset_y_phys;
    (x, y)
}
