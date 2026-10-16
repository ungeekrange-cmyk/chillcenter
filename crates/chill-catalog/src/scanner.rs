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

fn year_pattern() -> &'static Regex {
    static YEAR: OnceLock<Regex> = OnceLock::new();
    YEAR.get_or_init(|| Regex::new(r"\b((?:18|19|20)\d{2})\b").unwrap())
}

fn is_dash(c: char) -> bool {
    "-‐‑‒–—−".contains(c)
}

/// Stop before an audio/video section, not just its first recognised codec.
/// Language words are weaker evidence: "La French" is a legitimate title.
fn release_prefix(value: &str) -> &str {
    static TECHNICAL: OnceLock<Regex> = OnceLock::new();
    static LABEL: OnceLock<Regex> = OnceLock::new();
    static LANGUAGE: OnceLock<Regex> = OnceLock::new();
    let technical=TECHNICAL.get_or_init(||Regex::new(r"(?i)\b(?:2160p|1080[pi]|720p|576p|480p|4k|uhd|blu[ ._-]?ray|bdremux|bdrip|brrip|web[ ._-]?(?:rip|dl)|hdtv|dvdrip|[xh][ ._-]?26[45]|hevc|avc|av1|xvid|divx|remux|hdr10?\+?|vff|vfq|vfi|vost(?:fr|a|en)?|aac|e[ ._-]?ac[ ._-]?3|ac[ ._-]?3|dts|true[ ._-]?hd|atmos|flac|opus|proper|repack)\b|\bDDP(?:[ ._-]*[257](?:[ ._-][01])?)?\b|\bDD\+").unwrap());
    let label=LABEL.get_or_init(||Regex::new(r"(?i)\b(?:audio|vid[eé]o)[\s._:=\-–—\[\(]+(?:fr(?:an[cç]ais|ench)?|en(?:g(?:lish)?)?|vo(?:st(?:fr|a|en)?)?|vf[fiq]?|multi|ddp|dd|dts|aac|ac3|eac3|truehd|hevc|avc|av1|[xh][ ._-]?26[45]|1080[pi]|2160p|720p|4k|full[ ._-]?hd)\b").unwrap());
    let language =
        LANGUAGE.get_or_init(|| Regex::new(r"(?i)\b(?:french|english|multi|vf|vo|dv)\b").unwrap());
    let strong = technical
        .find_iter(value)
        .chain(label.find_iter(value))
        .map(|m| m.start());
    let weak = language
        .find_iter(value)
        .filter(|m| {
            let prefix = value[..m.start()].trim_end();
            prefix
                .chars()
                .last()
                .is_some_and(|c| is_dash(c) || c == '[' || c == '(')
                || year_pattern().is_match(prefix)
        })
        .map(|m| m.start());
    let cut = strong
        .chain(weak)
        .filter(|&at| value[..at].chars().any(char::is_alphanumeric))
        .min()
        .unwrap_or(value.len());
    &value[..cut]
}

fn tidy(value: &str) -> String {
    static EMPTY: OnceLock<Regex> = OnceLock::new();
    static DASHES: OnceLock<Regex> = OnceLock::new();
    let empty = EMPTY.get_or_init(|| Regex::new(r"\(\s*\)|\[\s*\]").unwrap());
    let dashes = DASHES.get_or_init(|| Regex::new(r"[-‐‑‒–—−](?:\s*[-‐‑‒–—−])+").unwrap());
    let spaced = value.replace(['.', '_'], " ");
    let emptied = empty.replace_all(&spaced, " ");
    let separated = dashes.replace_all(&emptied, " ");
    let trimmed = separated
        .trim_start_matches(|c: char| c.is_whitespace() || is_dash(c) || c == ']' || c == ')')
        .trim_end_matches(|c: char| c.is_whitespace() || is_dash(c) || c == '[' || c == '(')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    discard_orphaned_delimiters(&trimmed)
}

/// Keep complete title parentheticals, but remove delimiter characters that
/// have no partner. Older indexed labels can contain an opening parenthesis
/// left after a release suffix was cut, which otherwise leaks into TMDB query.
fn discard_orphaned_delimiters(value: &str) -> String {
    let mut result = Vec::with_capacity(value.chars().count());
    let mut openings = Vec::new();
    for c in value.chars() {
        match c {
            '(' | '[' => {
                openings.push((c, result.len()));
                result.push(Some(c));
            }
            ')' | ']' => {
                let expected = if c == ')' { '(' } else { '[' };
                if openings.last().is_some_and(|(open, _)| *open == expected) {
                    openings.pop();
                    result.push(Some(c));
                } else {
                    result.push(None);
                }
            }
            _ => result.push(Some(c)),
        }
    }
    for (_, index) in openings {
        result[index] = None;
    }
    result.into_iter().flatten().collect()
}

/// Clean technical release suffixes from a label without treating numbers in an
/// already indexed title (e.g. Blade Runner 2049) as release years.
pub fn title_for_search(value: &str) -> String {
    let spaced = value.replace(['.', '_'], " ");
    tidy(release_prefix(&spaced))
}

/// A narrower second query for source labels which use repeated dashes as
/// release-field separators.  It is deliberately not applied to the first
/// lookup: a single dash can be part of a real title (for example
/// "Spider-Man"), and complete parentheticals remain meaningful title text.
///
/// The caller may use this only after the normal query has no results.
pub fn title_for_search_fallback(value: &str) -> Option<String> {
    static REPEATED_DASH: OnceLock<Regex> = OnceLock::new();
    // `--` and `- -` are both common release-field separators. A single
    // spaced dash remains a title character, so it cannot trigger a retry.
    let separator = REPEATED_DASH.get_or_init(|| Regex::new(r"--+|-\s+-").unwrap());
    let primary = title_for_search(value);
    for found in separator.find_iter(value) {
        let prefix = value[..found.start()].trim();
        let tail = &value[found.end()..];
        let next = separator
            .find(tail)
            .map(|next| &tail[..next.start()])
            .unwrap_or(tail)
            .trim();
        if prefix.is_empty() || !release_field(next) {
            continue;
        }
        let fallback = tidy(prefix);
        if !fallback.is_empty() && fallback != primary {
            return Some(fallback);
        }
    }
    None
}

/// A repeated dash alone is too weak: it may be intentional punctuation in a
/// title. The immediately following field must look like a release year or a
/// media/distribution marker before the fallback is considered.
fn release_field(value: &str) -> bool {
    static FIELD: OnceLock<Regex> = OnceLock::new();
    static YEAR: OnceLock<Regex> = OnceLock::new();
    let field = FIELD.get_or_init(|| {
        Regex::new(r"(?i)^(?:audio|vid[eé]o|vost(?:fr|a|en)?|vff|vfq|vfi|vo|multi|(?:e[ ._-]?ac|ac)[ ._-]?3|ddp|dd\+|dts|true[ ._-]?hd|atmos|flac|opus|[xh][ ._-]?26[45]|hevc|avc|av1|xvid|divx|(?:2160|1080|720|576|480)[pi]?|4k|uhd|blu[ ._-]?ray|web[ ._-]?(?:rip|dl)|hdtv|dvdrip|remux)\b").unwrap()
    });
    let year = YEAR.get_or_init(|| Regex::new(r"^[\[(]?(?:18|19|20)\d{2}[\])]?$").unwrap());
    // The explicit wrapper anchors a year field; an unwrapped year must be a
    // whole field to avoid treating a title number as release metadata.
    field.is_match(value) || year.is_match(value)
}

/// Extract a release year only when parsing source filenames, not stored titles.
fn title_and_year(value: &str) -> (String, Option<i32>) {
    // Keep source punctuation separately for year disambiguation while using
    // the space-normalised form for release-token detection (an underscore is
    // a regex word character and would otherwise hide VOST/DDP markers).
    let spaced = value.replace(['.', '_'], " ");
    let content = release_prefix(&spaced).to_owned();
    // A leading number may be the actual title (1917, 2001, 1899), not a date.
    let year = year_pattern()
        .captures_iter(&content)
        .filter(|c| {
            let year = c.get(0).unwrap();
            let before = value[..year.start()].chars().last();
            let after = value[year.end()..].chars().next();
            let before_marker = value[..year.start()]
                .chars()
                .rev()
                .find(|c| !c.is_whitespace());
            let after_marker = value[year.end()..]
                .chars()
                .find(|c| !c.is_whitespace());
            // A plain terminal number is frequently part of a title (Blade
            // Runner 2049).  Treat a four-digit number as a release year only
            // when the filename marks it with a release delimiter or wrapper.
            let marked = matches!((before, after), (Some('('), Some(')')) | (Some('['), Some(']')))
                || before_marker.is_some_and(|c| matches!(c, '.' | '_' | '-'))
                    && after_marker.is_some_and(|c| matches!(c, '.' | '_' | '-'));
            marked
                && content[..year.start()]
                    .chars()
                    .any(char::is_alphanumeric)
        })
        .last();
    let (without_year, date) = if let Some(c) = year {
        let matched = c.get(0).unwrap();
        (
            format!(
                "{}{}",
                &content[..matched.start()],
                &content[matched.end()..]
            ),
            c[1].parse().ok(),
        )
    } else {
        (content, None)
    };
    let title = tidy(&without_year);
    (
        if title.is_empty() {
            value.to_owned()
        } else {
            title
        },
        date,
    )
}

pub fn parse_file(root: &Path, path: &Path) -> Media {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let directories = relative.parent().unwrap_or(Path::new("")).to_string_lossy();
    let folder = normalized(&directories);
    static EPISODIC: OnceLock<Regex> = OnceLock::new();
    static SEASON: OnceLock<Regex> = OnceLock::new();
    static EPISODE: OnceLock<Regex> = OnceLock::new();
    let episodic = EPISODIC.get_or_init(|| {
        Regex::new(r"(?i)(?:s(\d{1,3})[ ._-]*e(\d{1,4})|(\d{1,3})x(\d{1,4}))").unwrap()
    });
    let season_folder =
        SEASON.get_or_init(|| Regex::new(r"(?i)(?:season|saison|s)[ ._-]*(\d{1,3})\b").unwrap());
    let episode_only =
        EPISODE.get_or_init(|| Regex::new(r"(?i)\b(?:e|ep|episode)[ ._-]*(\d{1,4})\b").unwrap());
    let searchable = stem.replace(['.', '_'], " ");
    let (mut title, year) = title_and_year(&stem);
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
    let mut series = None;
    if category == "serie" {
        let prefix = cut
            .map(|n| title_for_search(&stem[..n]))
            .unwrap_or_default();
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
            title_and_year(&prefix).0
        } else {
            from_folder
                .map(|s| title_for_search(&s))
                .unwrap_or_else(|| title.clone())
        });
        if let Some(c) = ep
            .as_ref()
            .and_then(|c| c.get(0))
            .or_else(|| only_ep.as_ref().and_then(|c| c.get(0)))
        {
            let suffix = title_for_search(&stem[c.end()..]);
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
    fn release_separator_audio_video_and_empty_year_wrappers() {
        for (name, title, year) in [
            ("Attack the Block - - Vosta.mkv", "Attack the Block", None),
            ("Birdman () - vost DDP 5 1.mkv", "Birdman", None),
            ("Birdman_(2014)_--_VOST_DDP_5_1.mkv", "Birdman", Some(2014)),
            (
                "Attack the Block - - (2011) - - VOSTA - - 1080p.mkv",
                "Attack the Block",
                Some(2011),
            ),
            (
                "Birdman (2014) -- [Audio: VOST DDP 5.1] -- [Video: H.264 1080p].mp4",
                "Birdman",
                Some(2014),
            ),
            (
                "12 hommes en colère - - (1957) - Audio - FR DTS-HD MA 5.1 - Vidéo - H264.mkv",
                "12 hommes en colère",
                Some(1957),
            ),
            (
                "(500) jours ensemble -- 2009 -- [Audio: ENG E-AC-3 5.1] -- [Video: HEVC].mkv",
                "(500) jours ensemble",
                Some(2009),
            ),
            (
                "2001.A.Space.Odyssey.(1968) -- Video - x265 - Encoded 2020.mkv",
                "2001 A Space Odyssey",
                Some(1968),
            ),
            (
                "Wall-E -- (2008) -- Audio: VFF DD+ 5.1 -- Video: AV1 10bit.avi",
                "Wall-E",
                Some(2008),
            ),
        ] {
            let m = parse_file(Path::new("/nas"), &Path::new("/nas").join(name));
            assert_eq!(m.title, title, "{name}");
            assert_eq!(m.year, year, "{name}");
            assert_eq!(m.id, name, "parsing must not rename the source");
        }
    }
    #[test]
    fn release_cleaning_preserves_title_hyphens_parentheses_and_language_words() {
        for (name, title) in [
            (
                "Spider-Man - No Way Home (2021) - 1080p.mkv",
                "Spider-Man - No Way Home",
            ),
            ("La French (2014) - - 1080p.mkv", "La French"),
            ("French Kiss (1995) -- AAC.mkv", "French Kiss"),
            (
                "Birdman (or The Unexpected Virtue of Ignorance) (2014).mkv",
                "Birdman (or The Unexpected Virtue of Ignorance)",
            ),
            ("1917.mkv", "1917"),
            ("2001.A.Space.Odyssey.mkv", "2001 A Space Odyssey"),
        ] {
            let m = parse_file(Path::new("/nas"), &Path::new("/nas").join(name));
            assert_eq!(m.title, title, "{name}");
        }
    }
    #[test]
    fn fallback_query_only_uses_repeated_release_separators() {
        assert_eq!(
            title_for_search_fallback("Attack the Block - - (2011) - - Audio - VOSTA"),
            Some("Attack the Block".into())
        );
        assert_eq!(
            title_for_search_fallback("Attack the Block--2011--VOSTA--1080p"),
            Some("Attack the Block".into())
        );
        assert_eq!(
            title_for_search_fallback("Spider-Man - No Way Home (2021) - 1080p"),
            None,
            "single title dashes must not trigger the retry"
        );
        assert_eq!(
            title_for_search_fallback("Birdman (or The Unexpected Virtue of Ignorance)"),
            None
        );
        assert_eq!(
            title_for_search_fallback("Wallace -- Gromit"),
            None,
            "repeated title punctuation without a release field is not enough"
        );
    }
    #[test]
    fn search_cleaning_ignores_orphaned_delimiters_but_keeps_complete_parentheticals() {
        assert_eq!(title_for_search("Baby (Part 1"), "Baby Part 1");
        assert_eq!(
            title_for_search("Un Ange-Grenouille (Inédit"),
            "Un Ange-Grenouille Inédit"
        );
        assert_eq!(
            title_for_search("Birdman (or The Unexpected Virtue of Ignorance)"),
            "Birdman (or The Unexpected Virtue of Ignorance)"
        );
        let orphaned_source = parse_file(
            Path::new("/nas"),
            Path::new("/nas/Un Ange-Grenouille (Inédit).avi"),
        );
        assert_eq!(orphaned_source.title, "Un Ange-Grenouille (Inédit)");
        let malformed_source = parse_file(
            Path::new("/nas"),
            Path::new("/nas/Un Ange-Grenouille (Inédit.avi"),
        );
        assert_eq!(malformed_source.title, "Un Ange-Grenouille Inédit");
        let episode = parse_file(
            Path::new("/nas"),
            Path::new(
                "/nas/Séries/Malcolm in the Middle (2000) - S04E20 - Baby (Part 1) (1080p AMZN WEB-DL x265 Silence).mkv",
            ),
        );
        assert_eq!(episode.series.as_deref(), Some("Malcolm in the Middle"));
        assert_eq!(episode.title, "Baby (Part 1)");
    }
    #[test]
    fn release_cleaning_keeps_series_numbering_and_episode_titles() {
        let m = parse_file(
            Path::new("/nas"),
            Path::new(
                "/nas/Séries/Dark - - S02E03 - - Ghosts -- Audio - VOSTA DDP 5 1 -- Video - x265.mkv",
            ),
        );
        assert_eq!(m.series.as_deref(), Some("Dark"));
        assert_eq!(m.title, "Ghosts");
        assert_eq!((m.season, m.episode), (Some(2), Some(3)));
    }
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
    fn plain_terminal_title_numbers_are_not_mistaken_for_years() {
        let m = parse_file(
            Path::new("/nas"),
            Path::new("/nas/Blade Runner 2049 - - VOSTA - - 1080p.mkv"),
        );
        assert_eq!(m.title, "Blade Runner 2049");
        assert_eq!(m.year, None);
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
