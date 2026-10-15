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

---

## Réévaluation delta — candidat `3bbb87db5a3331f8f07d12ab1404afd8a5d86ee7`

- Delta revu : `f544684b002ad8c1bf5d31f98006da573f576019..3bbb87db5a3331f8f07d12ab1404afd8a5d86ee7`
- Mode : réévaluation opérationnelle ciblée, sources en lecture seule
- Verdict : `CHANGES_REQUESTED`

Les quatre défauts historiques ont une correction effective dans ce delta : écriture fournisseur/révisions/vitesse dans une transaction SQLite unique, alias de révision `media:<id>` pour chaque épisode avec alias de nom seulement lorsqu'il est non ambigu, identité stable pour la vitesse et migration prudente, regroupement dérivé sous transaction `IMMEDIATE` avec revérification de la révision. Les chemins inspectés restent bornés à 10 000 médias et ne sérialisent pas la racine NAS.

### P1 — Le CAS fournisseur ne protège plus la complétude du groupe figé

`Store::provider_apply_series` contrôle `common_revision` avant les lectures de migration, puis ouvre la transaction plus tard (`crates/chill-domain/src/provider_updates.rs:180-188`). Le scan publie sur une autre connexion SQLite ; il peut donc ajouter un épisode entre ce contrôle et la transaction. Dans `Database::apply_series_provider_with`, l'état courant est ensuite filtré uniquement par les identifiants déjà capturés (`crates/chill-catalog/src/database.rs:425-448`) : un nouveau membre du même groupe n'est ni détecté ni mis à jour. La série se retrouve partiellement associée à TMDB et peut être scindée en deux cartes.

La régression existe déjà sans ordonnancement artificiel : `database::tests::series_provider_rejects_stale_group_but_preserves_concurrent_episode_edits` ajoute un troisième épisode après la capture et l'opération réussit à tort (`crates/chill-catalog/src/database.rs:2511-2519`). Les imports d'affiche ont la même fenêtre : ils contrôlent la révision avant l'écriture du fichier, puis appliquent seulement `snapshot.members` sans revérifier la révision commune dans leur transaction (`crates/chill-domain/src/provider_updates.rs:316-324`, `384-411`).

Correction attendue : dans la transaction `IMMEDIATE` qui publie, relire et comparer la révision commune attendue, puis recalculer l'ensemble courant depuis l'ancre et vérifier qu'il correspond exactement aux identifiants capturés avant toute écriture. Appliquer le même contrôle transactionnel aux affiches série/saison. Ajouter un test à deux connexions qui fait committer un scan entre capture et publication.

### P1 — La suite propriétaire `chill-catalog` est rouge sur le SHA exact

Commande indépendante :

`cargo test --locked --offline -p chill-catalog -p chill-domain --lib`

Résultat : `chill-catalog` **73 PASS / 2 FAIL**. Outre le test de groupe périmé ci-dessus, `database::tests::series_provider_rejects_collision_and_rolls_back_sql_failure` échoue sur son ancienne attente qu'un libellé homonyme soit rejeté (`crates/chill-catalog/src/database.rs:2531-2536`). Autoriser deux séries portant le même nom mais ayant des identités TMDB distinctes est la nouvelle décision correcte ; ce test doit donc être réécrit pour vérifier la séparation par identité tout en conservant ses assertions d'identifiant invalide et de rollback SQL. La commande combinée s'arrête avant `chill-domain`; exécuté séparément, `chill-domain` est vert à **62 PASS**.

ADR-0015 reste `PENDING` sur ce candidat : son contrat « groupe figé complet » et son exigence de CAS ne sont pas encore garantis au point de commit. Une nouvelle révision source rendra cette réévaluation historique et nécessitera un nouveau verdict exact-SHA.

---

## Réévaluation finale — candidat `11aa529f2eced060156a3017456bf1d9634f8a56`

- Delta revu : `3bbb87db5a3331f8f07d12ab1404afd8a5d86ee7..11aa529f2eced060156a3017456bf1d9634f8a56`
- Verdict code : `APPROVED_SCOPED`
- Verdict ADR-0015 : `APPROVED_SCOPED`

Les deux blocages de la réévaluation précédente sont levés. La transaction `IMMEDIATE` relit maintenant la révision commune et celle de l’ancre, puis recalcule le groupe complet depuis l’ancre avant toute écriture. Cette vérification est appliquée à l’association fournisseur série, aux métadonnées de saison et aux affiches série/saison. Les métadonnées, révisions et migrations de vitesse restent dans le même commit SQLite. Le test à deux connexions prouve qu’un scan committé après le premier CAS est refusé sans association partielle. Les tests catalogue distinguent désormais correctement collision de libellé et collision d’identité tout en conservant les contrôles d’identifiant invalide et de rollback SQL.

Exécution indépendante sur le SHA exact :

`cargo test --locked --offline -p chill-catalog -p chill-domain -p chill-client -p chill-server --lib`

Résultat : `75 + 64 + 39 + 103 = 281` tests PASS, aucun échec. `git diff --check` du delta ciblé PASS. Aucun P1/P2 nouveau dans le périmètre série/saison. ADR-0015 décrit désormais fidèlement le CAS au point de commit, la séparation des homonymes, les métadonnées saison distinctes et les limites assumées de l’identité pré-TMDB ; son acceptation reste bornée à ce périmètre et ne constitue pas une validation TMDB/NAS réelle ni un feu vert de déploiement.
