# Revue sécurité accélérée R2 — macOS15 et reprises mesurées

- **Mode :** `DEV_FAST`, réévaluation accélérée locale et non publiée.
- **Base revue R1 :** `82836674ad1c7d0f12b8744d8e26e71cff14ce30`.
- **Candidat exact R2 :** `e54eb35726431bea2c01e6ad85d01e21da379de8`, branche `codex/macos15-build`.
- **Auteur :** `/root` ; **reviewer indépendant :** `/root/mac15_progress_security`.
- **Revu à :** `2026-10-21T19:06:50Z`.
- **Verdict :** **APPROVED**.

Le rapport R1 reste immuable, SHA-256
`4a97b442e13b8bf4bd531747afe9806a4bb7d135cfa6fd52dd15249a7186714c`.
Ce rapport couvre seulement les quatre fichiers du delta `8283667..e54eb357` et
réévalue le candidat exact ; il n'approuve ni les ADR rédigés par l'auteur, ni un
bundle, une publication, un déploiement ou un essai physique macOS15/26.

## Fermeture du finding R1

`MAC15-SEC-LOW-001` est **REMEDIATED** dans le candidat exact.

Après création des liens, `extract_selected` résout strictement leur cible
terminale et exige désormais simultanément :

- un parent exactement égal à `Cellar/<formule>/<version>/lib` ;
- un nom terminal finissant par `.dylib` ;
- un fichier terminal régulier.

Le cas R1 `lib/libmpv.dylib -> ../LICENSE.GPL` est maintenant refusé. Les liens
relatifs légitimes entre dylibs du même `lib/` restent acceptés. Les gardes
antérieures — racine isolée, traversal, liens sortants, hardlinks/fichiers
spéciaux, doublons, tailles et fermeture sans Homebrew global — restent
inchangées. Le test ajouté reproduit précisément le cas initial, sans élargir la
confiance à une sous-arborescence ou à un simple nom d'entrée d'archive.

## Rollback serveur et isolation du cache

Le second delta ne change ni API, autorisation, profil serveur, jeton de boot,
cookie, session, CSRF, redirect, log ou donnée persistée côté serveur.

Sur un `404/405`, uniquement si la réponse appartient encore au couple
`server_id:profile_id` et à la requête en attente courante, le client :

1. marque la capacité comme indisponible ;
2. vide la map et la révision en mémoire ;
3. retire exclusivement la clé `localStorage` de ce serveur/profil ;
4. redessine catalogue, barres et détail afin de ne pas afficher une mesure
   orpheline après rollback du serveur.

Une réponse tardive d'un ancien contexte ne franchit pas la garde
`context()===key && pending===task`. Une erreur transitoire telle que `503`
conserve au contraire map, révision et cache local pour le fonctionnement hors
ligne. Une action forcée peut retester un serveur remis à niveau et repeupler la
clé sans utiliser l'ancienne révision après rollback. Aucune suppression d'état
serveur ou d'un autre profil n'est introduite.

## Preuves reproductibles

| Outil/version | Commande/action | Résultat | Limite |
| :--- | :--- | :--- | :--- |
| Git 2.54.0 | `git rev-parse HEAD`; `git show e54eb357`; `git diff --check 8283667..e54eb357` | SHA/parent exacts ; PASS ; 4 fichiers, +36/-2 | Revue du delta uniquement. |
| Python 3.9.6 | `python3 -B scripts/helpers/test_macos_media_inputs.py` | 7/7 PASS | Archives temporaires ; aucun téléchargement ou paquetage réel. |
| Expérience adversariale R1 | TAR temporaire `lib/libmpv.dylib -> ../LICENSE.GPL` passé à `extract_selected` | `REFUSED: Dangling/escaping bottle library link` | Confirme la cause exacte ; la sonde Mach-O complète n'est pas rejouée. |
| Node 24.13.0 | `node --test ui/connected/progress.test.js web/test/bridge.test.js ui/connected/catalog-loading.dom.test.js` | 37/37 PASS | DOM/ponts simulés ; pas d'app native ou Safari physique. |
| Inspection de contexte | branches `404/405/503`, garde `context/pending`, clé cache et `redraw` | PASS : purge scoped sur rollback, conservation sur 503 | Revue statique complétant les tests ; pas de panne réseau réelle. |
| Scan ciblé | motifs clé privée/token/password sur `git diff 8283667..e54eb357` | Aucun résultat | Heuristique du delta, pas scan d'historique/entropie. |

## Findings et limites

Aucun finding Critical, High, Medium ou Low ne reste ouvert sur le candidat R2.
Le LOW R1 est fermé par code et régression exacts. Le delta n'ajoute aucune
dépendance, origine réseau, capacité Tauri, donnée sensible, exposition LAN ou
fallback Homebrew.

Le reviewer n'a exécuté aucun Cargo build/test, téléchargement de bouteille,
bundle/DMG, `codesign`, lancement d'app, connexion Pi/NAS, mutation GitHub/cloud
ou publication. Les preuves antérieures de 51 Mach-O, 50 dylibs et libmpv API2.5
ne sont pas réexécutées ici ; les chemins de bundle restent inchangés hors du
durcissement d'extraction. La conformité de licences et les essais physiques
macOS15/26 restent les gates de release explicités en R1. Toute modification de
source après `e54eb35726431bea2c01e6ad85d01e21da379de8` rend ce verdict obsolète.
