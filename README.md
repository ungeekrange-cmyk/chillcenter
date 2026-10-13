# Chill Center

Vidéothèque familiale issue de Cinéfolio : serveur Linux économe, client Mac Apple Silicon, profils et lecteur intégré.

État au 13 octobre 2026 : périmètre, architecture et user stories validés ; **implémentation en cours**. Les fondations Rust passent 108 tests et le service HTTPS de configuration a été testé localement. L'application Mac complète, la lecture serveur-client, les imports et la livraison Linux/Pi ne sont pas terminés ; aucune validation de l'étape 4 n'est demandée à ce stade.

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

Voir [l'état exact d'implémentation](docs/implementation/STATUS.md), [les preuves locales](docs/implementation/FOUNDATION-EVIDENCE.md) et [la configuration modifiable](docs/deployment/CONFIGURATION.md). Les manifestes historiques des remises documentaires restent dans `docs/manifests/` ; ils ne décrivent pas les fichiers modifiés après approbation.

## Service de développement

```sh
cargo test --locked -p chill-catalog -p chill-domain -p chill-server -p chill-client -p chill-player
cargo run --locked -p chill-server -- --config /chemin/config.json
```

Adapter `deploy/server.example.json` avant utilisation. Aucun montage NAS n'est nécessaire pour démarrer l'administration/profils. L'exemple Pi n'est jamais une adresse imposée par le programme. Le serveur par défaut écoute seulement loopback ; l'écoute LAN doit être configurée explicitement. Le paquet Mac Tauri est encore un squelette, et la suite ci-dessus ne prétend pas le tester.
