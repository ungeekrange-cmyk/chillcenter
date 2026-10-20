# API Chill Center

Le contrat public est versionné au **2026-10-14**.

- [Guide d’intégration tiers](INTEGRATION-FR.md)
- [OpenAPI 3.1](openapi-2026-10-14.json)

La seule base servie est `/api/2026-10-14`. Depuis le serveur0.7.1,
`/api/v1` est supprimé (404) ; les intégrations tierces doivent utiliser la date.
Il s’agit d’une API HTTPS LAN sans authentification ni CORS :
ne pas l’exposer sur Internet.
