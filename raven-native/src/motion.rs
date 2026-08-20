pub const HOME_WIDTH: f32 = 720.0;
pub const HOME_HEIGHT: f32 = 244.0;
pub const EXPANDED_WIDTH: f32 = 780.0;
pub const EXPANDED_HEIGHT: f32 = 340.0;
pub const MOTION_OPEN_MS: u32 = 510;
pub const MOTION_CONTENT_REVEAL_MS: u32 = 300;
pub const MOTION_CONTENT_EXIT_MS: u32 = 0;
pub const MOTION_CLOSE_MS: u32 = 390;
pub const SPRING_FRAME_COUNT: usize = 64;
pub const AUTO_HIDE_EDGE_HIT_PX: f32 = 3.0;
pub const CURSOR_HIT_PADDING: f32 = 8.0;
pub const SPRING_RESPONSE: f32 = 13.6;

use std::sync::{Arc, Mutex};

/// Snapshot of physics state shared across threads (Win32 → Slint)
#[derive(Clone, Copy, Debug, Default)]
pub struct MotionSnapshot {
    pub content_opacity: f32,
    pub border_radius: f32,
    pub width: f32,
    pub height: f32,
    pub is_open: bool,
    pub phase: NotchPhase,
    pub panel_ready: bool,
}

/// Thread-safe bridge: Win32 physics thread writes, Slint UI thread reads.
#[derive(Clone, Default)]
pub struct SharedMotionBridge {
    inner: Arc<Mutex<MotionSnapshot>>,
}

impl SharedMotionBridge {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(MotionSnapshot::default())) }
    }

    pub fn write(&self, snapshot: MotionSnapshot) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = snapshot;
        }
    }

    pub fn read(&self) -> MotionSnapshot {
        self.inner.lock().map(|g| *g).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotchPhase {
    Closed,
    Opening,
    OpenContentStaging,
    Open,
    ClosingContent,
    Closing,
}

impl Default for NotchPhase {
    fn default() -> Self {
        Self::Closed
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NotchGeometry {
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct MotionState {
    pub phase: NotchPhase,
    pub geometry: NotchGeometry,
    pub closed: NotchGeometry,
    pub open: NotchGeometry,
    pub expanding: bool,
    pub start_time: Option<std::time::Instant>,
    pub notch_opacity: f32,
    pub content_opacity: f32,
}

impl MotionState {
    pub fn closed(width: f32, height: f32, radius: f32) -> Self {
        let closed = NotchGeometry {
            width,
            height,
            radius,
        };
        let open = NotchGeometry {
            width: HOME_WIDTH,
            height: HOME_HEIGHT,
            radius: 20.0,
        };
        Self {
            phase: NotchPhase::Closed,
            geometry: closed,
            closed,
            open,
            expanding: false,
            start_time: None,
            notch_opacity: 1.0,
            content_opacity: 0.0,
        }
    }

    pub fn is_openish(&self) -> bool {
        matches!(
            self.phase,
            NotchPhase::Opening
                | NotchPhase::OpenContentStaging
                | NotchPhase::Open
                | NotchPhase::ClosingContent
        )
    }

    pub fn is_animating(&self) -> bool {
        matches!(
            self.phase,
            NotchPhase::Opening
                | NotchPhase::OpenContentStaging
                | NotchPhase::ClosingContent
                | NotchPhase::Closing
        )
    }

    pub fn begin_open(&mut self) {
        if self.is_openish() && !matches!(self.phase, NotchPhase::Closing | NotchPhase::Closed) {
            return;
        }
        self.start_time = Some(std::time::Instant::now());
        self.expanding = true;
        self.phase = NotchPhase::Opening;
        self.content_opacity = 0.0;
    }

    pub fn begin_close(&mut self) {
        if matches!(self.phase, NotchPhase::Closed | NotchPhase::Closing | NotchPhase::ClosingContent) {
            return;
        }
        self.start_time = Some(std::time::Instant::now());
        self.expanding = false;
        self.phase = NotchPhase::ClosingContent;
    }

    pub fn begin_toggle(&mut self) {
        if self.is_openish() {
            self.begin_close();
        } else {
            self.begin_open();
        }
    }

    pub fn set_open_geometry(&mut self, width: f32, height: f32, radius: f32) {
        self.open = NotchGeometry { width, height, radius };
        if matches!(self.phase, NotchPhase::Open | NotchPhase::OpenContentStaging) {
            self.geometry = self.open;
        }
    }

    pub fn set_closed_geometry(&mut self, width: f32, height: f32, radius: f32) {
        self.closed = NotchGeometry {
            width,
            height,
            radius,
        };
        if matches!(self.phase, NotchPhase::Closed) {
            self.geometry = self.closed;
        }
    }

    pub fn snapshot(&self) -> MotionSnapshot {
        MotionSnapshot {
            content_opacity: self.content_opacity,
            border_radius: self.geometry.radius,
            width: self.geometry.width,
            height: self.geometry.height,
            is_open: !matches!(self.phase, NotchPhase::Closed),
            phase: self.phase,
            panel_ready: matches!(
                self.phase,
                NotchPhase::OpenContentStaging | NotchPhase::Open | NotchPhase::ClosingContent
            ),
        }
    }

    pub fn advance_frame(&mut self) -> bool {
        let Some(start) = self.start_time else {
            return false;
        };

        let elapsed = start.elapsed().as_millis() as u32;

        if self.expanding {
            if elapsed < MOTION_OPEN_MS {
                self.phase = NotchPhase::Opening;
                let progress = elapsed as f32 / MOTION_OPEN_MS as f32;
                self.geometry = morph_frame(self.closed, self.open, progress, true);
                self.notch_opacity = 1.0;
                self.content_opacity = 0.0;
                true
            } else if elapsed < MOTION_OPEN_MS + MOTION_CONTENT_REVEAL_MS {
                self.phase = NotchPhase::OpenContentStaging;
                self.geometry = self.open;
                self.notch_opacity = 1.0;
                
                let reveal_elapsed = elapsed - MOTION_OPEN_MS;
                let progress = reveal_elapsed as f32 / MOTION_CONTENT_REVEAL_MS as f32;
                self.content_opacity = solve_cubic_bezier(0.16, 1.0, 0.3, 1.0, progress);
                true
            } else {
                self.phase = NotchPhase::Open;
                self.geometry = self.open;
                self.notch_opacity = 1.0;
                self.content_opacity = 1.0;
                self.start_time = None;
                false
            }
        } else {
            if elapsed < MOTION_CONTENT_EXIT_MS {
                self.phase = NotchPhase::ClosingContent;
                self.geometry = self.open;
                self.notch_opacity = 1.0;
                self.content_opacity = 0.0;
                true
            } else if elapsed < MOTION_CONTENT_EXIT_MS + MOTION_CLOSE_MS {
                self.phase = NotchPhase::Closing;
                let progress = (elapsed - MOTION_CONTENT_EXIT_MS) as f32 / MOTION_CLOSE_MS as f32;
                self.geometry = morph_frame(self.open, self.closed, progress, false);
                self.notch_opacity = 1.0;
                self.content_opacity = 0.0;
                true
            } else {
                self.phase = NotchPhase::Closed;
                self.geometry = self.closed;
                self.notch_opacity = 1.0;
                self.content_opacity = 0.0;
                self.start_time = None;
                false
            }
        }
    }
}

pub fn spring_value(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    if t == 0.0 {
        return 0.0;
    }
    if t == 1.0 {
        return 1.0;
    }

    let f = |time: f32| 1.0 - (1.0 + SPRING_RESPONSE * time) * (-SPRING_RESPONSE * time).exp();

    let val = f(t);
    let end = f(1.0);
    if end == 0.0 {
        val.clamp(0.0, 1.0)
    } else {
        (val / end).clamp(0.0, 1.0)
    }
}

pub fn staged_spring_value(progress: f32, lead: f32, expanding: bool) -> f32 {
    let p = (progress + lead).clamp(0.0, 1.0);
    let _ = expanding;
    spring_value(p)
}

pub fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

pub fn morph_frame(from: NotchGeometry, to: NotchGeometry, progress: f32, expanding: bool) -> NotchGeometry {
    let offset = progress.clamp(0.0, 1.0);
    let width_spring = if offset >= 1.0 { 1.0 } else { staged_spring_value(offset, 0.0, expanding) };
    let height_spring = if offset >= 1.0 { 1.0 } else { staged_spring_value(offset, 0.0, expanding) };
    let radius_spring = if offset >= 1.0 { 1.0 } else { staged_spring_value(offset, 0.0, expanding) };

    NotchGeometry {
        width: lerp(from.width, to.width, width_spring),
        height: lerp(from.height, to.height, height_spring),
        radius: lerp(from.radius, to.radius, radius_spring),
    }
}

pub fn solve_cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    
    // Initial guess
    let mut u = t;
    
    // Newton-Raphson iteration
    for _ in 0..8 {
        let x = 3.0 * (1.0 - u) * (1.0 - u) * u * x1 + 3.0 * (1.0 - u) * u * u * x2 + u * u * u;
        let dx = 3.0 * x1 + (6.0 * x2 - 12.0 * x1) * u + (3.0 + 9.0 * x1 - 9.0 * x2) * u * u;
        
        if dx.abs() < 1e-6 {
            break;
        }
        
        let next_u = u - (x - t) / dx;
        if next_u < 0.0 || next_u > 1.0 {
            break; // Fallback to binary search
        }
        u = next_u;
    }
    
    // Fallback to binary search if needed (or to refine)
    let mut low = 0.0;
    let mut high = 1.0;
    let mut x = 3.0 * (1.0 - u) * (1.0 - u) * u * x1 + 3.0 * (1.0 - u) * u * u * x2 + u * u * u;
    
    if (x - t).abs() > 1e-4 {
        for _ in 0..12 {
            u = (low + high) * 0.5;
            x = 3.0 * (1.0 - u) * (1.0 - u) * u * x1 + 3.0 * (1.0 - u) * u * u * x2 + u * u * u;
            if x < t {
                low = u;
            } else {
                high = u;
            }
        }
    }
    
    // Calculate y(u)
    3.0 * (1.0 - u) * (1.0 - u) * u * y1 + 3.0 * (1.0 - u) * u * u * y2 + u * u * u
}
