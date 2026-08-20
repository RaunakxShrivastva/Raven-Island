#[cfg(test)]
use crate::monitor_math::{self, MonitorBounds, CursorPos};

#[test]
fn fullscreen_bounds_allow_dpi_rounding() {
    let monitor = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1080 };

    assert!(monitor_math::rect_covers_monitor(monitor, monitor, 8));
    assert!(monitor_math::rect_covers_monitor(
        MonitorBounds { left: -8, top: -8, right: 1928, bottom: 1088 },
        monitor,
        8,
    ));
    assert!(monitor_math::rect_covers_monitor(
        MonitorBounds { left: 8, top: 8, right: 1912, bottom: 1072 },
        monitor,
        8,
    ));

    assert!(!monitor_math::rect_covers_monitor(
        MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1040 },
        monitor,
        8,
    ));
}

#[test]
fn fullscreen_like_accepts_visible_taskbar_work_area() {
    let monitor = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1080 };
    let work_area = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1040 };
    let window = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1040 };

    assert!(monitor_math::rect_is_fullscreen_like(window, monitor, work_area, 8, false));
}

#[test]
fn fullscreen_like_accepts_maximized_app() {
    let monitor = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1080 };
    let work_area = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1040 };
    let window = MonitorBounds { left: 100, top: 100, right: 1200, bottom: 900 };

    assert!(monitor_math::rect_is_fullscreen_like(window, monitor, work_area, 8, true));
}

#[test]
fn fullscreen_tolerance_scales_with_monitor_dpi() {
    assert_eq!(monitor_math::fullscreen_tolerance(96), 8);
    assert_eq!(monitor_math::fullscreen_tolerance(144), 12);
    assert_eq!(monitor_math::fullscreen_tolerance(192), 16);
    assert_eq!(monitor_math::fullscreen_tolerance(0), 8);
}

#[test]
fn test_primary_and_secondary_side_by_side() {
    let primary = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1080 };
    let secondary = MonitorBounds { left: 1920, top: 0, right: 3840, bottom: 1080 };
    
    // Test notch window centering on both monitors
    let comp_w = 744; // idle_w 720 + 24 padding
    let offset_x = 0;
    let offset_y = 0;
    
    let (p_x, p_y) = monitor_math::calculate_notch_center_pos(primary, comp_w, offset_x, offset_y);
    assert_eq!(p_x, (1920 - comp_w) / 2); // 588
    assert_eq!(p_y, 0);
    
    let (s_x, s_y) = monitor_math::calculate_notch_center_pos(secondary, comp_w, offset_x, offset_y);
    assert_eq!(s_x, 1920 + (1920 - comp_w) / 2); // 2508
    assert_eq!(s_y, 0);

    // Test hover detection at top edge of primary monitor
    let scale = 1.0_f32;
    let logical_idle_w = 260.0_f32;
    let logical_idle_h = 38.0_f32;
    let padding = 10;
    let edge_hit = 20;

    // Hover center top-edge of primary
    let cursor_primary_hover = CursorPos { x: 960, y: 2 };
    let hovering_p = monitor_math::check_hover(
        cursor_primary_hover,
        primary,
        scale,
        logical_idle_w,
        logical_idle_h,
        offset_x,
        offset_y,
        false, // not already hovered
        padding,
        edge_hit,
        false,
    );
    assert!(hovering_p);

    // Hover center top-edge of secondary
    let cursor_secondary_hover = CursorPos { x: 1920 + 960, y: 2 };
    let hovering_s = monitor_math::check_hover(
        cursor_secondary_hover,
        secondary,
        scale,
        logical_idle_w,
        logical_idle_h,
        offset_x,
        offset_y,
        false,
        padding,
        edge_hit,
        false,
    );
    assert!(hovering_s);
}

#[test]
fn test_mixed_dpi_scaling() {
    // Primary 100% (scale = 1.0)
    let primary = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1080 };
    // Secondary 150% (scale = 1.5)
    let secondary = MonitorBounds { left: 1920, top: 0, right: 3840, bottom: 1080 };
    
    let logical_idle_w = 260.0_f32;
    let logical_idle_h = 38.0_f32;
    let padding_logical = 10.0_f32;
    let edge_hit_logical = 20.0_f32;

    // 1. Primary monitor calculations (100% scale)
    let scale_p = 1.0_f32;
    let padding_p = (padding_logical * scale_p).round() as i32; // 10
    let edge_hit_p = (edge_hit_logical * scale_p).round() as i32; // 20
    let offset_x_p = 0;
    let offset_y_p = 0;
    
    // Hovering inside bounds on primary
    let hover_p = monitor_math::check_hover(
        CursorPos { x: 960, y: 15 },
        primary,
        scale_p,
        logical_idle_w,
        logical_idle_h,
        offset_x_p,
        offset_y_p,
        false,
        padding_p,
        edge_hit_p,
        false,
    );
    assert!(hover_p);

    // Hovering outside y bounds (y=25 > edge_hit_p=20) on primary
    let hover_p_outside = monitor_math::check_hover(
        CursorPos { x: 960, y: 25 },
        primary,
        scale_p,
        logical_idle_w,
        logical_idle_h,
        offset_x_p,
        offset_y_p,
        false,
        padding_p,
        edge_hit_p,
        false,
    );
    assert!(!hover_p_outside);

    // 2. Secondary monitor calculations (150% scale)
    let scale_s = 1.5_f32;
    let padding_s = (padding_logical * scale_s).round() as i32; // 15
    let edge_hit_s = (edge_hit_logical * scale_s).round() as i32; // 30
    let offset_x_s = 0;
    let offset_y_s = 0;

    // Hovering inside bounds on secondary (y=25 < edge_hit_s=30)
    let hover_s = monitor_math::check_hover(
        CursorPos { x: 2880, y: 25 }, // Center of secondary screen
        secondary,
        scale_s,
        logical_idle_w,
        logical_idle_h,
        offset_x_s,
        offset_y_s,
        false,
        padding_s,
        edge_hit_s,
        false,
    );
    assert!(hover_s); // Valid because scaling expanded edge_hit zone from 20px to 30px
}

#[test]
fn test_negative_coordinates_left_monitor() {
    // Secondary monitor is left of primary (bounds: -1920 to 0)
    let secondary_left = MonitorBounds { left: -1920, top: 0, right: 0, bottom: 1080 };
    let _primary = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1080 };
    
    let comp_w = 744;
    let offset_x = 0;
    let offset_y = 0;

    // Centering position on the negative coordinate monitor
    let (x, y) = monitor_math::calculate_notch_center_pos(secondary_left, comp_w, offset_x, offset_y);
    assert_eq!(x, -1920 + (1920 - comp_w) / 2); // -1332
    assert_eq!(y, 0);

    // Hover on negative coordinate monitor top-edge
    let scale = 1.0_f32;
    let logical_idle_w = 260.0_f32;
    let logical_idle_h = 38.0_f32;
    let padding = 10;
    let edge_hit = 20;

    let hover_negative = monitor_math::check_hover(
        CursorPos { x: -960, y: 2 },
        secondary_left,
        scale,
        logical_idle_w,
        logical_idle_h,
        offset_x,
        offset_y,
        false,
        padding,
        edge_hit,
        false,
    );
    assert!(hover_negative);
}

#[test]
fn test_vertically_stacked_monitors() {
    // Secondary monitor is above primary (bounds: top -1080 to 0)
    let secondary_top = MonitorBounds { left: 0, top: -1080, right: 1920, bottom: 0 };
    let _primary = MonitorBounds { left: 0, top: 0, right: 1920, bottom: 1080 };
    
    let comp_w = 744;
    let offset_x = 0;
    let offset_y = 0;

    // Centering position on the top monitor
    let (x, y) = monitor_math::calculate_notch_center_pos(secondary_top, comp_w, offset_x, offset_y);
    assert_eq!(x, (1920 - comp_w) / 2);
    assert_eq!(y, -1080);

    // Hover on top monitor top-edge (y = -1078)
    let scale = 1.0_f32;
    let logical_idle_w = 260.0_f32;
    let logical_idle_h = 38.0_f32;
    let padding = 10;
    let edge_hit = 20;

    let hover_stacked = monitor_math::check_hover(
        CursorPos { x: 960, y: -1078 },
        secondary_top,
        scale,
        logical_idle_w,
        logical_idle_h,
        offset_x,
        offset_y,
        false,
        padding,
        edge_hit,
        false,
    );
    assert!(hover_stacked);
}
