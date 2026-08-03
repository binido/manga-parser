use crate::archive;
use crate::error::{Error, Result};
use crate::report::{Event, Reporter, ReporterExt};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    /// .zip с главами или папка, в которой они лежат.
    pub source: PathBuf,
    /// Куда положить результат. Пусто — рядом с источником, как в CLI-версии.
    #[serde(default)]
    pub destination: Option<PathBuf>,
    /// Необязательная обложка: становится первой страницей сборки.
    #[serde(default)]
    pub cover: Option<PathBuf>,
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub output: PathBuf,
    pub chapters: usize,
    pub images: usize,
}

pub fn run(job: &Job, reporter: &dyn Reporter) -> Result<Outcome> {
    let source = resolve_source(&job.source)?;
    let cover = resolve_cover(job.cover.as_deref())?;
    let output = resolve_output(job, &source)?;

    let workspace = Workspace::beside(&output)?;

    reporter.info("Ищу архивы глав…");
    let chapters = collect_chapters(&source, &workspace, reporter)?;
    if chapters.is_empty() {
        return Err(Error::NoChapters);
    }
    reporter.report(Event::ChaptersFound {
        total: chapters.len(),
    });

    recreate_dir(&output, reporter)?;

    let cover_pages = match &cover {
        Some(cover) => {
            place_cover(cover, &output)?;
            reporter.info(format!(
                "Обложка {} стала первой страницей.",
                file_name(cover)
            ));
            1
        }
        None => 0,
    };

    let mut pages = 0;
    for (position, chapter) in chapters.iter().enumerate() {
        if reporter.is_cancelled() {
            return Err(Error::Cancelled);
        }

        let name = file_name(chapter);
        let collected = match unpack_chapter(chapter, &workspace, &output, cover_pages + pages) {
            Ok(count) => count,
            Err(Error::Archive { .. }) => {
                reporter.warn(format!("Пропущена повреждённая глава: {name}"));
                0
            }
            Err(other) => return Err(other),
        };

        pages += collected;
        reporter.report(Event::ChapterDone {
            index: position + 1,
            total: chapters.len(),
            name,
            images: collected,
        });
    }

    // Одна обложка без единой страницы из глав — это не сборка.
    if pages == 0 {
        return Err(Error::NoImages);
    }

    let outcome = Outcome {
        output,
        chapters: chapters.len(),
        images: cover_pages + pages,
    };
    reporter.report(Event::Finished {
        chapters: outcome.chapters,
        images: outcome.images,
        output: outcome.output.clone(),
    });
    Ok(outcome)
}

fn resolve_source(source: &Path) -> Result<PathBuf> {
    let source = absolute(source)?;
    if !source.exists() {
        return Err(Error::SourceMissing(source));
    }
    if !source.is_dir() && !archive::is_zip(&source) {
        return Err(Error::UnsupportedSource);
    }
    Ok(source)
}

fn resolve_cover(cover: Option<&Path>) -> Result<Option<PathBuf>> {
    let Some(cover) = cover.filter(|path| !path.as_os_str().is_empty()) else {
        return Ok(None);
    };

    let cover = absolute(cover)?;
    if !cover.is_file() {
        return Err(Error::CoverMissing(cover));
    }
    if !archive::is_image(&cover) {
        return Err(Error::UnsupportedCover);
    }
    Ok(Some(cover))
}

fn resolve_output(job: &Job, source: &Path) -> Result<PathBuf> {
    let name = job.output_name.trim();
    if name.is_empty() || Path::new(name).components().count() != 1 {
        return Err(Error::InvalidOutputName);
    }

    let parent = match &job.destination {
        Some(destination) if !destination.as_os_str().is_empty() => absolute(destination)?,
        _ => source.parent().unwrap_or(source).to_owned(),
    };
    let output = parent.join(name);

    // Выходная папка очищается перед сборкой, поэтому она не должна оказаться
    // самим источником или папкой над ним.
    if source.starts_with(&output) {
        return Err(Error::InvalidOutputName);
    }
    Ok(output)
}

fn collect_chapters(
    source: &Path,
    workspace: &Workspace,
    reporter: &dyn Reporter,
) -> Result<Vec<PathBuf>> {
    if source.is_dir() {
        reporter.info("Источник — папка с архивами глав, распаковка не нужна.");
        return Ok(archive::find_archives(source));
    }

    reporter.info("Распаковываю главный архив…");
    let unpacked = workspace.scratch()?;
    archive::extract(source, &unpacked)?;
    Ok(archive::find_archives(&unpacked))
}

/// Возвращает количество перенесённых картинок; нумерация продолжается с `offset`.
fn unpack_chapter(
    chapter: &Path,
    workspace: &Workspace,
    output: &Path,
    offset: usize,
) -> Result<usize> {
    let unpacked = workspace.scratch()?;
    archive::extract(chapter, &unpacked)?;

    let images = archive::find_images(&unpacked);
    for (position, image) in images.iter().enumerate() {
        let extension = image
            .extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_default();
        let target = output.join(format!("{:05}{extension}", offset + position + 1));
        relocate(image, &target)?;
    }

    // Распакованная глава может весить сотни мегабайт, поэтому не ждём
    // конца сборки, а освобождаем место сразу.
    let _ = fs::remove_dir_all(&unpacked);

    Ok(images.len())
}

/// Обложка живёт вне сборки, поэтому её копируем, а не переносим.
fn place_cover(cover: &Path, output: &Path) -> Result<()> {
    let extension = cover
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    fs::copy(cover, output.join(format!("{:05}{extension}", 1)))?;
    Ok(())
}

fn recreate_dir(path: &Path, reporter: &dyn Reporter) -> Result<()> {
    if path.exists() {
        reporter.warn(format!(
            "Папка {} уже существует — очищаю.",
            file_name(path)
        ));
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

/// `rename` не работает между томами, поэтому на такой случай остаётся копия.
fn relocate(from: &Path, to: &Path) -> Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to)?;
    fs::remove_file(from)?;
    Ok(())
}

fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// Временные файлы держим на том же томе, что и результат: тогда картинки
/// переезжают переименованием, а не копированием.
struct Workspace {
    root: TempDir,
}

impl Workspace {
    fn beside(output: &Path) -> Result<Self> {
        let parent = output.parent().unwrap_or(output);
        fs::create_dir_all(parent)?;
        let root = tempfile::Builder::new()
            .prefix(".manga-parser-")
            .tempdir_in(parent)?;
        Ok(Self { root })
    }

    /// Пустая папка внутри рабочего каталога. Отдельного удаления не требует:
    /// весь каталог исчезает вместе с `Workspace`.
    fn scratch(&self) -> Result<PathBuf> {
        Ok(TempDir::new_in(self.root.path())?.keep())
    }
}
