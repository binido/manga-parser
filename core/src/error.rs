use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("источник не найден: {0}")]
    SourceMissing(PathBuf),

    #[error("источник должен быть .zip-архивом или папкой с архивами глав")]
    UnsupportedSource,

    #[error("название выходной папки пустое или содержит разделители пути")]
    InvalidOutputName,

    #[error("обложка не найдена: {0}")]
    CoverMissing(PathBuf),

    #[error("обложка должна быть изображением: jpg, jpeg, png, gif или webp")]
    UnsupportedCover,

    #[error("не найдено ни одного .zip-архива с главами")]
    NoChapters,

    #[error("в архивах нет изображений поддерживаемых форматов")]
    NoImages,

    #[error("не удалось открыть архив {path}: {source}")]
    Archive {
        path: PathBuf,
        source: zip::result::ZipError,
    },

    #[error("ошибка файловой системы: {0}")]
    Io(#[from] std::io::Error),

    #[error("обработка отменена")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, Error>;
