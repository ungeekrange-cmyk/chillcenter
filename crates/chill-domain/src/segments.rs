use crate::Store;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    Intro,
    Recap,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub kind: SegmentKind,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentScope {
    Episode,
    Season,
}

const INVALID: &str = "PLAYBACK_SEGMENTS_INVALID";

pub fn valid_segments(segments: &[Segment]) -> bool {
    segments.len() <= 2
        && segments
            .iter()
            .all(|item| item.start_ms < item.end_ms && item.end_ms <= 86_400_000)
        && (segments.len() < 2
            || (segments[0].kind != segments[1].kind
                && (segments[0].end_ms <= segments[1].start_ms
                    || segments[1].end_ms <= segments[0].start_ms)))
}

impl Store {
    fn segment_owner(
        &mut self,
        profile: &str,
        media: &str,
        scope: SegmentScope,
    ) -> Result<String, String> {
        let item = self.media(profile, media)?;
        if item.category != "serie" {
            return Err(INVALID.into());
        }
        match scope {
            SegmentScope::Episode => Ok(format!("episode:{media}")),
            SegmentScope::Season => {
                let season = item.season.ok_or(INVALID)?;
                Ok(format!("season:{}:{season}", self.series_identity(media)?))
            }
        }
    }

    pub fn manual_segments(
        &mut self,
        profile: &str,
        media: &str,
        scope: SegmentScope,
        version: &str,
    ) -> Result<(Vec<Segment>, u64), String> {
        let owner = self.segment_owner(profile, media, scope)?;
        let record = self
            .personal
            .query_row(
                "SELECT version,data,revision FROM cc_playback_segments WHERE owner=?1",
                [owner],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| INVALID)?;
        let Some((stored_version, data, revision)) = record else {
            return Ok((Vec::new(), 0));
        };
        let revision = u64::try_from(revision).map_err(|_| INVALID)?;
        if matches!(scope, SegmentScope::Episode) && stored_version != version {
            return Ok((Vec::new(), revision));
        }
        Ok((serde_json::from_str(&data).map_err(|_| INVALID)?, revision))
    }

    pub fn save_manual_segments(
        &mut self,
        profile: &str,
        media: &str,
        scope: SegmentScope,
        edit: SegmentEdit,
    ) -> Result<(), String> {
        let revision = i64::try_from(edit.expected_revision).map_err(|_| INVALID)?;
        if !valid_segments(&edit.segments) {
            return Err(INVALID.into());
        }
        let owner = self.segment_owner(profile, media, scope)?;
        let data = serde_json::to_string(&edit.segments).map_err(|_| INVALID)?;
        let changed = self.personal.execute(
            "INSERT INTO cc_playback_segments(owner,version,data,revision)
             SELECT ?1,?2,?3,1 WHERE ?4=0 OR EXISTS(SELECT 1 FROM cc_playback_segments WHERE owner=?1 AND revision=?4)
             ON CONFLICT(owner) DO UPDATE SET version=excluded.version,data=excluded.data,revision=cc_playback_segments.revision+1
             WHERE cc_playback_segments.revision=?4",
            params![owner, edit.version, data, revision],
        ).map_err(|_| INVALID)?;
        if changed == 0 {
            return Err("PLAYBACK_SEGMENTS_CHANGED".into());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentEdit {
    pub version: String,
    pub expected_revision: u64,
    pub segments: Vec<Segment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_negative_length_duplicate_kinds_and_overlap() {
        let intro = Segment {
            kind: SegmentKind::Intro,
            start_ms: 500,
            end_ms: 1000,
        };
        let recap = Segment {
            kind: SegmentKind::Recap,
            start_ms: 0,
            end_ms: 500,
        };
        assert!(valid_segments(&[intro.clone(), recap.clone()]));
        assert!(!valid_segments(&[intro.clone(), intro.clone()]));
        assert!(!valid_segments(&[
            intro.clone(),
            Segment {
                end_ms: 501,
                ..recap
            }
        ]));
        assert!(!valid_segments(&[Segment {
            start_ms: 1000,
            ..intro
        }]));
    }
}
