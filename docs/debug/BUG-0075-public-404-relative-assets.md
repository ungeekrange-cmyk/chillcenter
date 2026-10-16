# BUG-0075 — Style 404 perdu sous une URL imbriquée

16 octobre 2026, site public local proposé ; **VERIFIED_LOCAL**, non publié. Revue indépendante du candidat `9274d5c` : le lien relatif `styles.css` se résout sous le chemin manquant lorsque Pages sert la page 404 pour une URL imbriquée. Cause confirmée par résolution d’URL, pas un incident déployé.

Correction `df95432` : page d’erreur autonome avec CSS clair/sombre intégré et lien d’accueil absolu. Régression `scripts/test-public-site.mjs` sur trois URL manquantes, dont un chemin terminé par `/`. Test navigateur local des trois pages, deux tons et quatre largeurs : PASS. Le comportement sur Pages réel reste à vérifier après une éventuelle publication autorisée.

Retour arrière : revenir au commit parent sur cette branche uniquement ; aucun état utilisateur ni serveur concerné. Revue code finale couvrant la correction : APPROVED au candidat `2cb6f73`.
