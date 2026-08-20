use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum RavenEvent {
    SettingsChanged,
    MediaChanged,
    NotificationReceived,
    ShortcutAction(String),
    CaptureDone,
    TimerTick(u64),
    ShareStatusChanged,
    RedrawRequested,
    ShowSettings,
    AccountTokenReceived(String),
}

type Subscriber = Box<dyn Fn(RavenEvent) + Send + Sync + 'static>;

#[derive(Clone, Default)]
pub struct EventBus {
    subscribers: Arc<Mutex<Vec<Subscriber>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe<F>(&self, handler: F)
    where
        F: Fn(RavenEvent) + Send + Sync + 'static,
    {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(Box::new(handler));
        }
    }

    pub fn emit(&self, event: RavenEvent) {
        if let Ok(subscribers) = self.subscribers.lock() {
            for subscriber in subscribers.iter() {
                subscriber(event.clone());
            }
        }
    }
}
