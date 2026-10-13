# Sources et baseline

Consultation : 2026-10-13. Les observations locales, déclarations utilisateur et propositions techniques sont distinguées. Une source atteste une capacité ou un contrat ; elle ne prouve pas que Chill Center l'a implémenté.

## Sources locales

| ID | Source | Observation |
| --- | --- | --- |
| L-01 | `Video catalog/README.md`, `Cargo.toml`, `rust-toolchain.toml` | Cinéfolio 0.5.0, Rust 2024 / 1.99.0, Tauri 2, SQLite via rusqlite 0.40, reqwest 0.13 ; UI HTML/CSS/JS. |
| L-02 | `src/model.rs`, `src/database.rs` | `Media` porte aujourd'hui `watched` et `rating` ; collections communes ; stockage SQLite WAL de documents JSON. Ces champs doivent quitter le catalogue partagé. |
| L-03 | `src/scanner.rs`, `src/main.rs` | Identité fondée sur chemin relatif ; scan isolé ; lecture externe macOS. Ni profils ni serveur ni lecteur intégré dans la baseline. |
| L-04 | `src/assistant.rs` | Codex/Claude CLI, propositions structurées, limites 3 000 vidéos / 1 Mo de prompt, pas de vidéo, chemin ou secret dans le prompt. |
| L-05 | `docs/debug/BUG-NAS-ACCESS.md` | Incident NAS historique, cause externe confirmée, aucune correction de scanner requise. |
| L-06 | `Cinefolio-source.bundle` | `git bundle verify` réussi ; historique complet ; HEAD/main `90c149b797b45cba8a56dca2b74eed06c4225060`, tag v0.5.0 `e842667ce78ed1e27054f81209c006f955af1aaf`. SHA-256 archive : `fb0ca3080a66477d271f440f053ac59d811094caf8fb9e9d5eeaa92e176bf148`. |
| L-07 | `gh repo view ungeekrange-cmyk/chillcenter --json nameWithOwner,isEmpty,defaultBranchRef,url` | Dépôt cible accessible avec le CLI autorisé, vide, aucune branche initiale. Le connecteur GitHub lié renvoie 404 : ce n'est pas une preuve d'inexistence du dépôt. |

Chemin de base L-01 à L-06 : `/Users/ghillaumesoubigou/Documents/Dev/Perso/Video catalog/`. Les fichiers de travail ne sont pas un checkout Git et peuvent évoluer ; avant réutilisation, comparer au commit de référence sans écraser leurs différences. Une extraction temporaire de l'historique a servi à vérifier le manifeste du commit ; aucune modification de la source.

## Documentation primaire

| ID | Lien | Usage dans l'architecture |
| --- | --- | --- |
| S-01 | [Rust 1.99.0](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/) | Version stable publiée le 1/10/2026, déjà épinglée dans Cinéfolio. Rust n'a pas de canal LTS : retenir stable, pas nightly. |
| S-02 | [Raspberry Pi OS](https://www.raspberrypi.com/software/operating-systems/) | Lite 64 bits Debian 13, édition publiée le 6/10/2026 ; modèles 3 B/B+ compatibles. |
| S-03 | [Pi 3 B](https://www.raspberrypi.com/products/raspberry-pi-3-model-b/), [Pi 3 B+](https://datasheets.raspberrypi.com/rpi3/raspberry-pi-3-b-plus-product-brief.pdf) | 1 Go de RAM ; caractéristiques réseau. Ces fiches ne garantissent pas trois flux en Wi-Fi. |
| S-04 | [Axum](https://docs.rs/axum/latest/axum/) | Routage HTTP, intégration Tokio/Tower ; version visible 0.8.9 lors de la consultation. |
| S-05 | [SQLite WAL](https://www.sqlite.org/wal.html) | Base et WAL sur disque local au serveur, jamais sur le montage NAS ; cohérence de sauvegarde à assurer. |
| S-06 | [Architecture Tauri](https://v2.tauri.app/concept/architecture/) | Réutilisation du pont Rust et de la webview système ; pas de runtime Chromium embarqué proposé. |
| S-07 | [mpv, manuel](https://mpv.io/manual/stable/) | libmpv embarquable, commandes de lecture et accélération à vérifier sur Mac. |
| S-08 | [Callbacks de flux libmpv](https://raw.githubusercontent.com/mpv-player/mpv/master/include/mpv/stream_cb.h) | API signalée instable ; cancel depuis API 1.106, contraintes de thread/lifetime. Épingler et vérifier dans une version stable avant intégration ; source master non livrée. |
| S-09 | [Licences mpv](https://github.com/mpv-player/mpv/blob/master/Copyright) | Licences du moteur et des composants à contrôler pour le bundle réellement distribué. Aucun verdict juridique prétendu. |
| S-10 | [rustls ServerCertVerifier](https://docs.rs/rustls/latest/rustls/client/danger/trait.ServerCertVerifier.html), [rcgen](https://docs.rs/rcgen/latest/rcgen/) | Vérification TLS spécifique au serveur choisi et génération de certificats. |
| S-11 | [axum-server TLS](https://docs.rs/axum-server/latest/axum_server/tls_rustls/index.html) | Candidat de terminaison HTTPS Rust. |

## Versions et incertitudes

Rust 1.99.0 est vérifié. Les familles proposées sont Axum 0.8, Tokio 1, rusqlite 0.40, reqwest 0.13, Tauri 2 et rustls/rcgen compatibles. Les patchs et la version stable de libmpv/FFmpeg seront vérifiés puis verrouillés dans `Cargo.lock` et le manifeste de distribution avant implémentation. Les sources `master` ne sont pas des versions à livrer. Aucun upgrade, installation ou test de compatibilité n'a été exécuté à cette étape.
