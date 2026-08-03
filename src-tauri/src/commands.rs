use crate::session::Session;
use manga_parser_core::{Job, Outcome};
use std::path::PathBuf;
use tauri::ipc::Response;
use tauri::{AppHandle, State};

/// Обложку показывают в окне целиком, поэтому крупные файлы отсекаем до чтения:
/// в вебвью такая картинка всё равно не нужна.
const MAX_PREVIEW_BYTES: u64 = 32 * 1024 * 1024;

#[tauri::command]
pub async fn prepare(
    app: AppHandle,
    session: State<'_, Session>,
    job: Job,
) -> Result<Outcome, String> {
    let _ticket = session
        .start()
        .ok_or_else(|| "Сборка уже выполняется".to_string())?;
    let reporter = session.reporter(app);

    tauri::async_runtime::spawn_blocking(move || manga_parser_core::run(&job, &reporter))
        .await
        .map_err(|_| "Обработчик неожиданно завершился".to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn cancel(session: State<'_, Session>) {
    session.cancel();
}

/// Отдаёт байты обложки сырым ответом, чтобы окно собрало из них blob-ссылку.
#[tauri::command]
pub fn cover_preview(path: PathBuf) -> Result<Response, String> {
    if !manga_parser_core::is_image(&path) {
        return Err("файл не похож на изображение".to_string());
    }

    let size = std::fs::metadata(&path)
        .map_err(|error| error.to_string())?
        .len();
    if size > MAX_PREVIEW_BYTES {
        return Err("обложка слишком большая для превью".to_string());
    }

    std::fs::read(&path)
        .map(Response::new)
        .map_err(|error| error.to_string())
}
