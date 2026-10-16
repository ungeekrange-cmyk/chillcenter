# ADR-0018 — Écoute HTTP non chiffrée optionnelle pour le client web (réseau local)

- Status : `PROPOSED` ; date : `2026-10-15`. Passe à `ACCEPTED` après revue indépendante des octets déployés et déploiement vérifié.
- Decision Owner / Author Instance : `/root` ; Reviewer Instance : à désigner (≠ auteur).
- Branche : `codex/web-pwa`.
- Supersedes / Superseded By : `NONE` / `NONE`. **Complète** ADR-0005 (HTTPS) et ADR-0017 (port 443) sans les remplacer : le listener HTTPS, son certificat et l'approbation d'empreinte du client Mac sont inchangés.
- Related Items : [WEB-CLIENT](../WEB-CLIENT.md), [PI-LIVE-STATE](../../deployment/PI-LIVE-STATE.md).

## Contexte

Le client web (ADR-0014) est servi en HTTPS avec le certificat auto-signé du serveur. Demande de l'utilisateur (15 octobre 2026) : **les autres membres de la maison doivent pouvoir l'ouvrir**, sans rien installer sur leurs appareils. Contraintes constatées :

- un navigateur n'accepte sans avertissement ni installation d'autorité qu'un certificat de confiance publique, ce qui exige un nom de domaine (et des renouvellements par l'API DNS) ; l'utilisateur **refuse d'installer une autorité privée** sur les autres appareils ;
- l'utilisateur ne fournit pas de domaine : **« fais du HTTP non chiffré sinon »** ;
- ADR-0005 impose HTTPS pour l'API et le client Mac ; le serveur n'a **aucune authentification** (périmètre familial, réseau local) ; l'écoute doit rester sur une adresse locale (`validate`, `is_local_address`).

Constats techniques sur une origine HTTP dont l'hôte est une adresse LAN (non « potentiellement fiable » pour les navigateurs), vérifiés dans un navigateur réel le 15 octobre 2026 : pas de contexte sécurisé (`isSecureContext` faux), pas de Service Worker, pas d'installation en PWA, pas de `crypto.randomUUID` ; `MediaSource`, `WebAssembly`, les Workers, `fetch` et `EventSource` fonctionnent. **Les navigateurs n'envoient pas `Sec-Fetch-*` à une telle origine** ; ils envoient en revanche `Origin` sur toute requête qui n'est pas un GET/HEAD.

## Options examinées

| Option | Pour | Contre |
| --- | --- | --- |
| A. Autorité privée maison | gratuit, TLS conservé | refusée par l'utilisateur (installation sur chaque appareil) |
| B. Domaine + Let's Encrypt (DNS-01) | reconnu par tous, TLS conservé | exige un domaine (coût) et un accès internet du Pi pour les renouvellements ; non disponible |
| C. Ne rien faire | aucun risque nouveau | le client web reste inutilisable pour les autres membres |
| **D. Second listener HTTP clair, optionnel, pour le client web** | aucun prérequis ; fonctionne sur tout appareil de la maison | trafic non chiffré sur le réseau local ; pas de PWA ni de Service Worker |

## Décision

Option **D**, strictement bornée.

1. **Configuration** : `ServerConfig.http_listen` (optionnel, absent par défaut, non sérialisé s'il est absent). Validation : adresse locale (privée, link-local ou loopback), jamais `0.0.0.0`/publique, port non nul et différent du port HTTPS. Sans ce champ, le comportement du serveur est strictement celui d'avant.
2. **Serveur** : un second `axum_server` sans TLS, avec le même plafond de 16 connexions, **HTTP/1 seulement** (pas de détection de protocole en clair), un délai de 5 s sur le premier octet et sur les en-têtes, le même routeur (API + fichiers statiques) et les mêmes gardes. Un arrêt de l'un des deux listeners arrête le processus (systemd le relance), pas de service dégradé silencieux.
3. **Garde d'origine consciente du schéma** : le routeur HTTP porte `Scheme::Http`. Sur ce listener, `Origin`, quand elle est présente sans `Sec-Fetch-Site`, doit être exactement `http://<Host>` (comparaison insensible à la casse) ; `Sec-Fetch-Site: cross-site/same-site` reste refusé ; le `Host` doit appartenir aux noms de `tls_names` et porter le port du listener HTTP **ou** le port 80. Sur le listener HTTPS, rien ne change : une `Origin` sans `Sec-Fetch-Site` y reste refusée.
4. **Client web** : `crypto.randomUUID` remplacé par un repli `getRandomValues` ; l'enregistrement du Service Worker reste réservé à HTTPS (déjà le cas).
5. **Réseau (Pi)** : écoute sur `chillcenter.local:8080` ; redirection nftables 80 → 8080 (même mécanisme que ADR-0017, table distincte `chillcenter_http`, unité distincte) pour que `http://chillcenter.local` fonctionne.

## Conséquences et risques acceptés par l'utilisateur

- **Confidentialité** : tout ce qui transite par le port HTTP (catalogue, profils, flux vidéo, identifiants de bail) est lisible et modifiable par n'importe quel appareil du réseau Wi-Fi/LAN. Le serveur n'ayant pas d'authentification, l'API est déjà joignable de tout le LAN en HTTPS ; la nouveauté est l'**absence de chiffrement et d'authentification du serveur** (un appareil du réseau peut se faire passer pour le serveur ou altérer les pages servies, donc exécuter du JavaScript dans l'origine du client web).
- **Surface exposée** : le listener HTTP sert la même API que le HTTPS (catalogue, profils, bibliothèque, lecture, administration de la bibliothèque), pas seulement les fichiers du client web. Pour limiter les secrets en clair, **les routes qui portent la clé TMDB ou des archives (`/admin/tmdb*`, `/admin/exports*`, `/admin/imports*`) sont refusées sur ce listener** (`HTTPS_ONLY`) ; elles restent disponibles en HTTPS, le client web ne les utilise pas. Restent joignables en clair, sous la garde d'`Origin`, d'autres routes d'écriture qui ne portent pas la clé : `PUT /admin/library`, `POST /admin/scan`, `/admin/reassociations` et les routes TMDB par média (`/admin/profiles/{p}/media/{m}/tmdb*`). Ne jamais saisir de clé ni d'archive via le port HTTP.
- **Disponibilité** : le listener HTTP a son propre plafond de 16 connexions, avec un délai de 5 s sur le premier octet (la poignée de main TLS a le même rôle en HTTPS) ; un appareil du réseau qui envoie des en-têtes puis un corps qui n'arrive jamais peut néanmoins occuper les 16 places pendant 30 s (délai de requête), en boucle. Le HTTPS n'est pas touché. Le HTTPS n'est pas touché (réserves séparées).
- **CSRF / navigateur tiers** : une page web quelconque ne peut pas écrire (l'`Origin` est vérifiée) ni lire (pas de CORS) ; le contrôle `Host` écarte le DNS rebinding. Les GET sont sans effet de bord.
- **Fonctions web perdues sur ce port** : installation en PWA, Service Worker, plein écran persistant selon navigateur, Wake Lock. Le client HTTPS (`https://chillcenter.local`) garde ces fonctions pour qui accepte l'avertissement de certificat.
- **Le Pi ne doit pas être exposé à internet** : ce listener rendrait l'accès sans protection ; ne jamais ouvrir 80/8080 sur la box.
- **Rollback** : retirer `http_listen` de `/etc/chillcenter/server.json` (sauvegarde conservée) et la redirection 80 ; un binaire sans ce champ refuse sinon la configuration (`deny_unknown_fields`).
- **Non prouvé tant que non exécuté sur le Pi** : lecture de médias depuis un appareil de la maison en HTTP, comportement avec les navigateurs mobiles, redémarrage réel du Pi.
