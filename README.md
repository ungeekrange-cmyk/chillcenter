# Chill Center

Votre vidéothèque, sur votre réseau. Projet issu de Cinéfolio : un serveur Linux léger et un client macOS Apple Silicon, principalement écrits en Rust.

Version de référence du client Mac : **v0.6 — 0.6.0, build 7**.
Livraison locale : `artifacts/macos/Chill Center-v0.6.app`.
Consulter les [notes de version et preuves](docs/releases/REL-V0.6.md) pour les
corrections, les contrôles exécutés et les limites connues. Le projet reste en
développement ; cette version ne publie ni ne déploie le serveur ou le client web.

## Fonctionnalités

- Films, séries, saisons, sagas et listes de lecture.
- Profils : reprise synchronisée, historique, vidéos vues et notes personnelles.
- Lecteur natif libmpv/FFmpeg : audio, sous-titres, vitesse et épisode suivant.
- Métadonnées TMDB avec votre clé, correction manuelle et import d’images.
- Filtres par genres et années, thèmes clair/sombre et ambiances saisonnières.
- Catalogue et images conservés côté client ; synchronisation différentielle en arrière-plan.
- Interface en anglais ou français, détection automatique et choix dans Configuration.

Le serveur fournit les médias en HTTPS depuis le stockage configuré. **Aucun transcodage serveur** : le client décode les vidéos sans monter le NAS. La cible inclut le Raspberry Pi 3 ; la fluidité dépend du réseau et des fichiers. Deux ou trois utilisateurs constituent un objectif, pas une garantie à tout débit.

## Sécurité et confidentialité

Usage prévu : **réseau local de confiance du foyer**. Les profils ne sont pas des comptes authentifiés. Ne pas exposer le serveur sur Internet. Le client Mac demande d’accepter le certificat auto-généré lors de la première connexion.

TMDB et les fonctions IA optionnelles utilisent des services externes. Configurez vos propres clés ; ne les ajoutez jamais au dépôt ni aux tickets. Le client web évolue dans un chantier séparé : cette préparation ne le modifie ni ne le déploie.

## Développer

Prérequis : Rust indiqué par `rust-toolchain.toml`, outils de compilation de la plateforme et libmpv pour le lecteur. Le packaging Mac utilise les outils Xcode et une installation locale de mpv comme entrée.

```sh
cargo test --workspace --all-targets --locked
cargo run --locked -p chill-server -- --config /chemin/vers/config.json
cargo run --locked -p chillcenter-desktop
```

Adapter la configuration à votre environnement et la conserver hors Git (par exemple `server.local.json`, ignoré). Le programme n’impose ni adresse de serveur ni chemin NAS. Par défaut, le serveur écoute uniquement sur loopback, port `8443` ; l’écoute LAN se configure explicitement. La cible serveur du produit reste Linux.

Construire une application Mac dans une destination **neuve**, sans écraser un bundle livré :

```sh
bash scripts/package-macos.sh /chemin/absolu/Chill-Center-test.app
```

Les bundles locaux sont signés ad hoc, non notariés. Les licences des bibliothèques embarquées doivent les accompagner.

## API et documentation

L’API HTTPS est versionnée : `/api/2026-10-14`. Les ajouts compatibles restent dans cette version ; une rupture nécessite une nouvelle version et le maintien de celle utilisée par les clients existants.

- [Contrat OpenAPI](docs/api/openapi-2026-10-14.json)
- [Contribuer](CONTRIBUTING.md) et [sécurité](SECURITY.md)
- [Préparer le dépôt GitHub public](docs/publication/GITHUB-REPOSITORY.md)
- [Préparer et vérifier GitHub Pages](docs/publication/GITHUB-PAGES.md)

Architecture, décisions et journaux de travail restent dans `docs/`. Ils ne sont **pas** inclus automatiquement dans le site public. Rendre le dépôt et son historique publics exige un audit distinct.

## GitHub Pages (optionnel)

Le livrable principal est le dépôt GitHub et son README. Un site de présentation statique peut être hébergé ultérieurement sur GitHub Pages, sans dépendance ni suivi applicatif et sans accès au serveur du foyer :

```sh
node --test scripts/test-public-site.mjs
node scripts/build-public-site.mjs
```

Ouvrir `dist/public/index.html` pour la prévisualisation. Le workflow Pages est manuel : aucun déploiement lors d’un commit ou d’une pull request. Sa présence ne vaut pas autorisation de publication.

## Licence

Code sous [licence MIT](LICENSE). Les bibliothèques tierces, les images de médias et les services externes conservent leurs propres licences et conditions.
