# Chill Center

Vidéothèque familiale issue de Cinéfolio : serveur Linux économe, client Mac Apple Silicon, profils et lecteur intégré.

État au14 octobre2026 : périmètre, architecture et33 user stories validés ; source logicielle126da0a approuvable après revues indépendantes code/tests/sécurité. Lecteur **natif libmpv/FFmpeg dans Tauri**, flux HTTPS/Range sans transcodage serveur, profils/reprise/transfert/enchaînement, catalogue/métadonnées/sagas/playlists, archives/import/CLI Cinéfolio, IA avec aperçu et ambiances sont raccordés. Scan exclusivement dans Administration.255 tests Rust,24 tests Node et7 tests de packaging passent ; parcours réels Tauri avec fixtures vidéo, deux lecteurs/transfert et archives. Pi/Linux/NAS déclarés prêts, Chill Center non installé : essais LinuxARM/Pi/systemd/NAS/Wi-Fi et fournisseurs réels restent **non exécutés** ; aucun «133 AC tous validés» n'est revendiqué. Voir [la remise et les limites](docs/implementation/COMPLETION-EVIDENCE.md).

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
