# Chill Center

Vidéothèque familiale issue de Cinéfolio : serveur Linux économe, client Mac Apple Silicon, profils et lecteur intégré.

État au 14 octobre 2026 : remise logicielle126da0a acceptée avec réserves. Client Tauri/libmpv/FFmpeg et serveur Linux sans transcodage, catalogue/profils/reprise/transfert/enchaînement/archives/IA/ambiances raccordés ; scan uniquement Administration. Le serveur ARM64 tourne désormais sur le Pi3B+/Debian13.7 à **192.168.1.42:8443**, avec NAS en lecture seule et HTTPS vérifiés indépendamment. Scan réel2 496vidéos et premières lectures AVI/MKV/MP4 PASS ; charges longues en cours, pas133ACtousPASS. [Installation et preuves](docs/releases/REL-PI-001.md), [tests médias](docs/releases/REL-PI-001-media-evidence.md), [remise logicielle](docs/implementation/COMPLETION-EVIDENCE.md) (255Rust/24Node/7packagingPASS et trois revuesfavorables).

## Lire le projet

- [Contexte et étapes autorisées](docs/PROJECT_CONTEXT.md)
- [Périmètre et décisions fonctionnelles](docs/discovery/SCOPE.md)
- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Contrats, modèle de données et états de lecture](docs/architecture/CONTRACTS.md)
- [Décisions techniques proposées](docs/architecture/decisions/INDEX.md)
- [Traçabilité et validations](docs/TRACEABILITY.md)
- [Journal des bugs et observations](docs/debug/INDEX.md)
- [Vérification et exploitation prévues](docs/VERIFICATION.md)
- [Sources et baseline Cinéfolio](docs/SOURCES.md)
- [Backlog et ordre d'exécution](docs/BACKLOG.md)
- [Couverture des exigences par les stories](docs/stories/TRACEABILITY.md)
- [Validations du projet](docs/gates/INDEX.md)

Voir [l'état exact d'implémentation](docs/implementation/STATUS.md), [les preuves de raccordement](docs/implementation/INTERFACE-EVIDENCE.md), [les preuves historiques](docs/implementation/FOUNDATION-EVIDENCE.md) et [la configuration modifiable](docs/deployment/CONFIGURATION.md). Les manifestes historiques des remises documentaires restent dans `docs/manifests/` ; ils ne décrivent pas les fichiers modifiés après approbation.

## Service de développement

```sh
cargo test --workspace --all-targets --locked
cargo run --locked -p chill-server -- --config /chemin/config.json
cargo run --locked -p chillcenter-desktop
```

Adapter `deploy/server.example.json` avant utilisation. Aucun montage NAS n'est nécessaire pour démarrer l'administration/profils. L'exemple Pi n'est jamais une adresse imposée par le programme. Le serveur par défaut écoute seulement loopback ; l'écoute LAN doit être configurée explicitement. Les essais serveur macOS sont des tests locaux, pas une nouvelle cible produit : serveur Linux uniquement.

Le paquet Mac Silicon final embarque libmpv et ses dépendances (signature ad hoc, non notarized ; licences tierces incluses). Pour reconstruire avec Rust, les outils Xcode et mpv installé comme entrée de packaging : `bash scripts/package-macos.sh /chemin/absolu/nouveau.app`. Destination neuve obligatoire ; aucun ancien paquet écrasé. Les chemins/hashes du paquet courant sont dans [la remise](docs/implementation/COMPLETION-EVIDENCE.md). Le client conserve ses réglages dans le répertoire utilisateur Tauri, séparément pour chaque serveur. Le CLI `--verify-bundle` vérifie le libmpv embarqué sans ouvrir l'interface ou les réglages réseau.

Pour Linux/Pi : [construction et installation explicites](docs/implementation/LINUX-PACKAGING.md), configuration `deploy/linux/server.json.example`, unité systemd et procédures backup/restore/recover. L'adresse192.168.1.42 et `/srv/volumes/video` ne sont que des exemples modifiables. Aucun serveur installé automatiquement.
