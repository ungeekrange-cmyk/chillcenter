# Chill Center

Vidéothèque familiale issue de Cinéfolio : serveur Linux économe, client Mac Apple Silicon, profils et lecteur intégré.

État au 13 octobre 2026 : périmètre, architecture et user stories validés ; **implémentation en cours**. L'interface Mac est raccordée au serveur HTTPS : profils, administration, catalogue, notes/vu et préférences. Le scan est uniquement dans Administration ; Actualiser recharge la liste sans scanner le NAS. 124 tests et trois parcours dans la vraie application passent, avec revues indépendantes du lot approuvées. Le lecteur intégré, les imports et la livraison Linux/Pi restent à faire ; aucune validation globale de l'étape 4 n'est demandée.

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

Paquet de développement Mac Silicon disponible localement dans `artifacts/macos/Chill Center.app` (signature ad-hoc, non notarized). Pour le reconstruire avec Rust et les outils Xcode : `bash scripts/package-macos.sh /chemin/absolu/nouveau.app`. La destination doit être neuve ; le script n'écrase rien. Ce paquet présente le catalogue et l'administration, pas encore le lecteur vidéo. Le client conserve ses réglages dans le répertoire de configuration utilisateur Tauri, séparément pour chaque serveur.
