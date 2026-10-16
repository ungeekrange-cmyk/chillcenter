# Optimisations ressources — 16 octobre2026

Base **228c4c6e2a02c9f6ff7520b3d2e0fb77bc5104db**, branche `codex/resource-optimizations`. Demande utilisateur explicite : appliquer les pistes de l'audit, mesurer avant/après, reconstruire le Mac et mettre à jour le Pi sans toucher au web. Livraison ciblée DEV_FAST familiale ; pas de runtime/dépendance/API, configuration, NAS ou tampon modifié. Sources/worktrees/historiques/versions livrées conservés.

## Frontière fonctionnelle

- Client : déplacer le `SyncResult` vers publication SQLite au lieu de le cloner ; tâche consommable une fois, finalisation légère, révision inchangée rapide et CAS/annulation conservés.
- Serveur : partager le snapshot complet par Arc entre reset paginé et checkpoint durable ; delta paginé propre mais checkpoint de l'état **complet** courant, admission/429/budget/TTL avant checkpoint inchangés. Les réponses HTTP ne changent pas.
- Images : [ADR0024](../architecture/decisions/ADR-0024-artwork-content-deduplication.md), liens physiques vers CAS privé avec chemins historiques conservés et fallback copie ; migration explicite client fermé, aucune ouverture ou modification SQLite/WAL/SHM.
- SQLite : aucune compaction : fragmentation non démontrée, VACUUM ne serait pas une optimisation neutre en latence/verrouillage/espace temporaire.

## Méthode RAM reproductible

Même matériel Mac, outils et flags Rust, mêmes fixtures déterministes, processus de test neuf à chaque mesure, `/usr/bin/time -l <test-binary> --exact <ignored-test> --ignored --nocapture`, trois runs par version ; le temps de compilation n'entre pas dans la mesure. Mesure **RSS maximum du processus de fixture**, pas baisse du RSS total de l'application Tauri ou promesse de gain identique sur le Pi. Les sorties brutes sont conservées privées horsGit.

Client : test `ui::tests::benchmark_catalog_publication_memory`,10000films avec synopsis2048octets chacun, publication réelle dans SQLite temporaire puis relecture de contrôle10000lignes. Baseline6039d43 = source228 + même harness uniquement, optimisé1311060 (même harness de mesure, tests supplémentaires sans effet sur ce test). La validation de relecture est incluse dans le pic des deux versions.

| Client, RSS maximal en octets | Run1 | Run2 | Run3 | Médiane |
| --- | ---: | ---: | ---: | ---: |
| Avant | 98254848 | 98336768 | 98189312 | 98254848 |
| Après | 57982976 | 57917440 | 57982976 | 57982976 |

Soit **93,70→55,30Mio**, **−38,41Mio / −40,99%** sur ce banc, sans conclusion de ce pourcentage pour la RAM de la webview/vidéo ou le repos utilisateur. Révision et10000lignes identiques vérifiées. Les tests CAS périmé/unchanged et hors-ligne restent distincts de ce chiffre.

Serveur : test `sync_api::tests::sync_memory_10k_production_route`,10000films/synopsis2500octets, vrai handler/contexte/checkpoint SQLite,100pages et delta1. Checksum incrémental des champs déterministes, pas des UUID/cursors propres à chaque base. Premières mesures avec candidat gardant encore une copie du reset et accumulation d'un gros String de contrôle écartées du résultat final ; aucun chiffre intermédiaire promu au gain final. Résultat final à compléter.

## Stockage

Fixture initiale : deux fichiers identiques de31732octets dans deux profils. Estimation allouée65536→32768octets, mêmes deux chemins et contenus, test de répétition stable. Ce n'est pas encore le gain du cache réel.

Cache utilisateur : application ouverte détectée, demande de fermeture puis confirmation reçue. Toute migration réelle attend revue du candidat, nouvelle vérification des processus, manifeste privé avant/après et validation des fichiers non-images. Mesures effectives à compléter ; aucun catalogue hors ligne, historique ou version livrée n'est supprimé.

## Livraison et limites

Source figée/revues, tests intégrés, gains serveur/cache réel, bundle/hash Mac, sauvegarde/bascule Pi et vérification distincte à joindre. Aucun nouveau lecteur général, fournisseur, scan NAS ou benchmark de flux réels demandé par ces optimisations.
