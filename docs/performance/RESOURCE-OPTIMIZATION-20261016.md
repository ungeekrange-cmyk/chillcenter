# Optimisations ressources — 16 octobre2026

Base **228c4c6e2a02c9f6ff7520b3d2e0fb77bc5104db**, branche `codex/resource-optimizations`. Demande utilisateur explicite : appliquer les pistes de l'audit, mesurer avant/après, reconstruire le Mac et mettre à jour le Pi sans toucher au web. Livraison ciblée DEV_FAST familiale ; pas de runtime/dépendance/API, configuration, NAS ou tampon modifié. Sources/worktrees/historiques/versions livrées conservés.

## Frontière fonctionnelle

- Client : déplacer le `SyncResult` vers publication SQLite au lieu de le cloner ; tâche consommable une fois, finalisation légère, révision inchangée rapide et CAS/annulation conservés.
- Serveur : partager le snapshot complet par Arc entre reset paginé et checkpoint durable ; delta paginé propre mais checkpoint de l'état **complet** courant, admission/429/budget/TTL avant checkpoint inchangés. Les réponses HTTP ne changent pas.
- Images : [ADR0024](../architecture/decisions/ADR-0024-artwork-content-deduplication.md), liens physiques vers CAS privé avec chemins historiques conservés et fallback copie ; migration explicite client fermé, aucune ouverture ou modification SQLite/WAL/SHM.
- SQLite : aucune compaction : fragmentation non démontrée, VACUUM ne serait pas une optimisation neutre en latence/verrouillage/espace temporaire.

## Méthode RAM reproductible

Même matériel Mac, outils et flags Rust au sein de chaque comparaison, mêmes fixtures déterministes, processus de test neuf à chaque mesure, `/usr/bin/time -l <test-binary> --exact <ignored-test> --ignored --nocapture`, trois runs par version ; le temps de compilation n'entre pas dans la mesure. Mesure **RSS maximum du processus de fixture**, pas baisse du RSS total de l'application Tauri ou promesse de gain identique sur le Pi. Les sorties brutes sont conservées privées horsGit. **Le bilan de remise utilise les compilations release ci-dessous** ; le premier relevé debug reste un checkpoint distinct.

Client : test `ui::tests::benchmark_catalog_publication_memory`,10000films avec synopsis2048octets chacun, publication réelle dans SQLite temporaire puis relecture de contrôle10000lignes. Baseline6039d43 = source228 + même harness uniquement, optimiséf980355. La validation de relecture est incluse dans le pic des deux versions.

| Client, RSS maximal en octets | Run1 | Run2 | Run3 | Médiane |
| --- | ---: | ---: | ---: | ---: |
| Avant | 98254848 | 98336768 | 98189312 | 98254848 |
| Après, final debug | 58015744 | 57966592 | 58064896 | 58015744 |

Soit **93,70→55,33Mio**, **−38,38Mio / −40,95%** en debug sur ce banc, sans conclusion de ce pourcentage pour la RAM de la webview/vidéo ou le repos utilisateur. Révision et10000lignes identiques vérifiées. Premier checkpoint1311060 après57982976médian conservé dans ses logs ; la table utilise désormais le finalf980. Les tests CAS périmé/unchanged et hors-ligne restent distincts de ces chiffres.

Serveur : test `sync_api::tests::sync_memory_10k_production_route`,10000films/synopsis2500octets, vrai handler/contexte/checkpoint SQLite,100pages et delta1. Checksum incrémental des champs déterministes, pas des UUID/cursors propres à chaque base. Premières mesures avec candidat gardant encore une copie du reset et accumulation d'un gros String de contrôle écartées du résultat final ; aucun chiffre intermédiaire promu au gain final. Baseline228c4c6 + harness strictement identique seulement ; serveur optimisé inchangé9fb→f980.

| Serveur, debug RSS maximal en octets | Run1 | Run2 | Run3 | Médiane |
| --- | ---: | ---: | ---: | ---: |
| Avant | 184664064 | 256524288 | 256409600 | 256409600 |
| Après | 158564352 | 196263936 | 191889408 | 191889408 |

244,53→183,00Mio,−25,16% médian debug ; dispersion conservée, pas une constante universelle. Les six checksums normalisés sont identiques : **833ca58fe50e12e241468c0fe7cc285e00699e2cd639a274120e01b92bede10d**.

### Mesures de production release — bilan retenu

Exécution par instance `/root/resource_audit`, sans modification du produit : `cargo test --release --locked --offline -p chill-client|chill-server --lib --no-run`, même harness, puis exécutable de test figé appelé directement trois fois par version, temps compilation exclu. Un cache Cargo partagé a d'abord réutilisé un artefact baseline lors du passage au final : mesure invalidée avant usage, paquets release concernés nettoyés de manière ciblée, rebuild effectif et hashes d'exécutables distincts contrôlés. Aucun clean global ni suppression de source, debug, worktree ou version livrée. Les run ci-dessous sont ceux des rebuilds contrôlés, pas de l'artefact réutilisé.

| RSS maximum, octets | Run1 | Run2 | Run3 | Médiane |
| --- | ---: | ---: | ---: | ---: |
| Client release avant | 95404032 | 96108544 | 82411520 | 95404032 |
| Client release après | 68993024 | 55672832 | 69500928 | 68993024 |
| Serveur release avant | 250511360 | 250576896 | 242089984 | 250511360 |
| Serveur release après | 231079936 | 222281728 | 212271104 | 222281728 |

Client : **90,98→65,80Mio**, **−25,19Mio / −27,68%**. Serveur : **238,91→211,98Mio**, **−26,92Mio / −11,27%**. Trois runs et leurs dispersions seulement, pas un intervalle de confiance ni une mesure de l'ensemble Tauri/Pi. Identité des10000lignes/révision client contrôlée ; même checksum serveur ci-dessus sur les six exécutions release,100pages et delta1. Logs privés `release-rss-{client,server}-{baseline,final}-{1,2,3}.log`, sans données utilisateur.

## Stockage

Fixture initiale : deux fichiers identiques de31732octets dans deux profils. Estimation allouée65536→32768octets, mêmes deux chemins et contenus, test de répétition stable. Ce premier résultat synthétique est distinct du cache réel ci-dessous.

Cache utilisateur : application ouverte détectée, fermeture confirmée et processus recontrôlés juste avant mutation ; revues sourcef980 approuvées. Migration réelle rc0, **3926/3926références d'images** contrôlées, même total logique173576179octets. Allocation images181870592→32976896octets ; totalité des fichiers du cache (inodes uniques) **198754304→49860608octets**, **189,55→47,55Mio**, **−148893696octets / −142,00Mio**. Ce n'est pas la somme des tailles logiques de chemins hardlinkés : chaque inode est compté une fois.

Ledger privé avant/après : **3930fichiers historiques** gardent tous contenu/taille/mode ; pour les fichiers non-images, inode/date également inchangés. **600blobs** ajoutés seulement. SHA manifestes9afe38307b44911e376f6eb049b0a3667d4e3dc3a42d1f3302da280c256843f8 et1d31a2a22608cb8b03633a8ae87781b1f86a89654f2037c6ee1f00a7628852b2 ; aucune donnée du foyer publiée. Vérificateur indépendant compare après→courant4530fichiers sans changement, même allocation. Aucun catalogue hors ligne, paramètre, WAL/SHM, historique ou version livrée supprimé. APFS/clones/snapshots : `st_blocks` estime l'allocation des fichiers, pas l'augmentation exacte du volume libre.

## Livraison et limites

Source figée **f9803557a452f59cbbe3cbf5b1c4bf1b48a9aa67**, [revues](../reviews/resource-optimization-f980355.md) APPROVED_SCOPED après correction de BUG0103 et Clippy ;51tests client/137serveur, API76 et Clippy-Dwarnings PASS,8tests cache ciblés, deux benchmarks ignorés de la suite routinière et exécutés séparément. [Bundle et déploiement](../releases/REL-RESOURCE-OPTIMIZATION-001.md), [vérification indépendante](../releases/REL-RESOURCE-OPTIMIZATION-001-verification.md) APPROVED_SCOPED, Pi mis à jour avec sauvegarde exacte, web/config/NAS/TLS inchangés. Aucun nouveau lecteur général, fournisseur, scan NAS ou benchmark de flux réels demandé par ces optimisations.
