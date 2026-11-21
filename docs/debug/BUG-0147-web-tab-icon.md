# BUG0147 — Favicon absente du client web

21 novembre 2026. Correctif publié dans web 0.7.12 et vérifié sur le Pi.

Le client web 0.7.11 du Pi, interrogé en HTTPS avec le certificat et
l’empreinte déjà approuvés, sert `/pwa-icons/icon-192.png` en 200
(`image/png`). Sa page déclare le manifeste et l’icône Apple. En l’absence
de lien `rel="icon"`, la requête par défaut vers `/favicon.ico` répond 404.

La construction web déclare le PNG existant comme favicon et ajoute
l’identifiant du build à son URL. Cette URL correspond à celle que le
service worker met déjà en cache. Le build 266ceb536ba4 et le test
`scripts/test-web-pwa.cjs` passent. Chrome charge et décode le PNG déclaré
dans le build local. Le Pi sert maintenant le build 78633fd17533. Le contrôle HTTPS a retrouvé le lien versionné et vérifié les octets du PNG et des autres ressources servies.
