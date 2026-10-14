//! Provider-update boundary.
//!
//! TMDB is fetched outside SQLite, but the snapshot and final write remain
//! compare-and-swap guarded.  This module never serializes storage paths: the
//! normal `profile_view` projection is used for every returned media item.
use crate::Store;
use chill_catalog::model::{Media, SeriesProvider};
use rusqlite::{OptionalExtension, params};

#[derive(Clone)]
pub struct ProviderSnapshot {
    pub media: Media,
    pub revision: u64,
    pub common_revision: u64,
}

#[derive(Clone)]
pub struct SeriesProviderSnapshot {
    pub series: String,
    pub members: Vec<Media>,
    pub anchor_revision: u64,
    pub common_revision: u64,
}

impl Store {
    /// Reads the raw provider record only for a short-lived server operation.
    /// Callers must return the result through `provider_apply_media`, which
    /// projects it to the safe profile DTO.
    pub fn provider_snapshot(
        &mut self,
        profile: &str,
        stable: &str,
        expected_revision: u64,
    ) -> Result<ProviderSnapshot, String> {
        self.require_profile(profile)?;
        let legacy = self.legacy_id(stable)?;
        let revision = self.media_revision(stable)?;
        if revision != expected_revision {
            return Err("La fiche a changé entre-temps.".into());
        }
        Ok(ProviderSnapshot {
            media: self.catalogue.get(&legacy)?,
            revision,
            common_revision: self.catalog_revision()?,
        })
    }

    pub fn provider_series_snapshot(
        &mut self,
        profile: &str,
        stable: &str,
        expected_revision: u64,
    ) -> Result<SeriesProviderSnapshot, String> {
        let anchor = self.provider_snapshot(profile, stable, expected_revision)?;
        if anchor.media.category != "serie" {
            return Err("Cette fiche n’est pas un épisode de série.".into());
        }
        let series = anchor
            .media
            .series
            .clone()
            .unwrap_or_else(|| anchor.media.title.clone());
        Ok(SeriesProviderSnapshot {
            members: self.catalogue.series_members(&series)?,
            series,
            anchor_revision: anchor.revision,
            common_revision: anchor.common_revision,
        })
    }

    fn assert_provider_cas(
        &self,
        stable: &str,
        expected_revision: u64,
        expected_common: u64,
    ) -> Result<(), String> {
        if self.media_revision(stable)? != expected_revision
            || self.catalog_revision()? != expected_common
        {
            return Err("La fiche ou la série a changé pendant la requête. Vos corrections sont conservées ; relancez l’opération.".into());
        }
        Ok(())
    }

    pub fn provider_apply_media(
        &mut self,
        profile: &str,
        stable: &str,
        snapshot: &ProviderSnapshot,
        downloaded: Media,
        explicit: bool,
    ) -> Result<(Media, u64), String> {
        self.require_profile(profile)?;
        self.assert_provider_cas(stable, snapshot.revision, snapshot.common_revision)?;
        let updated = self
            .catalogue
            .apply_provider(&snapshot.media, downloaded, explicit)?;
        self.personal
            .execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                [stable],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        Ok((self.profile_view(profile, updated)?, snapshot.revision + 1))
    }

    pub fn provider_restore_poster(
        &mut self,
        profile: &str,
        stable: &str,
        snapshot: &ProviderSnapshot,
        poster: String,
    ) -> Result<(Media, u64), String> {
        self.require_profile(profile)?;
        self.assert_provider_cas(stable, snapshot.revision, snapshot.common_revision)?;
        let updated = self
            .catalogue
            .restore_poster(&snapshot.media, Some(poster))?;
        self.personal
            .execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                [stable],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        Ok((self.profile_view(profile, updated)?, snapshot.revision + 1))
    }

    pub fn provider_apply_series(
        &mut self,
        profile: &str,
        stable: &str,
        snapshot: &SeriesProviderSnapshot,
        provider: &SeriesProvider,
        explicit: bool,
    ) -> Result<Vec<Media>, String> {
        self.require_profile(profile)?;
        self.assert_provider_cas(stable, snapshot.anchor_revision, snapshot.common_revision)?;
        let updated = self.catalogue.apply_series_provider(
            &snapshot.series,
            &snapshot.members,
            provider,
            explicit,
        )?;
        let tx = self
            .personal
            .unchecked_transaction()
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        for item in &updated {
            let stable: Option<String> = tx
                .query_row(
                    "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1",
                    [&item.id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
            let stable = stable.ok_or("Vidéo introuvable dans le catalogue.")?;
            tx.execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                params![stable],
            )
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        }
        tx.commit()
            .map_err(|_| "Erreur de base locale. Vos données n’ont pas été remplacées.")?;
        updated
            .into_iter()
            .map(|item| self.profile_view(profile, item))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chill_catalog::model::{Media, SeriesPatch};
    use tempfile::tempdir;

    fn media(id: &str, title: &str) -> Media {
        Media {
            id: id.into(),
            path: format!("/nas/{id}"),
            title: title.into(),
            category: "film".into(),
            year: Some(2000),
            series: None,
            season: None,
            episode: None,
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
            added_at: 0,
            tmdb_id: None,
        }
    }
    fn store(items: Vec<Media>) -> (Store, String) {
        let path = tempdir().unwrap().keep().join("catalog.sqlite");
        let mut store = Store::open(&path).unwrap();
        store.snapshot(items).unwrap();
        let profile = store.create_profile("Admin").unwrap();
        (store, profile.id)
    }

    #[test]
    fn provider_media_cas_preserves_custom_poster_and_rejects_concurrent_change() {
        let mut item = media("film.mkv", "Original");
        item.poster = Some("/cache/custom.png".into());
        item.poster_custom = true;
        let (mut store, profile) = store(vec![item]);
        let id = store.catalog(&profile).unwrap()[0].id.clone();
        let snapshot = store.provider_snapshot(&profile, &id, 1).unwrap();
        let mut downloaded = snapshot.media.clone();
        downloaded.title = "TMDB title".into();
        downloaded.tmdb_id = Some(42);
        downloaded.poster = Some("/cache/provider.jpg".into());
        let (updated, revision) = store
            .provider_apply_media(&profile, &id, &snapshot, downloaded.clone(), true)
            .unwrap();
        assert_eq!(revision, 2);
        assert!(updated.poster_custom);
        let artwork = format!("/api/v1/profiles/{profile}/media/{id}/artwork");
        assert_eq!(updated.poster.as_deref(), Some(artwork.as_str()));

        let snapshot = store.provider_snapshot(&profile, &id, revision).unwrap();
        store
            .update_media_at_revision(
                &profile,
                &id,
                revision,
                &chill_catalog::model::MediaPatch {
                    title: Some("Local correction".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(
            store
                .provider_apply_media(&profile, &id, &snapshot, downloaded, false)
                .is_err()
        );
        assert_eq!(
            store.media(&profile, &id).unwrap().title,
            "Local correction"
        );
    }

    #[test]
    fn series_provider_renames_atomically_but_keeps_episode_fields() {
        let mut first = media("s01e01.mkv", "Episode one");
        first.category = "serie".into();
        first.series = Some("Old name".into());
        first.season = Some(1);
        first.episode = Some(1);
        let mut second = media("s01e02.mkv", "Episode two");
        second.category = "serie".into();
        second.series = Some("Old name".into());
        second.season = Some(1);
        second.episode = Some(2);
        let (mut store, profile) = store(vec![first, second]);
        let ids = store
            .catalog(&profile)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect::<Vec<_>>();
        let snapshot = store
            .provider_series_snapshot(&profile, &ids[0], 1)
            .unwrap();
        let provider = SeriesProvider {
            name: "New name".into(),
            metadata: SeriesPatch {
                name: "New name".into(),
                year: 2024,
                genres: vec!["SF".into()],
                overview: "Shared".into(),
                cast: vec![],
                directors: vec![],
            }
            .validate(&snapshot.members[0])
            .unwrap(),
            tmdb_id: 7,
            poster: Some("/cache/tv.jpg".into()),
        };
        let changed = store
            .provider_apply_series(&profile, &ids[0], &snapshot, &provider, true)
            .unwrap();
        assert_eq!(changed.len(), 2);
        assert!(
            changed
                .iter()
                .all(|m| m.series.as_deref() == Some("New name"))
        );
        assert_eq!(
            changed.iter().find(|m| m.id == ids[1]).unwrap().title,
            "Episode two"
        );

        let snapshot = store
            .provider_series_snapshot(&profile, &ids[0], 2)
            .unwrap();
        store
            .update_media_at_revision(
                &profile,
                &ids[1],
                2,
                &chill_catalog::model::MediaPatch {
                    overview: Some("Corrected episode".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(
            store
                .provider_apply_series(&profile, &ids[0], &snapshot, &provider, false)
                .is_err()
        );
        assert_eq!(
            store.media(&profile, &ids[1]).unwrap().overview,
            "Corrected episode"
        );
    }
}
