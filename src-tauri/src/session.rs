use manga_parser_core::{Event, Reporter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub const EVENT_CHANNEL: &str = "pipeline://event";

/// Одна сборка за раз: параллельные запуски чистили бы одну и ту же папку.
#[derive(Default)]
pub struct Session {
    busy: AtomicBool,
    cancelled: Arc<AtomicBool>,
}

impl Session {
    pub fn start(&self) -> Option<Ticket<'_>> {
        if self.busy.swap(true, Ordering::SeqCst) {
            return None;
        }
        self.cancelled.store(false, Ordering::SeqCst);
        Some(Ticket { session: self })
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn reporter(&self, app: AppHandle) -> WindowReporter {
        WindowReporter {
            app,
            cancelled: Arc::clone(&self.cancelled),
        }
    }
}

pub struct Ticket<'a> {
    session: &'a Session,
}

impl Drop for Ticket<'_> {
    fn drop(&mut self) {
        self.session.busy.store(false, Ordering::SeqCst);
    }
}

/// Адаптер порта `Reporter`: события ядра уходят в окно как есть.
pub struct WindowReporter {
    app: AppHandle,
    cancelled: Arc<AtomicBool>,
}

impl Reporter for WindowReporter {
    fn report(&self, event: Event) {
        let _ = self.app.emit(EVENT_CHANNEL, event);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}
