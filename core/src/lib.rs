//! Подготовка манги, скачанной с InkStory, к сборке в Kindle Comic Converter:
//! главы распаковываются, страницы получают сквозную нумерацию и складываются
//! в одну плоскую папку.

mod archive;
mod error;
mod natural_sort;
mod pipeline;
mod report;

pub use error::{Error, Result};
pub use pipeline::{run, Job, Outcome};
pub use report::{Event, Level, Reporter};
