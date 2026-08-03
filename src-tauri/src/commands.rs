use crate::session::Session;
use manga_parser_core::{Job, Outcome};
use tauri::{AppHandle, State};

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
