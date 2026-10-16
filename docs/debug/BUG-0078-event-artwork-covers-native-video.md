# BUG-0078 — Le décor événementiel masque la vidéo native

**CLOSED — confirmation utilisateur du 16 octobre 2026 : « bugs 78, 79/80, 85/86/87 et 88 corrigés ».** Clôture sur ce retour humain après la remise df2870f ; aucune nouvelle exécution de test revendiquée pour cette clôture. Les statuts et limites ci-dessous conservent les checkpoints précédents.

- Signalement utilisateur : 16 octobre 2026, **BLOQUANT**. Noël ou Halloween recouvre la vidéo pendant la lecture.
- Attendu : image native visible, contrôles web au-dessus ; décor absent du panneau de lecture puis restauré au retour au catalogue.
- Source diagnostiquée : branche `codex/fix-player-event-overlay`, base `fd2b7a961f7fc9212c3208377cb9f4e61d306e27`. Version exacte de l’application du signalement non fournie ; pas de supposition sur le bundle lancé.
- Instance diagnostic : `/root`. Environnement local : vrai DOM connecté avec pont synthétique, Chromium ; aucun serveur ni média utilisé.
- Statut diagnostic : **ROOT_CAUSE_CONFIRMED**. Pas encore une validation humaine de lecture Tauri.

## Hypothèse et expérience

H1, seule hypothèse : `events.css` maintient `.workspace::before` pendant `body.player-open`. Le `z-index:-1` ne suffit pas : le fond est peint dans WKWebView, qui se trouve volontairement au-dessus de la vue OpenGL/libmpv native (`apps/desktop/src/native_player.rs`, montage et remontage). Les autres fonds de la découpe sont déjà transparents dans `player.css`, pas ce pseudo-élément.

Prédiction : en état de lecture, le style calculé conserve le contenu et le PNG événementiel. Test ciblé `scripts/test-event-player-cutout.cjs`, sans démarrer de vidéo : **RED**, `halloween/light/right-pane: decoration still paints over native video`.

Expérience causale : ajout d’une seule règle CSS en mémoire dans le navigateur de test, supprimant contenu et image du pseudo-élément seulement pendant `body.player-open`. **GREEN** sur Noël/Halloween × clair/sombre × panneau droit/plein écran, avec retour du décor à la fermeture et transparence de la découpe conservée. Aucune modification produit pendant le diagnostic ; injection temporaire détruite à la fermeture du navigateur.

## Handoff causal

Correction autorisée au builder distinct `/root/cleanup_client_modules` : seule couche `ui/connected/events.css`, désactiver la peinture du décor pendant la lecture ; ni moteur vidéo, ni API, ni protocoles, ni autres bugs à modifier. Preuve RED/GREEN dans le test ciblé, puis régression des ambiances. Ne pas rejouer un parcours vidéo complet.

Correction du builder : règle ciblée dans `events.css`, seul fichier produit modifié. Test ciblé **GREEN** sur les huit combinaisons et régression `test-appearance-v05.cjs` PASS, sans lecture, serveur ou action native. Statut **FIXED / VERIFIED_DOM** ; rendu vidéo Tauri réel à confirmer humainement. Le verdict de revue antérieur reste lié à son ancien SHA ; le correctif exige sa propre revue. Retour arrière : revert de ce seul changement CSS sur cette branche ; conserver les profils, réglages, catalogue, NAS et historiques.
