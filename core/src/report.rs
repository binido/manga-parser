use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    Log {
        level: Level,
        message: String,
    },
    ChaptersFound {
        total: usize,
    },
    ChapterDone {
        index: usize,
        total: usize,
        name: String,
        images: usize,
    },
    Finished {
        chapters: usize,
        images: usize,
        output: PathBuf,
    },
}

/// Порт наружу: ядро сообщает о ходе работы и спрашивает про отмену,
/// ничего не зная о том, кто его вызвал.
pub trait Reporter: Send + Sync {
    fn report(&self, event: Event);

    fn is_cancelled(&self) -> bool {
        false
    }
}

impl Reporter for () {
    fn report(&self, _event: Event) {}
}

pub(crate) trait ReporterExt {
    fn info(&self, message: impl Into<String>);
    fn warn(&self, message: impl Into<String>);
}

impl<R: Reporter + ?Sized> ReporterExt for R {
    fn info(&self, message: impl Into<String>) {
        self.report(Event::Log {
            level: Level::Info,
            message: message.into(),
        });
    }

    fn warn(&self, message: impl Into<String>) {
        self.report(Event::Log {
            level: Level::Warn,
            message: message.into(),
        });
    }
}
