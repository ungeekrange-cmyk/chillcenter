# Index des décisions techniques

[ADR-0020](ADR-0020-local-title-recommendations.md), IMPLEMENTED_DETAIL,16octobre2026 : recommandations fiche/conseil en rails, séries regroupées par identité sans épisodes, correspondances genres/réalisation/distribution calculées dans le miroir ou fournies au CLI sur demande explicite ; API historique et web installé conservés.

[ADR-0019](ADR-0019-explicit-series-selector.md), IMPLEMENTED_DETAIL,2026-10-15 : `selector=key` facultatif pour les nouvelles clés de séries ; titre historique par défaut sans préfixe réservé, aucune modification de l'ancien web. Remplace seulement la sélection implicite de l'ADR-0015-series-season-identity pour préserver l'API existante.

[ADR-0014](ADR-0014-background-catalog-deltas.md), ACCEPTED, 2026-10-15, `/root` : remplace le transfert complet/contrôle synchrone de l’ADR-0013 ; curseurs persistants, deltas et transfert détaché. Revue indépendante APPROVED sur11aa529.

[ADR-0015](ADR-0015-series-season-identity.md), ACCEPTED_SCOPED, 2026-10-15, `/root` : identité de série indépendante du titre, sources homonymes, métadonnées/affiches de saison séparées. Revue finale indépendante APPROVED_SCOPED sur11aa529.

[ADR-0013](ADR-0013-client-catalog-mirror.md), ACCEPTED,14octobre2026 : miroir SQLite et affiches persistées, API additive, opérations lecteur séparées ; revues indépendantes exactes APPROVED sur07358f7/7a0f480, réserve P2 nettoyage serveur documentée.

[ADR-0012](ADR-0012-client-player-and-collection-pages.md), IMPLEMENTED_DETAIL,14octobre2026 : plein écran initial réglable parMac, panneau droit intégral, collections et sélection par vignettes en pages. Correction utilisateur directe ; aucun changement moteur/serveur/API. Voir la release REL-MAC-UI-PANELS-001 pour les preuves, pas une validation visuelle humaine supposée.

Révision `CC-DOC-001`, 2026-10-13. Auteur et propriétaire : `/root`. Tous les ADR sont `ACCEPTED` par validation utilisateur de CC-DOC-001, sans prédécesseur ni successeur. La preuve est [G-02](../../gates/G-02-CC-DOC-001.md) ; les hashes historiques de revue portent sur la proposition approuvée. Les hash exacts de revue figurent dans les rapports.

| ID | Décision proposée | Exigences | Statut |
| --- | --- | --- | --- |
| [ADR-0001](ADR-0001-linux-rust.md) | Service Linux Rust, installation native et client distinct | F-01–04 | ACCEPTED |
| [ADR-0002](ADR-0002-data-identity.md) | SQLite locale, états par profil, UUID stables | F-05–06, F-14, F-16 | ACCEPTED |
| [ADR-0003](ADR-0003-playback-lease.md) | Bail exclusif et transfert confirmé | F-06–08, F-12–13 | ACCEPTED |
| [ADR-0004](ADR-0004-native-player.md) | Tauri conservé, lecteur libmpv intégré et transport Rust | F-02–03, F-09–11, F-22 | ACCEPTED |
| [ADR-0005](ADR-0005-https.md) | Confiance initiale et rotation TLS, exception après expiration | F-18–19 | ACCEPTED |
| [ADR-0006](ADR-0006-portable-import.md) | Archive portable JSON/affiches et import atomique | F-16–18 | ACCEPTED |
| [ADR-0007](ADR-0007-client-ai.md) | IA sur Mac, plans validés et appliqués par le serveur | F-14–15, F-18 | ACCEPTED |

Déclencheurs : frontière serveur/client, persistance, identité, contrat de session, dépendance native, chiffrement/exception, migration et fournisseur externe. Le calendrier et les palettes statiques sont des règles fonctionnelles déjà approuvées, documentées dans l'architecture, sans ADR distinct supplémentaire.

Après acceptation, changer une décision par un nouvel ADR avec `supersedes`, pas par une réécriture. Invalider les stories ou preuves qui consomment une décision modifiée.

Complément d'implémentation, distinct des sept décisions approuvées en G-02 : [ADR-0008](ADR-0008-bounded-media-io.md), `IMPLEMENTED_DETAIL`, I/O NAS bornées et ouverture par descripteurs, CC-PLAY-001. Ne remplace aucun ADR accepté ; pas de nouvelle validation utilisateur revendiquée.

[ADR-0009](ADR-0009-archive-validation.md), IMPLEMENTED_DETAIL : validation ZIP/images et application portable transactionnelle, revues indépendantes locales126da0a favorables. Aucun changement de l'ADR-0006 accepté ni nouvelle validation utilisateur supposée.

[ADR-0010](ADR-0010-client-playback-cache.md), ACCEPTED, CC-CACHE-002,2026-10-14, propriétaire/auteur/root : cible60 s avant/environ30 s arrière, défaut128/64 Mio, réglages locaux bornés16–512/0–256 Mio ; durée arrière variable. Supersedes: budgetclient§10ARCHITECTURE et preuves mémoireclient64Mio, pas ADR-0004 ; superseded_by:NONE. Revue initialeADR CHANGES_REQUESTED conservée, nouvelle revue/root/pi_integration_gate APPROVED624f244 ; [gate](../../gates/G-DEV-CC-CACHE-002.md), anciennes preuves client STALE pour ce delta.

[ADR-0011](ADR-0011-date-versioned-api.md), ACCEPTED, CC-API-2026-10-14,2026-10-14, auteur/propriétaire/root : base HTTPS `/api/2026-10-14`, alias v1 commun et OpenAPI/guide tiers. Supersedes/superseded_by:NONE/NONE ; revue indépendante APPROVED, proposition0a confirmée inchangée à9277ac4, [gate](../../gates/G-DEV-CC-UX-API-001.md). Demande utilisateur explicite, aucune modification des garanties TLS ou de l'absence d'authentification LAN.

[ADR-0014](ADR-0014-web-client-pwa.md), PROPOSED, CC-WEB-001, 2026-10-14, auteur/propriétaire/root, branche `codex/web-pwa` : client web PWA servi par le serveur (même origine), calcul vidéo côté navigateur, delta serveur minimal (dossier `web/` à côté du binaire, garde `Sec-Fetch-Site: same-origin`). Supersedes/superseded_by:NONE/NONE ; revue indépendante et validation utilisateur à obtenir, rien déployé. Voir [WEB-CLIENT](../WEB-CLIENT.md).

[ADR-0015](ADR-0015-web-playback-engine.md), PROPOSED, 2026-10-14, branche `codex/web-pwa` : moteur de lecture web sans dépendance (fetch/MediaSource, démux MP4 puis MKV en JS pur, le navigateur décode), à la demande « technologies stables et légères ». Complète ADR-0014, supersedes/superseded_by:NONE/NONE ; revue indépendante à obtenir.

[ADR-0016](ADR-0016-web-avi-xvid-decoder-evaluation.md), PROPOSED (évaluation seulement, aucune adoption), 2026-10-14, branche `codex/web-pwa` : coût d'un décodeur MPEG-4 ASP (Xvid) en WASM pour les AVI du client web ; recommandation A (message + client Mac) en attendant un spike chiffré. Supersedes/superseded_by:NONE/NONE.

**Renumérotation du client web (15 octobre 2026, après fusion de `codex/player-streaming`)** : la branche Mac avait déjà publié ADR-0012 (pages lecteur/collections) et ADR-0013 (miroir catalogue). Les ADR du client web, créés comme 0012, 0013 et 0014, sont devenus **ADR-0014** (client web PWA), **ADR-0015** (moteur de lecture) et **ADR-0016** (évaluation du décodeur Xvid). Les rapports de revue `docs/reviews/web-pi-deploy-*` sont conservés verbatim et citent encore les anciens numéros : ADR-0012 = ADR-0014, ADR-0013 = ADR-0015, ADR-0014 = ADR-0016 dans ces rapports.

[ADR-0017](ADR-0017-web-client-standard-https-port.md), PROPOSED, 2026-10-15, branche `codex/web-pwa` : client web sur le port HTTPS standard par redirection réseau 443 → 8443 sur le Pi, le contrôle `Host` du serveur acceptant aussi le port par défaut ; écarte un second listener (capacité réseau pour le service) et le déplacement du serveur (casse les clients Mac). Complète ADR-0014 et ADR-0005.

[ADR-0018](ADR-0018-web-client-plain-http-listener.md), PROPOSED, 2026-10-15, branche `codex/web-pwa` : écoute HTTP non chiffrée optionnelle (`http_listen`) pour que les autres membres de la maison ouvrent le client web sans rien installer ; refus de l'autorité privée par l'utilisateur, pas de domaine ; garde d'origine consciente du schéma ; risques de confidentialité acceptés et bornés au réseau local. Complète ADR-0005 et ADR-0017.
