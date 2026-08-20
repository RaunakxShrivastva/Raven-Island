use crate::events::EventBus;
use crate::renderer::NativeRenderer;
use crate::services::ServiceRegistry;
use crate::settings::RavenSettings;
use crate::window::NativeWindow;

pub fn run() -> crate::window::WindowResult<()> {
    let settings = RavenSettings::load();
    let events = EventBus::new();
    let services = ServiceRegistry::new(settings.clone(), events.clone());
    let renderer = NativeRenderer::new(settings.clone());
    let window = NativeWindow::create(settings, services, renderer)?;

    window.run_message_loop()
}
