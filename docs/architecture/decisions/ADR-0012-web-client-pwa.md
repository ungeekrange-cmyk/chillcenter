# ADR-0012 — Client web PWA, calcul vidéo côté navigateur

- Status : `PROPOSED` ; date : `2026-10-14`. Passe à `ACCEPTED` après revue indépendante et validation utilisateur du périmètre ci-dessous.
- Decision Owner / Author Instance : `/root` ; Reviewer Instance : à désigner (≠ auteur).
- Branche / base : `codex/web-pwa` / `0d0da0e` (issue de `codex/player-streaming` ; le développement Mac parallèle n'est pas touché).
- Supersedes / Superseded By : `NONE` / `NONE`. Lève le report « client web ultérieur » de F-02 (SCOPE) sans modifier ADR-0001/0004/0005 : le client Mac et son lecteur libmpv restent inchangés.
- Related Items : ADR-0003 (baux), ADR-0005 (HTTPS), ADR-0010 (cache client), ADR-0011 (API datée) ; [WEB-CLIENT](../WEB-CLIENT.md).

## Contexte

Demande utilisateur (14 octobre 2026) : garder la version serveur, reproduire design et fonctionnalités du client Mac sous forme de web app/PWA déployée sur le Raspberry, sans authentification, avec **presque tout le calcul (démux, décodage, rendu) côté client** et un serveur aussi léger que possible, en respectant les versions d'API exposées (`/api/2026-10-14`, alias `/api/v1`).

Contraintes constatées dans le code existant :

1. Le serveur refuse toute requête portant `Origin` ou `Sec-Fetch-Site` (`same_origin` dans `crates/chill-server/src/api.rs`, `BROWSER_ORIGIN_REFUSED`). Un navigateur envoie toujours `Sec-Fetch-Site` : aucune page web ne peut appeler l'API en l'état.
2. Le serveur ne sert aucun fichier statique.
3. `GET /media/{id}/stream` exige les en-têtes `X-Chill-Session` et `X-Chill-Generation`, impossibles à positionner sur un `<video src>`.
4. `media_revision` est un `uint64` : `JSON.parse` le corrompt au-delà de 2^53 (guide API, « Entiers 64 bits »).
5. MKV/AVI et de nombreux codecs (AC-3/DTS, HEVC selon navigateur) ne sont pas lus nativement par `<video>`.
6. Le client Mac fait approuver une empreinte SPKI ; un navigateur délègue la confiance à son magasin de certificats.

## Options examinées

| Option | Intérêt | Risque / réversibilité |
| --- | --- | --- |
| A. PWA servie par le serveur Rust lui-même (même origine) | Pas de CORS, un seul certificat, un seul déploiement Pi ; contrat API inchangé | Petite modification serveur (fichiers statiques + garde d'origine) ; réversible par configuration |
| B. PWA sur un autre hôte/port + CORS | Serveur intact | Contredit « pas de CORS » ; élargit la surface web d'une API sans authentification ; refusée |
| C. Reverse-proxy sur le Pi (nginx) | Pas de code Rust | Paquet supplémentaire sur 1 Gio de RAM, deuxième certificat ; refusée |
| D. Transcodage/remux serveur pour le navigateur | Lecture universelle | Interdit (aucun transcodage serveur, Pi 3B+) |

## Décision

Adopter **A**.

1. **Serveur (delta minimal, aucune route d'API modifiée)** : dossier `<data_dir>/web` (absent par défaut = comportement actuel, aucun champ de configuration ajouté). S'il existe, le serveur sert ce dossier en `GET`/`HEAD`, lecture seule, sans traversée, sans listing, types MIME fermés, `Cache-Control: no-cache` sur `index.html`/`sw.js`. La garde d'origine accepte `Sec-Fetch-Site: same-origin` avec `Origin` absent ou égal à `https://<Host>` ; elle continue de refuser `cross-site`, `same-site` et tout autre `Origin` ; `none` (adresse saisie) n'est accepté qu'en `GET` sur les fichiers statiques, jamais sous `/api/`. Le contrôle `Host` est inchangé. Pas de CORS, pas de nouvelle dépendance.
2. **Client web** : application statique sans framework, réutilisant le HTML/CSS/JS de `ui/connected` (pas de fork : copie au build). `window.__TAURI__.core.invoke` est remplacé par un pont JS (`web/src/bridge.js`) qui traduit les `UiCommand` en `fetch` de l'API datée ; mêmes DTO, mêmes messages d'erreur, même machine d'états.
3. **Calcul côté client** : lecture par MediaSource Extensions alimentées par des `fetch` `Range` portant les en-têtes de bail ; démux/remux (MP4, puis MKV/AVI) et, si nécessaire, décodage logiciel WebAssembly exécutés dans le navigateur. Le serveur ne lit que des octets (ADR-0008 inchangé). Les moteurs sont isolés derrière une interface `Engine` ; le choix des bibliothèques WASM (licences, taille, performance) fera l'objet d'un ADR séparé après spike mesuré.
4. **Confiance TLS** : le navigateur gère le certificat. Le serveur reste en HTTPS autogéné ; l'utilisateur l'approuve une fois par appareil. La PWA affiche l'empreinte et l'échéance de `GET /server` à titre informatif ; elle ne prétend pas reproduire le pinning SPKI du client Rust. Limite : plusieurs navigateurs refusent Service Worker et installation PWA sur un certificat non approuvé par le système.
5. **Réglages locaux** (thème, ambiance, cache de lecture, dernier profil, `device_id`) : `localStorage` de l'origine, propres à l'appareil (F-20).
6. **IA** (ADR-0007) : la CLI Codex/Claude n'existe pas dans un navigateur ; fonction non portée, masquée dans l'interface web.

## Conséquences

- Charge Pi : quelques centaines de Kio de fichiers statiques mis en cache par la PWA ; aucun décodage. Les objectifs RSS serveur restent à mesurer avec le dossier web présent.
- Sécurité : API toujours sans authentification, LAN seulement. Autoriser `same-origin` n'ouvre pas l'API aux autres sites (ils produisent `cross-site`). Les navigateurs sans `Sec-Fetch-*` (qui n'envoient que `Origin`) restent refusés.
- Lecture : couverture dépendante du moteur client, **non prouvée** : MKV/AVI, AC-3/DTS, HEVC, sous-titres ASS, débit Wi-Fi. Un format non pris en charge produit un message explicite, jamais un transcodage serveur.
- Bail exclusif, heartbeat 5 s, progression, règle des 10 s (F-08), transfert confirmé : reproduits côté web depuis le contrat existant (`WatchClock` porté en JS).
- Les évolutions de `ui/connected` côté Mac sont reprises au build web ; divergences listées dans WEB-CLIENT.
- Rollback : supprimer le dossier `<data_dir>/web` ou revenir au binaire précédent. Aucune migration SQLite.

## Validation

À faire, non exécuté : revue indépendante de cette décision ; tests Rust de la garde d'origine et du service statique (`cargo` absent de l'environnement de rédaction) ; essai réel navigateur ↔ Pi après autorisation explicite de déploiement.
