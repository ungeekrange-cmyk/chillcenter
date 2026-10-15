# Revue code indépendante — identité série/saison

- Reviewer : `/root/review_series_sync`
- Base : `a59f5e6921f5e495c7fadc039dadb99c814d844a`
- Candidat : `f544684b002ad8c1bf5d31f98006da573f576019`
- Périmètre : modèle/persistance série et saison, frontières fournisseur, catalogue, routes TMDB/catalogue et ADR-0015
- Verdict : `CHANGES_REQUESTED`

## Constats bloquants

### P1 — Application fournisseur série non atomique

`Store::provider_apply_series` valide puis fait committer `Database::apply_series_provider` avant d'ouvrir une seconde transaction pour les révisions `cc_media_revision` (`crates/chill-domain/src/provider_updates.rs:171-208`). Si cette deuxième phase échoue, les métadonnées et le chemin de l'affiche sont déjà publiés. Le handler traite pourtant le résultat comme un échec et supprime l'affiche nouvellement téléchargée (`crates/chill-server/src/tmdb_api.rs:1142-1155`). Le catalogue peut donc référencer un fichier supprimé avec des révisions média non avancées.

Correction attendue : CAS du groupe figé, mises à jour `media` et incréments `cc_media_revision` dans une unique transaction `IMMEDIATE` sur la même connexion SQLite. La transformation/validation catalogue peut être extraite en helper sans commit ; l'affiche n'est conservée qu'après le commit unique.

### P1 — Révision absente pour la clé publique d'une série non identifiée

Le catalogue expose désormais une série non reconnue sous `media:<uuid>`, mais `sync_snapshot.series_revisions` continue d'indexer ces séries par leur libellé (`crates/chill-domain/src/store.rs:1437-1450`). Le client miroir cherche ensuite `series_revisions[name]` avec la clé `media:<uuid>` (`crates/chill-client/src/ui.rs:1368-1371`) et obtient `null`. La modification des informations communes ne peut donc pas fournir le `u64 expected_revision` requis.

Correction attendue : publier une entrée de révision par groupe source sous sa clé publique `media:<uuid représentatif>`, et couvrir le parcours miroir `Series -> EditSeries` de bout en bout.

## Constats importants

### P2 — La vitesse reste partagée par le nom d'affichage

Les nouvelles identités séparent notes et enchaînement, mais `save_speed` et `playback_info` utilisent toujours `media.series` comme clé (`crates/chill-server/src/playback.rs:979-982`, `crates/chill-server/src/catalog_api.rs:1011-1014`). Deux séries homonymes identifiées partagent donc la même vitesse. De plus, l'édition commune tente de migrer depuis la clé opaque de route plutôt que depuis l'ancienne clé de vitesse (`crates/chill-domain/src/administration.rs:156-159`), ce qui perd la préférence lors d'un renommage.

Correction attendue : utiliser la même identité stable de série pour la vitesse, avec lecture/migration explicite des anciennes clés par nom.

### P2 — Le fallback par nom peut encore fusionner deux séries identifiées

Les branches de compatibilité par nom mesurent l'ambiguïté uniquement avec `series_source_key` (`crates/chill-domain/src/store.rs:645-651`, `crates/chill-domain/src/administration.rs:122-131`). Deux séries déjà identifiées, portant le même nom et rangées dans une même racine plate, ont une seule clé source malgré deux `tmdb_id`. Une lecture ou édition via l'ancienne route par nom rassemble alors les deux groupes.

Correction attendue : calculer l'identité d'ambiguïté avec `tmdb:tv:<id>` lorsqu'un identifiant existe, et la clé source seulement avant identification.

### P2 — Reconstruction de groupe publiable avec une révision périmée

`refresh_series_groups` lit la révision commune, matérialise tout le catalogue, puis enregistre cette première révision sans revérification (`crates/chill-domain/src/store.rs:2217-2247`). Un scan committé entre la matérialisation et la transaction laisse la requête courante voir les nouveaux épisodes sans entrée de groupe ; ils apparaissent temporairement comme cartes séparées jusqu'au prochain appel.

Correction attendue : revérifier la révision dans la transaction avant de publier l'état dérivé, ou abandonner/rejouer la reconstruction si elle a changé.

## Vérifications et delta de la première revue

Les corrections précédemment demandées pour la recherche/tri par libellé, le rejet saison sans parent avant I/O, la révision d'affiche de saison et le nettoyage RAII des nouvelles images sont présentes. Les tests existants couvrent deux racines homonymes, deux saisons, les imports échoués et la séparation des métadonnées saison/épisode.

Exécution indépendante sur le SHA exact :

`cargo test --locked --offline -p chill-domain -p chill-client -p chill-server --lib`

Résultat : `37 + 59 + 102` tests PASS. Ces tests ne reproduisent pas les constats ci-dessus ; le verdict reste `CHANGES_REQUESTED`. ADR-0015 reste `PENDING` car ses garanties de CAS atomique et de compatibilité non ambiguë ne sont pas encore satisfaites par le candidat.
