# Livraison CC-PLAY-001 — flux HTTPS et reprise

2026-10-13 · auteur /root · DEV_FAST, G-03 validé, G-04 non remis. Base84b08b1645a0c6fbccd17012eb7cbcd0fc7ad74a, candidat initial761496fa41d994002cf57d0a176bafebac92fdd3, **source finaleb4100a37f985943fa1932bd98a2ec99d70b6c044**, branche codex/player-streaming. Le carrier documentaire suivant ne change pas la source revue.

## Résultat disponible

API et client Rust pour baux profil/appareil/média/révision/epoch, expiration30s/heartbeat, flux Range bruts, progression ordonnée et reprise. Vu après10s réelles cumulées dans le dernier dixième, prorata de frontière, vitesses valides, pauses/seeks/buffering interrompent le crédit. Aucun transcodage/accès NAS client, pas de nouvel auth ni secret. API réelle et exclusions : [PLAYBACK-API](PLAYBACK-API.md) ; plan : [PLAYBACK-PLAN](PLAYBACK-PLAN.md), ADR0008 et BUG0012–14.

## Environnement et commandes exécutées

Darwin arm64, rustc1.99.0 (b940084d7,2026-09-28), ffmpeg9.0.2, libmpv Homebrew0.41.0_10. Versions tierces Cargo inchangées ; lock ajoute seulement dépendance interne chill-domain du client et tokio dev de l'exemple lecteur. CARGO_HOME=/private/tmp/video-catalog-cargo, RUSTUP_HOME=/private/tmp/video-catalog-rustup, CARGO_TARGET_DIR=/private/tmp/chillcenter-implementation.6HSJzC/repo/target ; cargo dans le PATH de ce runtime. Checkout privé /private/tmp/chillcenter-branding.OJILXG/repo.

Au SHA final, exit0 pour chaque commande :

```sh
cargo test --workspace --locked --offline --quiet
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo build --workspace --locked --offline
cargo build --locked --offline -p chill-player --example https_playback_smoke
git diff --check
```

146/146 PASS : catalogue65/client19/domaine13/lecteur11/serveur38. Aucun skip ou test quarantiné. Les tests clock et router prouvent bail expiré/génération/replay/isolation, ETag/octet206/412/416, POSIX symlink/parent traversal/FIFO/directory, révocation par release/racine/fichier/profil,1ms excédentaire rejetée et vitesse×0,5/×1/×2. Deux leases router/Store à×2 attendent chacun5s réelles :5000ms/nonvu puis10000ms/vu, autre profil0. Quatre jobs réellement bloqués : cinquièmeMEDIA_BUSY, timeout5s de chacun sans rendre les slots avant leur retour. Huit corps Range : neuvièmeMEDIA_BUSY, restitution sur drop et garde indépendant sans polling (release/racine/profil/idle5s), ancien corps repris refusé.

Observation outillage : test lecteur historique sous sandbox peut émettre IOSurface/VideoToolbox/« no frame » sans échec. Ce test ne certifie ni accélération matérielle ni image rendue. Aucun warning dans le smoke HTTPS réel final ; sa preuve reste headless, pas une surface graphique.

## Smoke service HTTPS réel

Fixture privée /private/tmp/chillcenter-playback-r2.I0NlUl, base/profils neufs, serveur loopback127.0.0.1:18545. Vidéo30s160×90/10fps H264+AAC+mov_text générée localement par ffmpeg, SHA256 `74b6529939b19d2aef3789160ba3e49ebe4928a130c9a8b57cede9aa6d431422`. ffmpeg utilisé uniquement pour fabriquer la fixture, jamais par le serveur en lecture.

```sh
chill-server --config /private/tmp/chillcenter-playback-r2.I0NlUl/config.json
https_playback_smoke https://127.0.0.1:18545 --fixture-consent
```

Serveur/exemple reconstruits au SHA final. Approbation SPKI explicite du seul certificat de fixture, pas de bypass TLS/curl-k ; exemple refuse hôte hors loopback ou profils préexistants. HttpMediaFactory fournit le média à libmpv par callback Rust ; l'exemple ne lit pas le fichier vidéo sur le stockage. Résultat exit0 :

`HTTPS_PLAYBACK_OK: pinned TLS, real MP4/audio/subtitles, seek, natural EOF, heartbeat, resume across devices, profile isolation, stale lease refusal`

Range contient ftyp ; chargement et pistes audio/sous-titres intégrés présents ; seek12s contrôlé ; pause/position enregistrée ; seek près de fin et lecture jusqu'à Ended reason0/error0 ; nouvelle génération sur second appareil simulé récupère12s ; ancien événement/flux refusé, second profil0. Le test sauvegarde explicitement un événement de pause, pas encore un mécanisme de progression automatique UI. Tests serveur et client distincts, aucun accès au NAS du foyer/Pi.

## Revue et portée

R1 CHANGES_REQUESTED conservées ; EOF et capacité reproduits RED puis GREEN. Revues indépendantes r2 [code](../reviews/PLAYBACK-code-r2.md), [tests](../reviews/PLAYBACK-tests-r2.md), [sécurité](../reviews/PLAYBACK-security-r2.md) APPROVED sur sourceb4100a3. Reviewer non auteur, aucun fichier modifié par eux ; rapports transcrits par /root sans auto-approbation.

**Non livré/prouvé** : fenêtre lecteur Tauri/AppKit, UI piste/vitesse/seek, progression/heartbeat automatiques et échéance locale, transfert/SSE/ACK/enchaînement, sous-titres externes, matriceMKV/AVI, bundle libmpv final, serveur Linux ARM/Pi/NAS/Wi-Fi/RSS/CPU/2–3 flux1080p. Le paquet .app du lot interface précédent n'est pas remplacé et n'a pas de lecteur. Ne pas fermer US016/US021/ENABLER005 sur ce checkpoint.

## Remise et retour

Remise locale uniquement au dépôt ChillCenter, contrôle préalable clean + HEAD84b08b attendu, fast-forward codex/player-streaming ; aucune réinitialisation, pushGitHub ou opération NAS/Cinéfolio. Serveurs loopback temporaires arrêtés après vérification. IPchillcenter.local et racine/srv/volumes/video restent des exemples configurables ; Pi déclaré non prêt.

Rollback réversible : nouvelle branche à84b08b, utiliser ancien serveur/client ; arrêter lecteurs incompatibles. Baux volatils oubliés au restart, schéma compatible sans migration destructive. Sauvegarder données/réglages avant futures migrations. Les LOW précédents UISEC002/SRVSEC005 ne sont pas corrigés par ce lot.
