# Debug Session: BUG-0046

- **Symptom**: le lecteur natif occupe toute la vue de contenu en plein écran, mais sa hauteur reste inférieure de 34 points à la hauteur de l'écran.
- **Expected / Actual**: attendu : surface vidéo AppKit de `1710 x 1107` points ; observé après détection du masque plein écran puis 500 ms : `1710 x 1073` points, à l'origine locale `(0, 0)`. La surface et sa vue parente ont exactement la même géométrie.
- **Environment, Branch, Commit**: macOS Apple Silicon, branche `codex/cinefolio-ui-rework`, commit `9aec0512eb7c8d2d911fedc00707c04cdc7a6a1e` avec modifications UX/API non commitées préexistantes.
- **Base Commit / Artifact Revision or Hash**: `9aec0512eb7c8d2d911fedc00707c04cdc7a6a1e` / script de diagnostic `ui/connected/player-qa.js` diff SHA-256 `841a239bc751a511c69e54e078f79257fce984e201b4887c3ec2c826d660e152`.
- **Debugger Instance**: `/root/fullscreen_diagnostic`
- **Started At UTC**: `2026-10-14T15:40:11Z`
- **Reproduction**: démarrer le QA Tauri réel en mode `CHILL_UI_QA_MODE=player` contre le serveur de fixture local `https://127.0.0.1:18548`, attendre le chargement vidéo, demander le plein écran, attendre que `NSWindowStyleMaskFullScreen` soit présent, attendre 500 ms, remonter la surface sur le rectangle DOM puis comparer `debug_surface.frame`, `bounds` et `screen`.
- **Regression Test Before Fix**: `ui/connected/player-qa.js`, étape `fullscreen geometry`, sortie 1 : `frame={height:1073,width:1710,x:0,y:0}`, `bounds={height:1073,width:1710,x:0,y:0}`, `screen={height:1107,width:1710,x:0,y:0}`. Toutes les étapes antérieures — image, superposition, inactivité, pistes, pause/reprise, navigation, vitesse et volume — passent.
- **Diagnostic Status**: `ROOT_CAUSE_CONFIRMED`

## Hypothesis Ledger

| # / UTC | Clean baseline before | Falsifiable hypothesis | Predicted observation | Exact experiment + changed environment/data | Exit/result | Observed result | Raw evidence | Verdict | Probes removed / baseline restored |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1 / 2026-10-14T15:40:11Z | Commit et diff ci-dessus ; aucune modification Rust/JS par le debugger | La transition AppKit n'est pas stabilisée 500 ms après l'apparition du masque plein écran. | Pendant 5,4 s de snapshots espacés de 300 ms, la hauteur `frame`/`bounds` doit évoluer de 1073 vers 1107 points. | Remplacer temporairement l'assertion immédiate par 18 cycles : attente 300 ms, commande `mount` avec le rectangle DOM courant, trace horodatée de `frame`/`bounds`/`screen`. Aucun correctif produit. | Sortie 1, même assertion finale | Les 18 échantillons restent identiques : surface `1710 x 1073` à `(0,0)`, écran `1710 x 1107` à `(0,0)`. Aucune évolution en 5,4 s. | Sortie `UI_QA_RESULT false` du rerun coordinateur ; échantillons 0 à 17 identiques. | `REFUTED` | Oui : boucle temporaire retirée par le coordinateur ; assertion enrichie conservée comme régression. |
| 2 / 2026-10-14T15:43:41Z | H1 réfutée ; géométrie stable pendant 5,4 s ; surface et parent identiques | AppKit/Tauri maintient la vue de contenu dans la zone sûre située sous l'encoche ou la barre système ; les 34 points ne proviennent donc pas du montage de la surface vidéo. | La fenêtre est en plein écran système, tandis que les limites sûres de l'écran expliquent exactement les 34 points manquants ; la surface vidéo continue de correspondre à 100 % à sa vue parente. | Ajouter uniquement au snapshot de diagnostic les limites sûres calculées depuis `NSScreen.safeAreaInsets`, puis exécuter une fois au même point plein écran. | Sortie 1 sur l'ancienne assertion physique | Surface `1710 x 1073`, écran physique `1710 x 1107`, limites sûres `1710 x 1073,5`, toutes à `(0,0)`. La différence de 33,5 points explique la hauteur AppKit arrondie à 1073 ; le masque plein écran système est présent. | Snapshot Tauri local réel : `frame={height:1073,width:1710,x:0,y:0}`, `screen_safe_bounds={height:1073.5,width:1710,x:0,y:0}`, `screen={height:1107,width:1710,x:0,y:0}`. | `CONFIRMED` | Oui : sonde temporaire retirée après capture ; aucun diff Rust/JS restant. |
| 3 / | | | | | | | | | |
| 4 / | | | | | | | | | |
| 5 / | | | | | | | | | |

## Confirmed Cause and Causality

- Confirmed hypothesis: 2
- Enabling/removing the condition changes the defect: la surface passe déjà l'oracle correct `frame == screen_safe_bounds` à 0,5 point près et remplit exactement sa vue parente. Seul l'oracle erroné `frame == NSScreen.frame` échoue de 34 points sur l'écran à encoche.
- Platform contract: `NSScreen.safeAreaInsets` décrit la zone sûre de l'écran ; référence AppKit : <https://developer.apple.com/documentation/appkit/nsscreen/safeareainsets>.
- Temporary probes removed and baseline restored: oui ; `git status` ne montre aucune modification Rust/JS issue des expériences.

## Causal Correction Handoff

- Authorized minimal correction derived from cause: dans le code de diagnostic uniquement, exposer les limites sûres de l'écran et faire vérifier au QA plein écran que la surface remplit ces limites avec une tolérance d'arrondi ; conserver les assertions existantes de remplissage du parent, ordre natif et hit-test. Supprimer toute mesure temporaire non utilisée.
- Explicitly out of scope: forcer le contenu sous l'encoche, modifier le comportement AppKit/Tauri de production, refactorisation, NAS, données utilisateur
- Target builder instance: distinct `agent-code-builder` à désigner par le coordinateur
- Builder evidence destination: handoff au coordinateur `/root`
- Consuming triple-review gate to invalidate: candidat courant, identifiant à renseigner par le coordinateur

The debugger's report terminates here. The builder records red-to-green and rerun evidence in its own handoff; closure occurs only after current independent approvals and integration.

## User Handoff After Five Unconfirmed Hypotheses

Sans objet : cause confirmée à l'hypothèse 2.
