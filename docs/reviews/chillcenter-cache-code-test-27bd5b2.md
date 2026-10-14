# Revue indépendante cache lecteur — `27bd5b2`

- Mode : `EXECUTION_REVIEW` complété par une revue ADR indépendante.
- Base / source revue : `c58da7e614b54890e0ef0207f4679d9b02cfca0e` / `27bd5b24b101ac7028bbd3ad8a2f81159bd40e03`.
- Extraction immuable : `/private/tmp/chillcenter-review-27bd5b2.cTHvUZ` ; `crates/chill-player/src/lib.rs` = `a92528c7dba06efa2eebc62e5c58fdc71e4528c95e8285918bc7866ab25f328e`.
- ADR revue : `docs/architecture/decisions/ADR-0010-client-playback-cache.md` = `e20eab4affd917d736ba675c5e5cf22b91c5cce5048c3b4c4204f81a2b09746a`.
- Relecteur : `/root/pi_integration_gate`, distinct de l'auteur `/root` ; revue le `2026-10-14T12:15:54Z`.
- Environnement : macOS 27.0.1 arm64 ; rustc 1.99.0 ; cargo 1.99.0 ; mpv/libmpv 0.41.0 (API embarquée documentée 2.5) ; FFmpeg 9.0.2.

## Verdicts

- **Code et tests du SHA exact : `APPROVED`.** Les valeurs effectives libmpv sont vérifiées avec la vraie bibliothèque : `cache=yes`, avance temporelle 60 s, budgets avant/arrière 128/64 Mio et donation désactivée. Le schéma personnalisé est donc mis en cache sans dépendre de l'heuristique réseau. Les budgets portent sur les paquets compressés et non sur le RSS total ; mpv peut techniquement dépasser légèrement ses limites. Aucun fichier serveur n'est modifié et le rollback ne requiert ni migration ni action NAS/Pi.
- **ADR-0010 : `CHANGES_REQUESTED`.** Elle doit rester `PROPOSED`. Le choix technique est crédible et réversible, mais la décision n'est pas encore cohérente/traçable avec l'architecture acceptée.
- **Gate bundle/essai NAS : `NOT_RUN` dans cette revue.** Le build `.app`, le RSS client réel, le retour arrière utilisable et les trois flux Dune/NAS ne sont pas déduits des tests unitaires. Leur exécution en cours ne devient pas une preuve de ce rapport.

## Preuves reproductibles

| Commande sur l'extraction immuable | Exit | Durée | Résultat |
| --- | ---: | ---: | --- |
| `cargo test --locked --offline -p chill-player -p chill-client` | 0 | 4,08 s | 46 pass, 0 fail, 0 ignored ; doctests 0. Exécuté hors sandbox uniquement pour les deux listeners loopback de non-contournement HTTP/HLS. |
| `cargo test --locked --offline --manifest-path scripts/pi-probe/Cargo.toml` | 0 | 8,45 s | 1 pass, 0 fail, 0 ignored ; la nouvelle lecture de propriété compile. |
| `cargo clippy --locked --offline -p chill-player -p chill-client --all-targets -- -D warnings` | 0 | 4,16 s | Aucun avertissement. |
| `cargo check --locked --offline -p chillcenter-desktop` | 0 | 4,11 s | Intégration desktop compile. |
| `cargo clippy --locked --offline --manifest-path scripts/pi-probe/Cargo.toml -- -D warnings` | 0 | 0,32 s | Harnais opérateur linté dans son mode de build. |
| Recalcul du manifeste avec les chemins de `git ls-tree -r 27bd5b2` puis `diff -u MANIFEST.sha256` | 0 | 10,3 s | Manifeste exact. |
| `git diff --check c58da7e..27bd5b2` | 0 | <0,1 s | Aucun défaut d'espacement. |

La suite à 46 tests comporte une assertion non simulée sur les options libmpv effectivement acceptées, un décodage/seek/pistes par `stream_cb`, et deux tests qui prouvent qu'une référence HTTP/M3U/HLS ne contourne pas le transport épinglé. Les préconditions des tests conditionnels étaient présentes (`/opt/homebrew` mpv et FFmpeg), donc elles n'ont pas été assimilées à tort à des skips.

Contrôle supplémentaire informatif : `cargo clippy ... --manifest-path scripts/pi-probe/Cargo.toml --all-targets -- -D warnings` échoue sur `clippy::items-after-test-module`. La même disposition existe déjà à la base `c58da7e` (module test ligne 125, fonctions produit après ligne 139) ; le clippy de build normal passe. Ce lint de tests préexistant n'est pas une régression de `27bd5b2`, mais reste à nettoyer séparément.

## Couverture, limites et risques

| Risque | Preuve | Résultat |
| --- | --- | --- |
| Option inconnue/incompatible | libmpv 0.41 réelle initialise et expose les six valeurs attendues | PASS |
| Cache absent sur `chill-media` | `cache=yes`, indépendamment de `cache=auto` | PASS |
| Mémoire non bornée / arrière consommant l'avant | 128/64 Mio et `demuxer-donate-buffer=no` | PASS pour les budgets de paquets ; RSS total NOT_RUN |
| Référence contournant HTTPS épinglé | listeners HTTP/M3U et HLS jamais contactés | PASS |
| Régression client/baux/transport | 30 tests client + tests de callback/annulation lecteur | PASS au niveau code ; déploiement réel non rejoué ici |
| 60 s avant / ~30 s arrière sur NAS Wi-Fi | `demuxer-cache-duration` seulement approximatif et aucune limite arrière temporelle native | NOT_RUN dans cette revue ; ne pas promettre une durée exacte |

Le test d'options valide fortement la configuration par défaut, mais pas le remplissage effectif, le seek arrière sans nouvelle lecture réseau, ni le RSS du processus. La propriété `demuxer-cache-duration` est décrite par le manuel mpv comme une estimation « very unreliable » et parfois absente ; une valeur proche de 60 s est une observation indicative, pas une preuve de garantie. Pour l'arrière, il faut observer un seek d'environ 30 s et l'absence de refetch/échec sur les médias représentatifs.

## Findings ADR requis

| ID | Évidence | Risque | Changement requis |
| --- | --- | --- | --- |
| `ADR-F-001` | `docs/architecture/ARCHITECTURE.md:99` conserve « cache vidéo Mac plafonné à 64 Mio par lecture », tandis qu'ADR-0010 décide 128+64 = 192 Mio de paquets par lecteur. L'ADR dit `Supersedes: NONE` et ne liste pas cet artefact/NFR parmi les invalidations. | Deux budgets incompatibles restent normatifs ; les anciennes conclusions mémoire pourraient être réutilisées à tort. | Nommer explicitement la section affectée, expliquer si le plafond 64 Mio est remplacé ou redéfini, et marquer les preuves/seuils mémoire antérieurs comme à revalider. Mettre ensuite l'architecture à jour par le propriétaire après acceptation. |
| `ADR-F-002` | Le manuel mpv 0.41 qualifie `demuxer-cache-duration` d'approximation très peu fiable et précise que les limites octets peuvent être légèrement dépassées ; l'ADR parle d'avance « mesurée » et de plafonds sans cette qualification. | Une télémétrie indicative pourrait être présentée comme preuve de 60 s ou comme limite mémoire stricte. | Qualifier la propriété et les plafonds de valeurs approximatives, définir l'oracle de l'essai réel (valeur observée + progression/buffering + seek arrière/refetch + RSS), et interdire explicitement l'inférence d'un back-cache 30 s depuis cette seule propriété. |
| `ADR-F-003` | L'entrée ADR-0010 dans l'index ne porte pas les métadonnées date/propriétaire/liens de remplacement demandées par la politique ADR. | Traçabilité de décision incomplète. | Ajouter ces métadonnées lors de la révision, sans passer le statut à `ACCEPTED` avant la nouvelle revue et le gate utilisateur exact. |

La demande ultérieure de rendre les budgets mémoire configurables n'est pas incluse dans `27bd5b2` et devra être revue sur un nouveau SHA (validation, persistance locale, UI et application effective au lecteur). Aucun fichier produit, test ou ADR n'a été modifié pendant cette revue.
