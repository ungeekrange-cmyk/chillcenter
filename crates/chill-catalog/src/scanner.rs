use crate::model::{Media, now};
use regex::Regex;
use std::{path::Path, sync::OnceLock};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

pub fn normalized(value: &str) -> String {
    value
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

fn clean(value: &str) -> String {
    static RELEASE: OnceLock<Regex> = OnceLock::new();
    let release = RELEASE.get_or_init(|| Regex::new(r"(?i)\b(2160p|1080p|720p|480p|bluray|blu-ray|bdrip|brrip|webrip|web-dl|hdtv|dvdrip|x264|x265|h264|h265|hevc|hdr|dv|remux|multi|vff|vfq|vostfr|french|aac|ac3|dts|proper|repack)\b.*").unwrap());
    let spaced = value.replace(['.', '_'], " ");
    let without_release = release.replace(&spaced, "");
    without_release
        .trim_matches(|c: char| c.is_whitespace() || ['-', '[', ']', '(', ')'].contains(&c))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse_file(root: &Path, path: &Path) -> Media {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let directories = relative.parent().unwrap_or(Path::new("")).to_string_lossy();
    let folder = normalized(&directories);
    static EPISODIC: OnceLock<Regex> = OnceLock::new();
    static SEASON: OnceLock<Regex> = OnceLock::new();
    static EPISODE: OnceLock<Regex> = OnceLock::new();
    static YEAR: OnceLock<Regex> = OnceLock::new();
    let episodic = EPISODIC.get_or_init(|| {
        Regex::new(r"(?i)(?:s(\d{1,3})[ ._-]*e(\d{1,4})|(\d{1,3})x(\d{1,4}))").unwrap()
    });
    let season_folder =
        SEASON.get_or_init(|| Regex::new(r"(?i)(?:season|saison|s)[ ._-]*(\d{1,3})\b").unwrap());
    let episode_only =
        EPISODE.get_or_init(|| Regex::new(r"(?i)\b(?:e|ep|episode)[ ._-]*(\d{1,4})\b").unwrap());
    let year_re = YEAR.get_or_init(|| Regex::new(r"\b((?:19|20)\d{2})\b").unwrap());
    let searchable = stem.replace(['.', '_'], " ");
    let year_capture = year_re.captures_iter(&searchable).last();
    let year = year_capture.as_ref().and_then(|c| c[1].parse().ok());
    let ep = episodic.captures(&stem);
    let folder_season = season_folder
        .captures(&directories)
        .and_then(|c| c[1].parse::<i32>().ok());
    let only_ep = episode_only.captures(&searchable);
    let (season, episode, cut) = if let Some(c) = &ep {
        let a = c.get(1).or_else(|| c.get(3)).unwrap();
        let b = c.get(2).or_else(|| c.get(4)).unwrap();
        (
            a.as_str().parse().ok(),
            b.as_str().parse().ok(),
            c.get(0).map(|m| m.start()),
        )
    } else if let (Some(folder_season), Some(c)) = (folder_season, only_ep.as_ref()) {
        (
            Some(folder_season),
            c[1].parse().ok(),
            c.get(0).map(|m| m.start()),
        )
    } else {
        (None, None, None)
    };
    let category = if episode.is_some() || folder.contains("serie") || folder.contains("tv show") {
        "serie"
    } else if folder.contains("document") || folder.contains("docu") {
        "documentaire"
    } else if folder.contains("spectacle")
        || folder.contains("concert")
        || folder.contains("stand-up")
        || folder.contains("standup")
    {
        "spectacle"
    } else {
        "film"
    };
    let without_year = if let Some(c) = year_capture.as_ref().and_then(|c| c.get(0)) {
        format!("{}{}", &searchable[..c.start()], &searchable[c.end()..])
    } else {
        searchable.clone()
    };
    let mut title = clean(&without_year);
    let mut series = None;
    if category == "serie" {
        let prefix = cut.map(|n| clean(&stem[..n])).unwrap_or_default();
        let from_folder = relative
            .parent()
            .into_iter()
            .flat_map(|p| p.ancestors())
            .filter_map(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .find(|s| {
                let n = normalized(s);
                !n.is_empty()
                    && !["series", "serie", "tv", "tv shows"].contains(&n.as_str())
                    && !season_folder.is_match(s)
            });
        series = Some(if !prefix.is_empty() {
            clean(&year_re.replace(&prefix, ""))
        } else {
            from_folder
                .map(|s| clean(&s))
                .unwrap_or_else(|| title.clone())
        });
        if let Some(c) = ep
            .as_ref()
            .and_then(|c| c.get(0))
            .or_else(|| only_ep.as_ref().and_then(|c| c.get(0)))
        {
            let suffix = clean(&stem[c.end()..]);
            title = if suffix.is_empty() {
                format!("Épisode {}", episode.unwrap_or(0))
            } else {
                suffix
            };
        }
    }
    if title.is_empty() {
        title = stem.into_owned();
    }
    Media {
        id: relative.to_string_lossy().into_owned(),
        path: path.to_string_lossy().into_owned(),
        title,
        category: category.into(),
        year,
        series,
        season,
        episode,
        genres: vec![],
        overview: String::new(),
        poster: None,
        poster_custom: false,
        runtime_minutes: None,
        cast: vec![],
        directors: vec![],
        series_overview: String::new(),
        series_metadata: None,
        series_manual: false,
        rating: None,
        saga: None,
        watched: false,
        available: true,
        added_at: now(),
        tmdb_id: None,
    }
}

/// Complete read-only snapshot. Any directory/file I/O error aborts the snapshot.
pub fn discover(root: &Path) -> Result<Vec<Media>, String> {
    discover_with_budget(root, MAX_SCAN_BYTES - 1024)
}

pub const MAX_SCAN_BYTES: usize = 64 * 1024 * 1024;
pub fn discover_with_budget(root: &Path, budget: usize) -> Result<Vec<Media>, String> {
    if !root.is_absolute() {
        return Err("Le dossier doit être un chemin absolu.".into());
    }
    let root = root
        .canonicalize()
        .map_err(|_| "Dossier NAS inaccessible. Le catalogue précédent est conservé.")?;
    if !root.is_dir() {
        return Err("Le chemin choisi n’est pas un dossier.".into());
    }
    let mut files = Vec::new();
    let mut serialized_bytes = 0usize;
    let mut pending = vec![(root.clone(), 0)];
    while let Some((directory, depth)) = pending.pop() {
        if depth > 128 {
            return Err("Arborescence trop profonde. Scan annulé.".into());
        }
        let entries = std::fs::read_dir(&directory)
            .map_err(|_| "Un dossier est inaccessible. Scan annulé ; catalogue conservé.")?;
        for entry in entries {
            let entry =
                entry.map_err(|_| "Erreur pendant la lecture du NAS. Catalogue conservé.")?;
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let kind = entry
                .file_type()
                .map_err(|_| "Fichier inaccessible pendant le scan. Catalogue conservé.")?;
            // Never follow symlinks outside the library or through cycles.
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                pending.push((path, depth + 1));
            } else if kind.is_file()
                && path.extension().is_some_and(|e| {
                    [
                        "mkv", "mp4", "avi", "mov", "m4v", "wmv", "mpg", "mpeg", "ts", "m2ts",
                        "webm", "vob", "ogv",
                    ]
                    .contains(&e.to_string_lossy().to_lowercase().as_str())
                })
            {
                entry
                    .metadata()
                    .map_err(|_| "Une vidéo est devenue inaccessible. Catalogue conservé.")?;
                let media = parse_file(&root, &path);
                serialized_bytes += serde_json::to_vec(&media)
                    .map_err(|_| "Nom de fichier invalide.")?
                    .len()
                    + 1;
                if serialized_bytes > budget {
                    return Err("Catalogue trop volumineux pour ce scan (64 Mo). Le catalogue précédent est conservé.".into());
                }
                files.push(media);
                if files.len() > 200000 {
                    return Err("Plus de 200 000 vidéos : scan annulé.".into());
                }
            }
        }
    }
    std::fs::read_dir(&root).map_err(|_| "Le NAS a été déconnecté. Catalogue conservé.")?;
    files.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movie_year_and_release() {
        let m = parse_file(
            Path::new("/nas"),
            Path::new("/nas/Films/Alien.1979.1080p.BluRay.mkv"),
        );
        assert_eq!(m.title, "Alien");
        assert_eq!(m.year, Some(1979));
        assert_eq!(m.category, "film");
    }
    #[test]
    fn year_in_movie_title_is_preserved() {
        let m = parse_file(
            Path::new("/nas"),
            Path::new("/nas/2001.A.Space.Odyssey.1968.1080p.mkv"),
        );
        assert_eq!(m.title, "2001 A Space Odyssey");
        assert_eq!(m.year, Some(1968));
    }
    #[test]
    fn seasons_and_episodes() {
        for name in ["Dark.S02E03.1080p.mkv", "Dark.2x03.mkv"] {
            let m = parse_file(Path::new("/nas"), &Path::new("/nas/Séries").join(name));
            assert_eq!(m.series.as_deref(), Some("Dark"));
            assert_eq!(m.season, Some(2));
            assert_eq!(m.episode, Some(3));
        }
    }
    #[test]
    fn episode_in_season_folder() {
        let m = parse_file(
            Path::new("/nas"),
            Path::new("/nas/Séries/Severance/Saison 01/E02.Half.Loop.mkv"),
        );
        assert_eq!(m.series.as_deref(), Some("Severance"));
        assert_eq!(m.title, "Half Loop");
        assert_eq!(m.season, Some(1));
    }
    #[test]
    fn categories_and_accents() {
        assert_eq!(
            parse_file(
                Path::new("/nas"),
                Path::new("/nas/Documentaires/Oceans.mp4")
            )
            .category,
            "documentaire"
        );
        assert_eq!(
            parse_file(Path::new("/nas"), Path::new("/nas/Spectacles/Concert.mp4")).category,
            "spectacle"
        );
        assert_eq!(normalized("SÉRIES"), "series");
    }
    #[test]
    fn missing_root_is_an_error() {
        assert!(discover(Path::new("/not/a/real/nas")).is_err());
    }
    #[test]
    fn recursive_scan_ignores_non_video_and_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("Films")).unwrap();
        std::fs::write(temp.path().join("Films/Alien.1979.mkv"), b"").unwrap();
        std::fs::write(temp.path().join("note.txt"), b"").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(temp.path(), temp.path().join("loop")).unwrap();
        assert_eq!(discover(temp.path()).unwrap().len(), 1);
    }
}
