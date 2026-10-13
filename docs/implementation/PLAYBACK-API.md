# CC-PLAY-001 — API réellement disponible

Contrats produit inchangés ; complément de mise en œuvre v1. Tous les appels utilisent le transport Rust SPKI existant. Un bail coordonne les lectures sur le LAN familial ; ce n'est pas une authentification.

- `POST /api/v1/profiles/{profile}/playback/acquire` : AcquirePlayback (UUID média/appareil, contexte standalone uniquement) → PlaybackGrant (bail30s, epoch/génération/révision, état de reprise). Second appareil du même profil :409 PLAYBACK_ACTIVE ; trois profils simultanés maximum. Heartbeat prévu toutes les5s.
- `POST …/heartbeat` : PlaybackControl → Lease renouvelé. `POST …/progress` : Progress → PersonalState enregistré. Séquence strictement croissante, bail/révision actifs, durée/vitesse validées et crédit borné au temps réel serveur ; première observation sans crédit, passage90% proratisé.
- `POST …/release` : ReleasePlayback avec progression finale facultative. Libération seulement après enregistrement réussi ; les anciens flux/événements sont refusés.
- `GET /api/v1/profiles/{profile}/media/{media}/state` : état personnel, sans chemin NAS.
- `GET /api/v1/media/{media}/stream` : headers uniques `X-Chill-Session`, `X-Chill-Generation`, `Range: bytes=start-end` ; `If-Match` facultatif au premier accès, ETag obligatoire dans le transport client pour les suivants. Plages simple/ouverte/suffixe,206 et Content-Range ;416 avec taille,412 si fichier changé,409 si bail périmé. Métadonnées/fichier régulier vérifiés à chaque bloc64Kio ; pas de transcodage. Racine modifiée/profil supprimé/libération/expiration interrompent les futurs blocs.

Types partagés : chill-domain ; helpers async : chill-client::playback ; flux bloquant consommé par HttpMediaFactory libmpv sur son worker. Pas d'URL HTTPS libre ou de chemin du stockage dans le lecteur.

Non implémentés ici : transfert confirmé/SSE/ACK, progression automatique du lecteur Tauri, pause locale avant expiration, contexte série/saga/playlist, enchaînement, fichiers sous-titres externes. Les sous-titres intégrés sont décodés par libmpv depuis le média brut ; l'essai headless n'est pas une UI de sélection des pistes.

Ressources :4 jobs fichiers,8 flux actifs maximum ; après5s sans demande de bloc ou révocation/expiration du bail, un garde indépendant rend la capacité et interdit la reprise du corps ancien. Ces limites sont des contraintes de départ, pas des mesures Pi/Wi-Fi. EOF connue côté adaptateur retourne0 au callback mpv, sans requête416 inutile.
