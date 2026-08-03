use manga_parser_core::{run, Error, Job};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;

fn write_zip(path: &Path, entries: &[&str]) {
    let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
    for entry in entries {
        zip.start_file(*entry, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(entry.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

fn page_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn chapters_are_flattened_into_continuous_page_numbers() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("chapters");
    fs::create_dir(&source).unwrap();

    write_zip(&source.join("ch_10.zip"), &["1.jpg"]);
    write_zip(&source.join("ch_2.zip"), &["1.png", "10.png", "2.png"]);
    write_zip(&source.join("ch_1.zip"), &["cover.jpg", "notes.txt"]);

    let job = Job {
        source: source.clone(),
        destination: None,
        cover: None,
        output_name: "ready".into(),
    };
    let outcome = run(&job, &()).unwrap();

    assert_eq!(outcome.chapters, 3);
    assert_eq!(outcome.images, 5);
    assert_eq!(
        page_names(&outcome.output),
        [
            "00001.jpg", // ch_1
            "00002.png",
            "00003.png",
            "00004.png", // ch_2, натуральный порядок
            "00005.jpg", // ch_10 идёт последней, а не второй
        ]
    );
    assert!(outcome.output.starts_with(root.path()));
}

#[test]
fn nested_archive_is_unpacked_before_chapters() {
    let root = tempfile::tempdir().unwrap();
    let inner = root.path().join("inner");
    fs::create_dir(&inner).unwrap();
    write_zip(&inner.join("ch_1.zip"), &["a.webp"]);

    let bundle = root.path().join("manga.zip");
    {
        let mut zip = zip::ZipWriter::new(File::create(&bundle).unwrap());
        zip.start_file("ch_1.zip", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&fs::read(inner.join("ch_1.zip")).unwrap())
            .unwrap();
        zip.finish().unwrap();
    }

    let job = Job {
        source: bundle,
        destination: None,
        cover: None,
        output_name: "ready".into(),
    };
    let outcome = run(&job, &()).unwrap();

    assert_eq!(page_names(&outcome.output), ["00001.webp"]);
}

#[test]
fn broken_chapter_is_skipped_not_fatal() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("chapters");
    fs::create_dir(&source).unwrap();

    write_zip(&source.join("ch_1.zip"), &["a.jpg"]);
    fs::write(source.join("ch_2.zip"), b"this is not a zip").unwrap();

    let job = Job {
        source,
        destination: None,
        cover: None,
        output_name: "ready".into(),
    };
    let outcome = run(&job, &()).unwrap();

    assert_eq!(outcome.images, 1);
}

#[test]
fn cover_becomes_the_first_page_and_shifts_the_rest() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("chapters");
    fs::create_dir(&source).unwrap();
    write_zip(&source.join("ch_1.zip"), &["a.jpg", "b.jpg"]);

    let cover = root.path().join("cover.png");
    fs::write(&cover, b"cover bytes").unwrap();

    let job = Job {
        source,
        destination: None,
        cover: Some(cover.clone()),
        output_name: "ready".into(),
    };
    let outcome = run(&job, &()).unwrap();

    assert_eq!(outcome.images, 3);
    assert_eq!(
        page_names(&outcome.output),
        ["00001.png", "00002.jpg", "00003.jpg"]
    );
    assert_eq!(
        fs::read(outcome.output.join("00001.png")).unwrap(),
        b"cover bytes"
    );
    // Обложка остаётся у пользователя: её копируют, а не забирают.
    assert!(cover.exists());
}

#[test]
fn cover_must_be_an_existing_image() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("chapters");
    fs::create_dir(&source).unwrap();
    write_zip(&source.join("ch_1.zip"), &["a.jpg"]);

    let notes = root.path().join("notes.txt");
    fs::write(&notes, b"not an image").unwrap();

    let job = |cover| Job {
        source: source.clone(),
        destination: None,
        cover: Some(cover),
        output_name: "ready".into(),
    };

    assert!(matches!(
        run(&job(notes), &()).unwrap_err(),
        Error::UnsupportedCover
    ));
    assert!(matches!(
        run(&job(root.path().join("ghost.png")), &()).unwrap_err(),
        Error::CoverMissing(_)
    ));
    // Проверка идёт до очистки, поэтому папку результата ещё не создавали.
    assert!(!root.path().join("ready").exists());
}

#[test]
fn output_folder_cannot_swallow_the_source() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("chapters");
    fs::create_dir(&source).unwrap();
    write_zip(&source.join("ch_1.zip"), &["a.jpg"]);

    let job = Job {
        source: source.clone(),
        destination: Some(root.path().to_owned()),
        cover: None,
        output_name: "chapters".into(),
    };

    assert!(matches!(
        run(&job, &()).unwrap_err(),
        Error::InvalidOutputName
    ));
    assert!(source.join("ch_1.zip").exists());
}

#[test]
fn existing_output_is_cleared_before_assembly() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("chapters");
    fs::create_dir(&source).unwrap();
    write_zip(&source.join("ch_1.zip"), &["a.jpg"]);

    let output = root.path().join("ready");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("stale.jpg"), b"old").unwrap();

    let job = Job {
        source,
        destination: None,
        cover: None,
        output_name: "ready".into(),
    };
    run(&job, &()).unwrap();

    assert_eq!(page_names(&output), ["00001.jpg"]);
}
