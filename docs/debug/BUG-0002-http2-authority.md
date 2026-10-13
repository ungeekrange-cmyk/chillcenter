# BUG-0002 — validation du nom réseau incompatible HTTP/2

- Date : 2026-10-13 ; environnement local Mac ARM64, Axum 0.8.9, curl système, HTTPS sur loopback 18443. Aucun déploiement Pi.
- Branche `codex/implementation`, base `c8d2a4c`, serveur nouvellement écrit avant commit.
- Attendu : `/api/v1/server` répond quel que soit HTTP/1.1 ou HTTP/2 ; réel : HTTP/2 répond `400 HOST_REQUIRED`.
- H1 : la validation utilise uniquement `Host`, absent lorsque Hyper reçoit `:authority`. Expérience : même URL/certificat, `curl --http1.1` répond 200, curl négociant HTTP/2 répond 400 avec `HOST_REQUIRED`. Cause confirmée ; aucune hypothèse suivante.
- Correction : lire `Host` ou, à défaut, l'autorité de l'URI HTTP/2 ; appliquer la même liste de noms et le même port. Aucun assouplissement CORS/Origin ni acceptation globale d'un nom.
- Régression : requête HTTP/2 sans Host à l'URI HTTPS qualifiée ; résultat/smoke à consigner dans les preuves de livraison.
- Rollback : parent du candidat serveur, sans modification de vidéos ni base réelle.
