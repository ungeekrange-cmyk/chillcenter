# BUG-0149 — Logo du raccourci sur l’écran d’accueil

23 novembre 2026 · iPhone signalé, Android à vérifier sur appareil.

Le paquet web déclare un PNG Apple sans dimension à une URL relative. Il ne
contient ni `/apple-touch-icon.png`, adresse de secours de Safari, ni
`/favicon.ico`, demandé par certains navigateurs pour leurs raccourcis.

Le paquet fournit ces deux ressources à la racine et déclare leurs adresses
versionnées, avec la taille 180 × 180 de l’icône Apple. Le manifeste continue
de fournir les PNG 192 et 512 ainsi que l’icône adaptative. La vérification
Chromium contrôle les ressources de secours, le décodage et la mise en cache
des icônes déclarées. Le logo d’un raccourci déjà créé peut nécessiter de
recréer le raccourci après le déploiement.

[Documentation Apple sur les icônes d’applications web](https://developer.apple.com/library/archive/documentation/AppleApplications/Reference/SafariWebContent/ConfiguringWebApplications/ConfiguringWebApplications.html).
