# Revue indépendante tests / intégration — CC-UX-002 et API 2026-10-14

- **Review Mode** : `INTEGRATION_REVIEW`
- **Branche et commit** : `codex/cinefolio-ui-rework` / `9277ac4d51281b3e6d72475294b1e7822cb8e913`
- **Base historique** : `5665f60bc59ceb3b2a9caa1622af053af0645d47`
- **Reviewer Instance** : `/root/ux_api_test_review`, distinct des auteurs, du debugger, des builders et de l'integration manager
- **Verdict** : **APPROVED**

Le candidat exact ferme les deux findings de la revue précédente. Le chargeur catalogue réellement livré est couvert sur plusieurs lots et contextes périmés. Le lecteur natif a produit deux parcours complets, frais, consécutifs, sans retry ni skip, sous watchdog externe 120 s : fixture minimale puis fixture représentative. Les deux anciens timeouts restent dans l'historique ; leur condition de page WKWebView cachée est désormais reproduite, causalement retirée dans le chemin QA debug et non observée lors des deux parcours d'acceptation.

## Preuves reproductibles au SHA exact

| Vérification | Résultat | Détail |
| :--- | :--- | :--- |
| `cargo test --workspace --all-targets --locked --offline --quiet` | **PASS** | 289 réussis, 0 échec, 0 ignoré ; 33,86 s. Exécution hors confinement uniquement pour les deux tests de sécurité ouvrant un socket loopback. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | **PASS** | 0 warning ; 4,99 s. Le finding intermédiaire `collapsible_if` du SHA 6bf n'existe plus. |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` | **PASS** | 47 réussis, 0 échec/skip/todo ; 0,35 s. |
| `node scripts/validate-api-contract.mjs` | **PASS** | 66 chemins, 85 opérations inspectées. |
| `openapi-spec-validator==0.9.0` | **PASS** | `OPENAPI_FORMAL_OK`; document inchangé, SHA-256 `d12c606e4d922c1a52f0fe6f12f72940b1498c442dce7ba43ac9f83f946778f4`. |
| `git diff --check`, statut exact | **PASS** | Aucun défaut de whitespace ; worktree propre à `9277ac4`. |
| Tauri player A — fixture minimale | **PASS** | Processus frais PID 59168, serveur/DB/client neufs, 1 MP4 + SRT, port 18550 ; départ 17:30:31.890, PASS 17:30:58.909, 27 057 ms ; aucun retry/skip/timeout. |
| Tauri player B — fixture représentative | **PASS** | Processus frais PID 59272 immédiatement après A, serveur/DB/client distincts, 4 médias synthétiques et état série préchargé par Shell QA, port 18551 ; départ 17:30:59.039, PASS 17:31:25.062, 26 075 ms ; aucun retry/skip/timeout. |

Le wrapper `/private/tmp/chillcenter-native-watchdog-20261014.mjs` imposait 120 s par parcours ; aucun signal ni timeout. Les deux journaux traversent `visible-wait`, `visible-ready`, `boot`, connexion/profil/racine, chargement natif, image non vide, z-order/hit-test, inactivité/focus, pistes et reprise, seek, navigation, vitesse, volume, plein écran, géométrie/safe area, Escape, dix dernières secondes, EOF/vu et réouverture. `document_hidden=false` à toutes les phases. Les frames et safe bounds observés correspondent (`1710x1073` contre `1710x1073.5`) et les indicateurs natifs de hit-test/z-order sont vrais.

Une relecture indépendante des SQLite de preuve confirme les effets terminaux :

- minimale : 1 profil, 1 média, 1 état, 1 vu ; `Native QA`, position 120 000 ms, fenêtre finale 11 635 ms ;
- représentative : 2 profils, 4 médias, 3 états, 1 vu ; `Native QA`, position 120 000 ms, fenêtre finale 11 701 ms.

Les preuves HTTPS/OpenAPI et shell Tauri antérieures restent applicables aux chemins produit inchangés : 30 réponses JSON réelles plus Range 206/32 octets, 38 payloads validés OAS et shell Tauri intégré. Le delta courant touche le chemin QA debug, ses assertions et la documentation, pas les handlers API ni l'interface produit.

## Qualité des assertions et fermeture des findings

### `TEST-F-001` — fermé

Le rapport `/private/tmp/chillcenter-ux-native-timeout-diagnostic.md` applique cinq hypothèses bornées. H5 reproduit la condition : cacher l'application rend `document.hidden=true` et suspend l'attente timer ; le seul `unhide` + activation de la propre application remet le document visible et libère le timer déjà en attente. La correction :

- est compilée uniquement sous `debug_assertions` ;
- prépare l'application et la webview QA avant `webview.eval` ;
- attend `visibilitychange` sans polling/timer caché avant toute phase sensible ;
- conserve les assertions produit d'inactivité, pointeur, focus, rendu, lecture et EOF ;
- n'allonge pas le watchdog, ne modifie ni `player.js`, ni CSS, ni renderer, ni temporisateur produit.

Les deux parcours frais complets satisfont le critère convenu pour requalifier les timeouts historiques en incidents non reproductibles sur le candidat corrigé. Le premier nouveau timeout sur un futur SHA doit rouvrir immédiatement ce finding ; aucun retry vert ne devra le masquer.

### `TEST-F-002` — fermé

Les quatre tests `catalog-loading.dom.test.js` traversent les fonctions livrées `catalogCommand`, `catalogFilters`, `catalogRequest`, `loadPage` et `renderCatalogWindow` :

- 103 médias, lots exacts `0/48/96`, ordre complet et identités uniques ;
- viewport DOM strictement inférieur à 48 cartes avec accès au dernier média ;
- déduplication d'un chevauchement de lots ;
- rejet des réponses tardives d'un ancien profil ou d'une ancienne requête.

## Limites explicites non bloquantes pour ce gate

- Fournisseurs TMDB et IA réels : `NOT_RUN` ; les preuves utilisent leurs seams privées/synthétiques et n'engagent aucun coût externe.
- Catalogue principal réel de 2 496/3 000 éléments dans une webview et mesure RSS : `NOT_RUN` ; la logique et la borne DOM sont couvertes.
- Le bundle final doit encore être inspecté sur l'artefact effectivement construit : architecture/signature locale, dylibs et lancement hors cache de développement.
- Le déploiement du SHA exact sur Pi/NAS, les deux/trois flux Full HD, le Wi-Fi, le son humain, HDR/HEVC, veille/réveil et multi-écran restent `NOT_RUN` pour la release verification.
- L'approbation du candidat ne transforme pas les anciennes limites REL-PI-001 en succès ; elles doivent être rapportées séparément à l'utilisateur.

## Disposition

`9277ac4d51281b3e6d72475294b1e7822cb8e913` est **APPROVED** pour poursuivre vers la construction et la vérification du bundle Mac, puis l'intégration/déploiement selon les gates distincts applicables. Cette approbation couvre la source et les preuves d'intégration décrites ; elle ne revendique ni le déploiement Pi/NAS ni les NFR physiques listées ci-dessus.
