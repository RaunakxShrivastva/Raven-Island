use crate::motion::NotchGeometry;
use crate::widgets::NativeTab;
use std::collections::HashMap;
use std::path::Path;
use windows::core::{PCWSTR, Result as WindowsResult};
use windows::Win32::Foundation::HWND;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_BITMAP_PROPERTIES, D2D1_DRAW_TEXT_OPTIONS_NONE,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT, ID2D1Factory,
    ID2D1Bitmap, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_MEDIUM,
    DWRITE_MEASURING_MODE_NATURAL, IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBackendKind {
    GdiCompatibility,
    DirectCompositionDirect2D,
}

#[derive(Clone, Debug)]
pub struct RenderScene {
    pub geometry: NotchGeometry,
    pub is_open: bool,
    pub current_tab: NativeTab,
    pub notch_opacity: f32,
    pub content_opacity: f32,
    pub clock_text: String,
    pub status_text: String,
    pub cpu_pct: f32,
    pub ram_pct: f32,
    pub battery_text: String,
    pub power_text: String,
    pub caffeine_text: String,
    pub media_title: String,
    pub media_artist: String,
    pub media_album: String,
    pub media_album_art_path: String,
    pub media_source: String,
    pub media_is_playing: bool,
    pub media_has_media: bool,
    pub media_progress_pct: f32,
    pub timer_label: String,
    pub timer_running: bool,
    pub stopwatch_label: String,
    pub stopwatch_running: bool,
    pub shelf_items: Vec<String>,
    pub shelf_image_paths: Vec<String>,
    pub notification_access: String,
    pub notifications: Vec<String>,
    pub notification_icon_paths: Vec<String>,
    pub capture_enabled: bool,
    pub capture_mode: String,
    pub capture_recording_mode: String,
    pub capture_dir: String,
    pub capture_last: String,
    pub capture_last_path: String,
    pub capture_message: String,
    pub calendar_source: String,
    pub calendar_status: String,
    pub calendar_events: Vec<String>,
    pub settings_path: String,
    pub settings_summary: String,
    pub static_asset_paths: Vec<String>,
}

pub trait RenderBackend {
    fn kind(&self) -> RenderBackendKind;
    fn render_scene(&mut self, hwnd: HWND, scene: &RenderScene) -> bool;
}

#[derive(Default)]
pub struct DirectCompositionBackend {
    factory: Option<ID2D1Factory>,
    text_factory: Option<IDWriteFactory>,
    title_format: Option<IDWriteTextFormat>,
    body_format: Option<IDWriteTextFormat>,
    target: Option<ID2D1HwndRenderTarget>,
    texture_cache: HashMap<String, ID2D1Bitmap>,
    last_size: (u32, u32),
}

impl DirectCompositionBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_initialized(&self) -> bool {
        self.target.is_some()
    }

    fn ensure_target(&mut self, hwnd: HWND, _scene: &RenderScene) -> WindowsResult<()> {
        let scale_bits = crate::window::PILL_SCALE_FACTOR.load(std::sync::atomic::Ordering::SeqCst);
        let scale = if scale_bits > 0 {
            f32::from_bits(scale_bits)
        } else {
            1.0
        };

        let mut rect = windows::Win32::Foundation::RECT::default();
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect) };
        let size = (
            ((rect.right - rect.left).max(1) as u32),
            ((rect.bottom - rect.top).max(1) as u32),
        );

        if self.target.is_some() && self.last_size == size {
            return Ok(());
        }

        let factory = match &self.factory {
            Some(factory) => factory.clone(),
            None => {
                let factory: ID2D1Factory =
                    unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
                self.factory = Some(factory.clone());
                factory
            }
        };

        let render_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0 * scale,
            dpiY: 96.0 * scale,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd,
            pixelSize: D2D_SIZE_U {
                width: size.0,
                height: size.1,
            },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };

        self.target = Some(unsafe { factory.CreateHwndRenderTarget(&render_props, &hwnd_props)? });
        self.texture_cache.clear();
        self.last_size = size;
        Ok(())
    }

    fn ensure_text(&mut self) -> WindowsResult<()> {
        if self.title_format.is_some() && self.body_format.is_some() {
            return Ok(());
        }

        let text_factory = match &self.text_factory {
            Some(factory) => factory.clone(),
            None => {
                let factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
                self.text_factory = Some(factory.clone());
                factory
            }
        };

        let font = wide_null("Segoe UI");
        let locale = wide_null("en-us");
        self.title_format = Some(unsafe {
            text_factory.CreateTextFormat(
                PCWSTR(font.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                18.0,
                PCWSTR(locale.as_ptr()),
            )?
        });
        self.body_format = Some(unsafe {
            text_factory.CreateTextFormat(
                PCWSTR(font.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_MEDIUM,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                13.0,
                PCWSTR(locale.as_ptr()),
            )?
        });
        Ok(())
    }
}

impl RenderBackend for DirectCompositionBackend {
    fn kind(&self) -> RenderBackendKind {
        RenderBackendKind::DirectCompositionDirect2D
    }

    fn render_scene(&mut self, hwnd: HWND, scene: &RenderScene) -> bool {
        if self.ensure_target(hwnd, scene).is_err() || self.ensure_text().is_err() {
            return false;
        }

        let Some(target) = self.target.clone() else {
            return false;
        };

        unsafe {
            target.BeginDraw();

            let clear = target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                None,
            );
            let notch = target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.02,
                    g: 0.02,
                    b: 0.025,
                    a: scene.notch_opacity.clamp(0.0, 1.0),
                },
                None,
            );

            let text = target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.96,
                    g: 0.96,
                    b: 0.98,
                    a: scene.content_opacity.clamp(0.0, 1.0),
                },
                None,
            );
            let muted = target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.58,
                    g: 0.58,
                    b: 0.62,
                    a: scene.content_opacity.clamp(0.0, 1.0),
                },
                None,
            );

            if let (Ok(clear), Ok(notch), Ok(text), Ok(muted)) = (clear, notch, text, muted) {
                let scale_bits = crate::window::PILL_SCALE_FACTOR.load(std::sync::atomic::Ordering::SeqCst);
                let scale = if scale_bits > 0 {
                    f32::from_bits(scale_bits)
                } else {
                    1.0
                };
                let target_w = self.last_size.0 as f32 / scale;
                let target_h = self.last_size.1 as f32 / scale;
                let x = (target_w - scene.geometry.width) / 2.0;

                // Clear the entire client viewport before translating
                target.FillRectangle(
                    &D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: target_w,
                        bottom: target_h,
                    },
                    &clear,
                );

                // Apply translation transform to center the capsule notch
                let translation = Matrix3x2::translation(x, 0.0);
                target.SetTransform(&translation);

                target.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F {
                            left: 0.0,
                            top: 0.0,
                            right: scene.geometry.width,
                            bottom: scene.geometry.height,
                        },
                        radiusX: scene.geometry.radius,
                        radiusY: scene.geometry.radius,
                    },
                    &notch,
                );

                self.draw_text(&target, "Raven Native", 24.0, 7.0, scene.geometry.width - 32.0, 30.0, &text, true);

                if scene.is_open {
                    self.draw_text(&target,
                        &scene.clock_text,
                        24.0,
                        43.0,
                        scene.geometry.width - 48.0,
                        22.0,
                        &muted,
                        false,
                    );
                    let panel = scene.current_tab.descriptor();
                    self.draw_text(&target, panel.title, 24.0, 88.0, scene.geometry.width - 48.0, 26.0, &text, true);
                    self.draw_text(&target, panel.detail, 24.0, 113.0, scene.geometry.width - 48.0, 22.0, &muted, false);
                    self.draw_text(&target,
                        &scene.status_text,
                        24.0,
                        139.0,
                        scene.geometry.width - 48.0,
                        22.0,
                        &muted,
                        false,
                    );
                    if scene.current_tab == NativeTab::Stats {
                        self.draw_text(&target,
                            &format!("CPU meter: {:.0}%     RAM meter: {:.0}%", scene.cpu_pct, scene.ram_pct),
                            24.0,
                            166.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                        self.draw_text(&target,
                            &format!("{}     {}", scene.battery_text, scene.power_text),
                            24.0,
                            193.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        self.draw_text(&target,
                            &scene.caffeine_text,
                            24.0,
                            218.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        self.draw_text(&target,
                            "[ Toggle caffeine ]",
                            24.0,
                            scene.geometry.height - 96.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                        self.draw_text(&target,
                            "[ Vol - ]     [ Mute ]     [ Vol + ]     [ Bright - ]     [ Bright + ]",
                            24.0,
                            scene.geometry.height - 72.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                    }
                    if scene.current_tab == NativeTab::Media {
                        let playback = if scene.media_is_playing { "Playing" } else { "Paused" };
                        let title = if scene.media_has_media {
                            scene.media_title.as_str()
                        } else {
                            "No active media session"
                        };
                        self.draw_text(&target,
                            title,
                            24.0,
                            166.0,
                            scene.geometry.width - 48.0,
                            24.0,
                            &text,
                            true,
                        );
                        self.draw_text(&target,
                            &format!(
                                "{}  |  {}  |  {:.0}%  |  {}",
                                scene.media_artist,
                                playback,
                                scene.media_progress_pct,
                                scene.media_source
                            ),
                            24.0,
                            193.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        if !scene.media_album_art_path.is_empty() {
                            self.draw_cached_image(
                                &target,
                                &scene.media_album_art_path,
                                24.0,
                                166.0,
                                48.0,
                                48.0,
                            );
                        }
                        if !scene.media_album.is_empty() {
                            self.draw_text(&target,
                                &scene.media_album,
                                24.0,
                                216.0,
                                scene.geometry.width - 48.0,
                                22.0,
                                &muted,
                                false,
                            );
                        }
                        self.draw_text(&target,
                            "[ Previous ]     [ Play / Pause ]     [ Next ]",
                            24.0,
                            224.0,
                            scene.geometry.width - 48.0,
                            24.0,
                            &text,
                            false,
                        );
                    }
                    if scene.current_tab == NativeTab::Clock {
                        self.draw_text(&target,
                            &format!(
                                "Timer {}  {}",
                                scene.timer_label,
                                if scene.timer_running { "Running" } else { "Paused" }
                            ),
                            24.0,
                            166.0,
                            scene.geometry.width - 48.0,
                            24.0,
                            &text,
                            true,
                        );
                        self.draw_text(&target,
                            "[ Timer Start / Pause ]     [ Timer Reset ]",
                            24.0,
                            193.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        self.draw_text(&target,
                            &format!(
                                "Stopwatch {}  {}",
                                scene.stopwatch_label,
                                if scene.stopwatch_running { "Running" } else { "Stopped" }
                            ),
                            24.0,
                            218.0,
                            scene.geometry.width - 48.0,
                            24.0,
                            &text,
                            true,
                        );
                        self.draw_text(&target,
                            "[ Stopwatch Start / Pause ]     [ Stopwatch Reset ]",
                            24.0,
                            244.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                    }
                    if scene.current_tab == NativeTab::Drop {
                        if scene.shelf_items.is_empty() {
                            self.draw_text(&target,
                                "Drop files onto Raven to keep them on the native shelf.",
                                24.0,
                                166.0,
                                scene.geometry.width - 48.0,
                                22.0,
                                &text,
                                false,
                            );
                        } else {
                            self.draw_text(&target,
                                "Native shelf items",
                                24.0,
                                166.0,
                                scene.geometry.width - 48.0,
                                24.0,
                                &text,
                                true,
                            );
                            for (index, item) in scene.shelf_items.iter().take(4).enumerate() {
                                self.draw_text(
                                    &target,
                                    item,
                                    24.0,
                                    194.0 + index as f32 * 19.0,
                                    scene.geometry.width - 48.0,
                                    18.0,
                                    &muted,
                                    false,
                                );
                            }
                            for (index, path) in scene.shelf_image_paths.iter().take(3).enumerate() {
                                self.draw_cached_image(
                                    &target,
                                    path,
                                    scene.geometry.width - 174.0 + index as f32 * 50.0,
                                    166.0,
                                    42.0,
                                    42.0,
                                );
                            }
                        }
                        self.draw_text(&target,
                            "[ Open first ]     [ Reveal first ]     [ Clear ]",
                            24.0,
                            scene.geometry.height - 72.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                    }
                    if scene.current_tab == NativeTab::Notifications {
                        self.draw_text(&target,
                            &format!("Notification access: {}", scene.notification_access),
                            24.0,
                            166.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                        if scene.notifications.is_empty() {
                            self.draw_text(&target,
                                "No recent notifications available.",
                                24.0,
                                193.0,
                                scene.geometry.width - 48.0,
                                22.0,
                                &muted,
                                false,
                            );
                        } else {
                            for (index, item) in scene.notifications.iter().take(3).enumerate() {
                                if let Some(path) = scene.notification_icon_paths.get(index) {
                                    if !path.is_empty() {
                                        self.draw_cached_image(
                                            &target,
                                            path,
                                            24.0,
                                            191.0 + index as f32 * 22.0,
                                            16.0,
                                            16.0,
                                        );
                                    }
                                }
                                self.draw_text(&target,
                                    item,
                                    24.0, // Should this be shifted right? Yes, let's shift it if icon is present, but for simplicity let's just draw text over/next to it
                                    193.0 + index as f32 * 22.0,
                                    scene.geometry.width - 48.0,
                                    20.0,
                                    &muted,
                                    false,
                                );
                            }
                        }
                        self.draw_text(&target,
                            "[ Open notification settings ]",
                            24.0,
                            scene.geometry.height - 72.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                    }
                    if scene.current_tab == NativeTab::Capture {
                        self.draw_text(&target,
                            &format!(
                                "Screenshot mode: {}  |  Recording: {}",
                                scene.capture_mode, scene.capture_recording_mode
                            ),
                            24.0,
                            166.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                        self.draw_text(&target,
                            &format!(
                                "Status: {}{}",
                                scene.capture_message,
                                if scene.capture_enabled { "" } else { " (disabled)" }
                            ),
                            24.0,
                            193.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        self.draw_text(&target,
                            &format!("Folder: {}", scene.capture_dir),
                            24.0,
                            218.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        self.draw_text(&target,
                            &scene.capture_last,
                            24.0,
                            243.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        if !scene.capture_last_path.is_empty() {
                            self.draw_cached_image(
                                &target,
                                &scene.capture_last_path,
                                scene.geometry.width - 116.0,
                                166.0,
                                72.0,
                                54.0,
                            );
                        }
                        self.draw_text(&target,
                            "[ Screenshot ]     [ Region ]     [ Open last ]     [ Open folder ]",
                            24.0,
                            scene.geometry.height - 72.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                    }
                    if scene.current_tab == NativeTab::Calendar {
                        self.draw_text(&target,
                            &format!("Source: {}  |  {}", scene.calendar_source, scene.calendar_status),
                            24.0,
                            166.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                        if scene.calendar_events.is_empty() {
                            self.draw_text(&target,
                                "No upcoming events loaded.",
                                24.0,
                                193.0,
                                scene.geometry.width - 48.0,
                                22.0,
                                &muted,
                                false,
                            );
                        } else {
                            for (index, item) in scene.calendar_events.iter().take(4).enumerate() {
                                self.draw_text(&target,
                                    item,
                                    24.0,
                                    193.0 + index as f32 * 22.0,
                                    scene.geometry.width - 48.0,
                                    20.0,
                                    &muted,
                                    false,
                                );
                            }
                        }
                        self.draw_text(&target,
                            "[ Refresh calendar ]",
                            24.0,
                            scene.geometry.height - 72.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                    }
                    if scene.current_tab == NativeTab::Settings {
                        self.draw_text(&target,
                            &scene.settings_summary,
                            24.0,
                            166.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                        self.draw_text(&target,
                            &format!("File: {}", scene.settings_path),
                            24.0,
                            193.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        self.draw_text(&target,
                            "Schema stays shared with the current Raven app during migration.",
                            24.0,
                            218.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                        self.draw_text(&target,
                            "[ Width - ]     [ Width + ]     [ Opacity - ]     [ Opacity + ]",
                            24.0,
                            scene.geometry.height - 100.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                        self.draw_text(&target,
                            "[ Hover ]     [ Open settings file ]",
                            24.0,
                            scene.geometry.height - 72.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &text,
                            false,
                        );
                    }
                    if scene.current_tab == NativeTab::Home {
                        for (index, path) in scene.static_asset_paths.iter().take(3).enumerate() {
                            self.draw_cached_image(
                                &target,
                                path,
                                24.0 + index as f32 * 58.0,
                                166.0,
                                48.0,
                                48.0,
                            );
                        }
                        self.draw_text(&target,
                            "Native assets cached as Direct2D bitmaps",
                            24.0,
                            224.0,
                            scene.geometry.width - 48.0,
                            22.0,
                            &muted,
                            false,
                        );
                    }

                    let tab_width = (scene.geometry.width / NativeTab::ALL.len() as f32).max(1.0);
                    for (index, tab) in NativeTab::ALL.iter().enumerate() {
                        let brush = if *tab == scene.current_tab { &text } else { &muted };
                        self.draw_text(&target,
                            tab.label(),
                            index as f32 * tab_width + 18.0,
                            scene.geometry.height - 43.0,
                            tab_width - 22.0,
                            24.0,
                            brush,
                            false,
                        );
                    }
                }

                // Reset the transform matrix to identity
                let identity = Matrix3x2::identity();
                target.SetTransform(&identity);
            }

            target.EndDraw(None, None).is_ok()
        }
    }
}

impl DirectCompositionBackend {
    unsafe fn draw_cached_image(
        &mut self,
        target: &ID2D1HwndRenderTarget,
        path: &str,
        left: f32,
        top: f32,
        width: f32,
        height: f32,
    ) {
        if !Path::new(path).exists() {
            return;
        }
        if !self.texture_cache.contains_key(path) {
            if let Some(bitmap) = create_bitmap_from_path(target, path) {
                self.texture_cache.insert(path.to_string(), bitmap);
            }
        }
        let Some(bitmap) = self.texture_cache.get(path) else {
            return;
        };
        let rect = D2D_RECT_F {
            left,
            top,
            right: left + width.max(1.0),
            bottom: top + height.max(1.0),
        };
        target.DrawBitmap(bitmap, Some(&rect), 0.92, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
    }

    unsafe fn draw_text(
        &self,
        target: &ID2D1HwndRenderTarget,
        value: &str,
        left: f32,
        top: f32,
        width: f32,
        height: f32,
        brush: &ID2D1SolidColorBrush,
        title: bool,
    ) {
        let Some(format) = (if title { &self.title_format } else { &self.body_format }) else {
            return;
        };
        let text = wide(value);
        target.DrawText(
            &text,
            format,
            &D2D_RECT_F {
                left,
                top,
                right: left + width.max(1.0),
                bottom: top + height.max(1.0),
            },
            brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn create_bitmap_from_path(target: &ID2D1HwndRenderTarget, path: &str) -> Option<ID2D1Bitmap> {
    let image = image::open(path).ok()?.resize(256, 256, image::imageops::FilterType::Lanczos3);
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let mut bgra = Vec::with_capacity((width * height * 4) as usize);
    for pixel in rgba.pixels() {
        let [r, g, b, a] = pixel.0;
        bgra.push(b);
        bgra.push(g);
        bgra.push(r);
        bgra.push(a);
    }

    let props = D2D1_BITMAP_PROPERTIES {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
    };
    target
        .CreateBitmap(
            D2D_SIZE_U { width, height },
            Some(bgra.as_ptr().cast()),
            width * 4,
            &props,
        )
        .ok()
}

