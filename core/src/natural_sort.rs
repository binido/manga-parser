use std::cmp::Ordering;
use std::path::Path;

/// Кусок имени: число сравнивается как число, текст — без учёта регистра.
/// Порядок вариантов задаёт правило «числа раньше букв» при равных префиксах.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Chunk {
    Number(u128),
    Text(String),
}

/// Ключ сортировки, при котором «10» идёт после «9», а не после «1».
fn key(name: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut rest = name;

    while !rest.is_empty() {
        let digits = rest.starts_with(|c: char| c.is_ascii_digit());
        let end = rest
            .find(|c: char| c.is_ascii_digit() != digits)
            .unwrap_or(rest.len());
        let (head, tail) = rest.split_at(end);
        rest = tail;

        // Число длиннее u128 сравнивается как обычный текст — это заведомо
        // не номер главы, а мусор в имени файла.
        chunks.push(match head.parse::<u128>() {
            Ok(number) if digits => Chunk::Number(number),
            _ => Chunk::Text(head.to_lowercase()),
        });
    }

    chunks
}

/// Сравнивает пути покомпонентно, чтобы вложенные папки не перемешивались
/// между собой. На плоских архивах результат совпадает со сравнением по имени.
pub fn compare_paths(left: &Path, right: &Path) -> Ordering {
    let components = |path: &Path| {
        path.components()
            .map(|part| key(&part.as_os_str().to_string_lossy()))
            .collect::<Vec<_>>()
    };
    components(left).cmp(&components(right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sorted(names: &[&str]) -> Vec<String> {
        let mut paths: Vec<PathBuf> = names.iter().map(PathBuf::from).collect();
        paths.sort_by(|a, b| compare_paths(a, b));
        paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn numbers_are_ordered_by_value_not_by_digits() {
        assert_eq!(
            sorted(&["ch_10.zip", "ch_2.zip", "ch_1.zip"]),
            ["ch_1.zip", "ch_2.zip", "ch_10.zip"]
        );
    }

    #[test]
    fn leading_zeros_do_not_change_order() {
        assert_eq!(sorted(&["p003", "p1", "p02"]), ["p1", "p02", "p003"]);
    }

    #[test]
    fn case_is_ignored() {
        assert_eq!(sorted(&["Bravo", "alpha"]), ["alpha", "Bravo"]);
    }

    #[test]
    fn nested_folders_stay_grouped() {
        assert_eq!(
            sorted(&["vol2/01.jpg", "vol10/01.jpg", "vol2/10.jpg"]),
            ["vol2/01.jpg", "vol2/10.jpg", "vol10/01.jpg"]
        );
    }

    #[test]
    fn oversized_number_falls_back_to_text() {
        let huge = "9".repeat(64);
        assert_eq!(sorted(&[&huge, "1"]), ["1", &huge]);
    }
}
