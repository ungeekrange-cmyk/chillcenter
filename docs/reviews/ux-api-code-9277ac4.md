# Revue delta indépendante — préparation visible du QA lecteur natif

- Verdict code : **APPROVED**
- Candidat exact : `9277ac4d51281b3e6d72475294b1e7822cb8e913`
- Base fonctionnelle : `db9455cf202a59be7ec17079ab3ad75cbcb07d44`
- Candidat intermédiaire non approuvé : `6bf1b20d92d598f5eeb6db9326720b33a6559161`
- Auteur(s) du delta : instances distinctes ; reviewer : `/root/ux_api_code_review`
- Revue UTC : `2026-10-14T17:27:47Z`
- ADR-0011 : contenu inchangé, approbation antérieure conservée sur SHA-256 `4e8456c60c1cc980a246f8f7192a15d642affce57a67346760ff49f49a9acaff`
- Périmètre : host Tauri debug, helper AppKit QA, readiness Page Visibility, tests et traçabilité. Aucun changement serveur, API, lecteur produit, renderer, CSS, temporisateur produit, dépendance ou donnée.

## Findings

Aucun finding ouvert sur le candidat exact.

## Finding intermédiaire corrigé

La première révision `6bf1b20` importait `objc2::runtime::AnyObject` sans garde alors que tous ses usages étaient dans le helper `#[cfg(debug_assertions)]`. Un build/clippy release avec `-D warnings` aurait donc rejeté l'import inutilisé. `55c2d0c` place maintenant cet import sous la même garde debug (`apps/desktop/src/native_player.rs:13-15`). La simplification de garde qui en résulte et son test source ont été alignés dans `9277ac4`. Ce finding historique ne s'applique plus au SHA revu.

## Analyse du correctif causal

### Confinement au QA debug

- Le hook qui appelle la préparation visible se trouve dans le builder entièrement protégé par `#[cfg(debug_assertions)]` (`apps/desktop/src/main.rs:548-574`) et ne s'active que lorsque `CHILL_UI_QA_URL`, `CHILL_UI_QA_ROOT` et le mode exact `player` sont présents (`:563-574`).
- Le helper natif porte lui-même `#[cfg(debug_assertions)]` (`apps/desktop/src/native_player.rs:308-330`). Les symboles et le comportement d'activation ne sont donc pas intégrés au chemin release.
- Aucun fichier `player.js`, CSS, moteur libmpv/OpenGL, watchdog, délai produit ou logique de lecture n'est modifié. Le changement ne masque aucune assertion QA.

### Propriété de l'application et thread principal

- Le helper obtient `NSApplication.sharedApplication`, c'est-à-dire l'application Chill Center du processus courant, puis lui envoie uniquement `unhide:` et `activateIgnoringOtherApps:`. Il ne cible ni PID, ni bundle externe, ni autre application (`native_player.rs:312-323`).
- Ces messages AppKit sont entièrement contenus dans `Webview::run_on_main_thread`; aucun pointeur Objective-C ne sort de la closure (`:312-324`). `show` et `set_focus` utilisent ensuite le dispatcher Tauri de la webview (`:325-330`).
- Même si l'envoi vers le main loop est asynchrone, l'évaluation QA ne démarre les phases sensibles qu'après l'oracle réel `document.hidden === false`. La garde écoute `visibilitychange` sans polling ni timer lorsque la page est cachée, puis émet `visible-ready` avant `boot` (`ui/connected/player-qa.js:4-12`). Elle rend la correction robuste au délai d'application des messages natifs sans bloquer le thread principal.
- Si la préparation native ne peut même pas être planifiée, le host produit un échec QA borné et n'évalue pas le parcours (`main.rs:568-573`). Aucun détail interne n'est exposé.

### Concordance avec le diagnostic

Le diagnostic H5 a reproduit la suspension en cachant l'application propre, puis a libéré le timer déjà en attente avec seulement unhide/activation. Le delta applique précisément cette condition causale, conserve l'inactivité/pointeur/focus et n'introduit ni modification GPU/Metal ni allongement du watchdog. L'échantillon historique `glReadPixels` n'est donc pas requalifié en deadlock corrigé : il reste une observation pendant la suspension du document.

Le test Node ajouté vérifie la séquence préparation host avant `webview.eval`, la garde debug et les quatre actions attendues, ainsi que readiness avant boot et absence de `setTimeout` dans cette garde (`ui/connected/player-qa.test.js:14-32`). Il s'agit d'un test structurel pertinent pour un harnais debug, complété — mais non remplacé — par les parcours natifs séparés.

## Vérifications exécutées

- `git rev-parse HEAD` : SHA exact ; `git status --short` : propre.
- `git diff --check db9455c..9277ac4` : PASS.
- `node --check ui/connected/player-qa.js` : PASS.
- Tests ciblés `player-qa.test.js` + `catalog-loading.dom.test.js` : **7 PASS**, 0 échec/skip.
- Lecture complète de `/private/tmp/chillcenter-ux-native-timeout-diagnostic.md` : H5 confirmé au cinquième essai, limite de cinq hypothèses respectée, probes retirées et source restaurée.
- Lecture de `/private/tmp/chillcenter-ux-visible-qa-builder.md` : correction, périmètre et limites conformes.
- Le builder rapporte `cargo check -p chillcenter-desktop`, 21 tests desktop et Node 46 PASS sur la révision fonctionnelle ; ces exécutions sont réutilisées comme preuve auteur, pas revendiquées comme exécutions du reviewer sur le SHA final.

## Limites et gates séparés

- Les rapports défavorables et timeouts des SHA précédents restent historiques ; aucun n'est transformé rétroactivement en PASS.
- Les deux parcours Tauri natifs frais sous watchdog 120 s étaient encore en attente au moment de cette revue. **Aucun PASS natif n'est revendiqué ici.** Le reviewer tests doit confirmer `visible-ready document_hidden=false`, progression au-delà de l'inactivité et fin complète sur le SHA exact.
- Le mode `player` observé est seul corrigé par cette garde ; ce rapport n'étend pas implicitement la preuve aux modes `next_series` ou transfert.
- Pi/NAS, Wi-Fi, deux vidéos Full HD, trois flux, fournisseurs TMDB/IA réels, RSS physique, codecs/HDR, son humain, veille et multi-écran restent hors périmètre.

## Disposition

Code approuvé pour `9277ac4d51281b3e6d72475294b1e7822cb8e913` uniquement. La promotion exige le verdict indépendant tests applicable au même SHA et les autres gates courants. L'ADR reste approuvée sur son hash documentaire inchangé. Cette approbation n'est transitive vers aucun SHA ultérieur et ne vaut ni PASS natif, ni validation Pi/NAS, ni autorisation de déploiement.
