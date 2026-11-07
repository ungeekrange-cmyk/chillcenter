# BUG-0136 — Curseur visible lorsque les commandes vidéo sont cachées

2026-11-07. Demande utilisateur sur web/macOS. Source initiale `df144e0`, intégrée
sur la branche courante en `f48cb4a`. La règle cursor:none ne visait que le
conteneur native-video ; le pointeur sur la couche WebKit, le video ou canvas
échappait au masquage. Étendre le sélecteur au stage et à ses surfaces de lecture,
strictement sous player-panel.controls-hidden. Aucun curseur système global.

35 tests DOM passent : expiration de l'inactivité, mouvement, pause et fermeture.
Revue indépendante APPROVED du delta df144e0 contre2b46d23 ; le hit-test Mac et la
couche WKWebView au-dessus d'OpenGL ont été relus. Pas de nouveau binaire Mac lancé
ni vérification visuelle de lecture réelle dans ce lot ; test UI synthétique.
Rollback : règle CSS du parent. Aucun changement de décodeur, API ou NAS.
