# Revue delta de sécurité — Chill Center UI/API

## Décision

- **Verdict : APPROVED**
- **Candidat exact :** `50bbdf800a6c46c67ddd3cce74d8358286cd8eb6`
- **Base de delta :** `258db0ad27f05fafb12bd3592013ef300ead93fd`
- **Relecteur indépendant :** `/root/ux_api_security_review`
- **Date de vérification :** 2026-10-14T16:21:29Z
- **Périmètre :** fermeture de `SEC-258-01`, absence de régression secrets/API/native UI. Le modèle domestique LAN sans authentification reste une frontière produit acceptée, pas un modèle multi-tenant.

Aucun finding HIGH ou CRITICAL ne reste ouvert dans ce delta. Le HIGH `SEC-258-01` est fermé par des limites appliquées avant toute création de tâche ou publication de l'état.

## Fermeture de SEC-258-01

| Menace initiale | Contrôle du candidat | Preuve source exacte |
|---|---|---|
| Nombre non borné de futures | Sémaphore de deux slots, acquis avec `try_acquire_owned` avant lecture DB/énumération ; dépassement = `429 TMDB_JOB_BUSY` | `crates/chill-server/src/tmdb_api.rs:46`, `491-495`, `637-642`, `802-823`, `845-862` |
| Annulation libérant trop tôt la capacité | Le permis appartient à `JobTask`, déplacé dans la future et gardé jusqu'à retour/abort/drop ; le job reste dans `inflight` jusque-là | `tmdb_api.rs:216-241`; `tmdb_worker.rs:127-138` |
| Worklist implicite/auto/retry trop grande | Maximum 3 000 sur désérialisation HTTP, construction implicite, auto et chargement persistant ; identifiants/profil limités à 128 octets | `tmdb_api.rs:49-50`, `269-291`, `505-519`, `585-622`, `779-816` |
| Historique persistant non borné | Maximum huit terminaux et TTL quinze minutes ; pruning lors de la lecture, persistance, création et reprise | `tmdb_api.rs:47-49`, `249-268`, `418-453`, `545-558` |
| État hostile/non borné au démarrage | Fichier régulier uniquement, lecture et sérialisation plafonnées à 2 Mio, visite streaming, invariants de compteurs et listes vérifiés | `tmdb_api.rs:269-369`, `382-416` |
| Amplification/race des écritures | `persist_gate` sérialise les publications ; création stage une copie bornée, écrit atomiquement, puis seulement publie map et `inflight` ; erreur = aucune mutation et permis rendu | `tmdb_api.rs:418-441`, `542-566` |
| Course annulation/CAS | Le slot subsiste durant l'appel fournisseur et la vérification d'annulation reste ordonnée avec l'écriture catalogue | `tmdb_worker.rs:256-295` |

Les ordres de verrous observés sont cohérents (`persist_gate → jobs → inflight → state_path`, ou `jobs → inflight` sans reprise inverse). Une annulation répétée, un retry et l'auto-enrichissement ne peuvent pas remplacer les futures qui possèdent encore les deux slots. Un abort passe par `Drop`, retire l'identifiant `inflight`, stabilise l'état terminal puis persiste avant de rendre le permis.

## Régressions de sécurité recherchées

- **Secrets TMDB :** aucun changement de frontière. Credential toujours privé/typé, expurgé des DTO et erreurs, fichier `0600` publié avec fsync, supprimé durablement et exclu des exports. L'état du worker ne contient aucun credential ni URL fournisseur.
- **Réseau fournisseur :** hôtes HTTPS fixes, redirections désactivées, limites et timeouts inchangés ; aucun commutateur public de test ou base URL configurable ajouté.
- **API :** les erreurs de JSON surdimensionné sont normalisées sans détail interne ; `3001` entrées produit `413 TMDB_JOB_TOO_LARGE`. L'OpenAPI datée reste strictement typée et le secret est `writeOnly`.
- **Isolation fonctionnelle :** profil requis/validé avant création ; les données de job internes ne sont pas renvoyées par le DTO de polling. L'absence d'authentification signifie toujours que toute personne admise sur le LAN peut administrer le foyer.
- **Client JS :** traitement explicite des codes job expiré/saturé/trop grand, arrêt du polling périmé et conservation des contrôles de génération profil/serveur ; aucune injection HTML ajoutée.
- **Natif :** le delta plein écran ne change que l'oracle de géométrie debug vers la safe area AppKit et retire `window_frame`; il n'élargit ni IPC, navigation, fichier, URL ni secret.
- **Dépendances/runtime :** aucun manifeste ou lockfile modifié. Rust/Cargo `1.99.0`, Node `v24.13.0`; aucune nouvelle provenance ou licence à examiner dans ce delta.
- **Balayage ciblé :** aucun motif de clé privée, token GitHub/OpenAI ou clé AWS détecté au SHA exact. Ce contrôle par motifs n'est pas un scanner d'entropie.

## Vérifications indépendantes exécutées

| Commande | Résultat |
|---|---|
| `cargo test -p chill-server tmdb --lib --locked --offline` avec `CARGO_TARGET_DIR=/private/tmp/chillcenter-server-ux-target` | **PASS — 26/26**, 71 filtrés |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` | **PASS — 40/40** |
| `node scripts/validate-api-contract.mjs` | **PASS — 66 chemins, OpenAPI 2026-10-14** |
| `git diff --check 258db0a..50bbdf8` | **PASS** |
| Diff manifestes/lockfiles | Aucun fichier |
| `git status --short` et `git rev-parse HEAD` | Worktree propre, SHA exact |

Les tests ciblés couvrent notamment : saturation/cancel/retry/auto, abort, pruning et redémarrage, état surdimensionné/malformé conservé sans écrasement, erreur de persistance sans mutation, budget sérialisé, 3 001 entrées et exclusion des secrets des exports.

## Limites et preuves non revendiquées

- Aucun token TMDB réel, credential externe, fournisseur réel, Pi, NAS, SSH ou cloud n'a été utilisé ou modifié.
- Les mesures réelles RSS/disque du Pi n'ont pas été répétées. La preuve est une borne structurelle : deux jobs, huit terminaux, 2 Mio d'état. Une publication atomique peut temporairement occuper le fichier courant plus un stage, soit environ deux fois le budget.
- Le smoke HTTPS de 30 réponses, les 38 payloads OpenAPI 3.1, les 289 tests Rust, le clippy et le vrai shell/plein écran Tauri sont des preuves transmises par des instances distinctes ; cette revue n'a réexécuté que les contrôles listés ci-dessus.
- L'ancien diagnostic `1073/1107` du SHA `258db0a` reste historique. Le PASS natif associé au nouveau SHA est une preuve QA distincte, pas une conclusion déduite par cette revue statique.
- `scripts/agent-context.py` est absent de ce dépôt, conformément à `docs/PROJECT_CONTEXT.md`; le contexte, le scope, l'architecture et le journal des bugs ont été lus directement.
- L'approbation porte uniquement sur le SHA exact ci-dessus. Toute nouvelle correction source, notamment queue/persistance/API/native, requiert une revue delta.

## Conclusion

`SEC-258-01` est corrigé sans régression de secret, d'isolation fonctionnelle ou d'exposition réseau détectée. Le candidat exact est **APPROVED** pour intégration dans la frontière LAN domestique documentée.
