# Chill Center

**Français** | [English](README.en.md)

Votre vidéothèque, sur votre réseau : un serveur Linux léger et un client macOS Apple Silicon, principalement écrits en Rust, ainsi qu’un client web responsive installable sur l’écran d’accueil (PWA).

**[Télécharger l’application Mac Apple Silicon v0.7 — DMG](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7/Chill-Center-0.7.0-macos-arm64.dmg)** · [Notes FR/EN et paquets](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7)

Mac, serveur, web et thèmes sont versionnés séparément dans
[versions.json](versions.json). Les versions installées et les réglages de mise
à jour sont visibles dans **Configuration → Général**. Le Mac demande toujours
« Installer maintenant » ; serveur/web/thèmes peuvent se mettre à jour à 02 h
locale, hors lecture et travaux actifs. Les [packs de thèmes](themes/README.md)
décrivent leur calendrier et priorité.

| Composant 0.7.0 | Paquet et instructions |
| --- | --- |
| Mac Apple Silicon, build 9 | [Installateur DMG](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7/Chill-Center-0.7.0-macos-arm64.dmg), macOS 27+ pour ce bundle |
| Serveur Linux ARM64 | [Archive avec installateurs](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7/Chill-Center-server-0.7.0-linux-aarch64.tar.gz), hôte 64 bits avec systemd |
| Client web/PWA | [Paquet avec installateur](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7/Chill-Center-web-0.7.0-install.zip), serveur 0.7 requis |
| Thèmes indépendants | [Pack avec installateur](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7/Chill-Center-themes-0.7.0-install.zip) |

Lire les [instructions de vérification et d’installation](docs/updates/GUIDE-FR.md)
et les [notes de version et limites](docs/releases/REL-V0.7.md). Les manifestes
signés sont dans les releases propres à chaque composant. Le serveur existant
n’est pas mis à jour par cette publication ; la v0.6 et les anciens paquets
restent disponibles. Ne pas lancer l’installateur initial sur une installation
existante : il refuse volontairement de l’écraser.

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

TMDB et les fonctions IA optionnelles du client Mac utilisent des services externes. Configurez vos propres clés ; ne les ajoutez jamais au dépôt ni aux tickets. Les fonctions IA ne sont pas proposées dans le client web.

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
python3 scripts/package-macos-dmg.py /chemin/absolu/Chill-Center-test.app /chemin/absolu/Chill-Center-test.dmg
```

Les bundles locaux sont signés ad hoc, non notariés. Les licences des bibliothèques embarquées doivent les accompagner.

## Client web et PWA

```sh
node scripts/build-web.mjs
node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js
```

Les fichiers générés dans `web/dist` sont servis par Chill Center, sur la même
origine que l’API. Ils ne constituent pas un serveur autonome. Sur iPhone,
ouvrir le site dans Safari, puis **Partager → Sur l’écran d’accueil** ; activer
**Ouvrir comme app web** si proposé. Sur Android, utiliser l’installation
proposée par le navigateur lorsque les conditions PWA sont réunies.

L’application reste dépendante du serveur du foyer et des capacités du navigateur.
Les safe areas protègent les commandes des encoches et poinçons ; Safari peut
encore refuser le plein écran natif d’une vidéo Xvid rendue sur canvas. Les
essais sur appareil restent distincts des tests Chromium. Aucune app native
iOS/Android ni authentification Face ID n’est incluse.

## API et documentation

L’unique API HTTPS est `/api/2026-10-14`. Les ajouts compatibles restent dans cette version ; depuis le serveur0.7.1, l’ancien alias `/api/v1` est supprimé (404). Les clients tiers doivent utiliser la base datée.

- [Contrat OpenAPI](docs/api/openapi-2026-10-14.json)
- [Documentation bilingue](docs/README.md) et [guide d'intégration API](docs/api/INTEGRATION-FR.md)
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
