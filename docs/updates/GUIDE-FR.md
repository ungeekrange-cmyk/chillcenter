# Versions et mises à jour

**Français** | [English](GUIDE-EN.md)

La [release v0.7](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7)
regroupe les quatre composants 0.7.0 versionnés indépendamment. La publication
ne les installe pas sur un serveur existant. Les anciennes releases restent
disponibles ; voir les [preuves et limites](../releases/REL-V0.7.md).

## Quatre composants

| Composant | Version source | Tag de release | Installation |
| --- | --- | --- | --- |
| Mac Apple Silicon | `versions.json.mac` | `mac-vX.Y.Z` | DMG manuel ; confirmation native « Installer maintenant » pour les mises à jour suivantes |
| Serveur Linux | `versions.json.server` | `server-vX.Y.Z` | Installateur administrateur local, service non-root |
| Client web/PWA | `versions.json.web` | `web-vX.Y.Z` | Fichiers statiques sur le serveur, sans remplacer son binaire |
| Ambiances | `versions.json.themes` | `themes-vX.Y.Z` | Pack de données et images, sans reconstruire l’application |

L’API unique reste `/api/2026-10-14` ; `/api/v1` est supprimé depuis le serveur0.7.1.
Les versions produit ne sont
pas des versions d’API. Le pack comporte aussi ses versions de format et de
renderer. Voir [le calendrier des thèmes](../../themes/README.md).

## Configuration

La carte **Versions et mises à jour**, dans Général, affiche les versions
réellement installées du client, du serveur, des ambiances et de l’API. Le Mac
montre également la version web installée sur le serveur. Un ancien serveur
reste utilisable : seules les commandes de mise à jour non supportées sont désactivées.

- Manuel : aucune vérification planifiée ; bouton Vérifier maintenant.
- Vérification automatique : contrôle selon l’heure choisie, sans installation.
- Installation automatique : contrôle puis installation d’une publication signée
  compatible. Ce choix n’existe pas pour le Mac.

Par défaut, serveur/web/ambiances sont automatiques à **02:00 locale du serveur**.
Le fuseau est celui du système Linux, pas celui du navigateur. Une seule tentative
planifiée par date locale évite les doublons lors du changement d’heure. Le Mac
vérifie lorsque l’application est ouverte ; il demande toujours une confirmation
native avant d’installer et de redémarrer. Il n’existe pas de tâche Mac cachée
qui s’exécute application fermée.

Le catalogue reste utilisable pendant les vérifications/téléchargements. Les
lectures et travaux actifs diffèrent l’installation ; aucun bouton du LAN ne
permet de les interrompre de force. Après une mise à jour web, recharger la page
pour utiliser la nouvelle interface ; un onglet déjà ouvert conserve ses ressources
de version précédente. Le changement des ambiances est détecté en arrière-plan.

## Installation et confiance

Une installation 0.7 initiale est nécessaire pour activer le nouveau mécanisme :
le client 0.6 ne peut pas s’ajouter silencieusement une clé de confiance.
Les artefacts comprennent manifeste signé, signature et provenance.
Vérifier avant exécution ; ne pas utiliser `curl | sudo sh`.

Les scripts inspectables sont [install-server.sh](../../scripts/install-server.sh),
[install-web.sh](../../scripts/install-web.sh) et
[install-themes.sh](../../scripts/install-themes.sh). Le premier refuse de
remplacer une installation existante et exige explicitement la localisation de
la bibliothèque, l’écoute et le nom TLS. Il ne monte ni ne modifie le NAS.
Les deux autres requièrent le helper déjà configuré et ne remplacent que leur
composant. Les unités et règles de privilèges sont décrites dans
[ENGINE.md](ENGINE.md). Les limites des tests y sont explicites.

Les paquets Mac restent signés ad hoc, non notariés : une signature de mise à
jour vérifie la provenance des octets mais ne constitue pas une notarisation Apple.
La clé publique de release est [release.pub](../../updates/release.pub), identité
`18FD790B48C202F8`. La clé privée n’est jamais distribuée ni conservée dans Git.

### Télécharger et vérifier

Utiliser les archives complètes de distribution de v0.7, pas le ZIP des sources
généré automatiquement par GitHub. Télécharger le `.minisig` correspondant à
chaque archive. Vérifier avec Minisign installé indépendamment et la clé
publique fixée ci-dessous ; une clé contenue dans l'archive téléchargée ne
suffit pas, à elle seule, à établir la confiance.

```sh
minisign -Vm Chill-Center-server-0.7.0-linux-aarch64.tar.gz \
  -P 'RWT4AsJIC3n9GIPorIeouS7Ycc5ROkeCsMXypDtOGnikR2Jvm6cuxeQA'
tar -xzf Chill-Center-server-0.7.0-linux-aarch64.tar.gz
```

Même vérification pour le DMG Mac, le ZIP d'installation web et celui des thèmes
en adaptant le nom. La distribution Mac est réservée à Apple Silicon et
**nécessite macOS 27+**, à cause des bibliothèques multimédias embarquées.
Ouvrir le DMG, quitter l'ancienne application, puis glisser **Chill Center** sur
le raccourci **Applications**. Éjecter le disque et ouvrir l'app depuis Applications.
Garder l'ancien bundle pour un retour arrière local. Cette première installation
est manuelle, pas une mise à jour depuis v0.6. Le `.app.tar.gz` signé reste le
contenu utilisé par l'updater interne : le DMG ne change ni clé de confiance,
ni consentement, ni protocole de mise à jour.

L'archive serveur contient les deux binaires ARM64, le manifeste signé/la clé
publique, `scripts/` et `deploy/updates/`. Conserver cette arborescence. Une
installation Linux neuve requiert systemd, glibc 2.31+, Python 3.10+, Minisign,
curl et une bibliothèque lisible déjà configurée ; le helper ne monte aucun
stockage. Examiner le script, puis le lancer depuis la racine extraite avec
ses propres chemins absolus et réglages LAN :

```sh
sudo ./scripts/install-server.sh \
  --server "$PWD/chill-server" --updater "$PWD/chill-update" \
  --release-pub "$PWD/release.pub" --manifest "$PWD/server-manifest.json" \
  --manifest-signature "$PWD/server-manifest.json.minisig" \
  --version 0.7.0 --library /absolute/read-only-library \
  --listen LAN_ADDRESS:8443 --tls-name LAN_DNS_OR_IP
```

Remplacer les exemples ; aucune adresse du foyer n'est intégrée. **Ne pas
lancer ce bootstrap sur une installation existante.** La migration d'un ancien
serveur nécessite une sauvegarde administrateur et une installation préparée
séparément.

Après installation vérifiée du serveur/helper 0.7, vérifier puis extraire le
ZIP d'installation web ou thèmes et lancer avec sudo son script inspectable
`scripts/install-web.sh` ou `scripts/install-themes.sh`. Ces wrappers récupèrent
le flux signé correspondant et refusent les travaux actifs ; ils ne remplacent
ni le binaire serveur ni sa configuration. Le ZIP de contenu inclus peut être
conservé pour audit ; le wrapper n'accepte pas de chemins locaux ou d'URL de
release arbitraires.

## Publication par le mainteneur

Avant publication : revue indépendante, tests, source propre committée, paquets
et licences vérifiés. Incrémenter le composant dans `versions.json` et réserver
sa séquence strictement croissante dans [sequences.json](../../updates/sequences.json).
Le script [component-release.mjs](../../scripts/component-release.mjs) vérifie les
reçus signés publiés sur le dépôt fixe et refuse une séquence égale/inférieure,
une collision de version ou une erreur réseau. Une date système corrigée ne peut
donc pas faire reculer la séquence. Il signe localement et ne publie rien lui-même.
Ne jamais modifier une release stable en place ; publier une nouvelle version.

L’acceptation de séquence est conservée après un échec d’installation. Un paquet
défectueux nécessite une nouvelle publication corrective ou une récupération
administrateur locale vérifiée, pas une désactivation des signatures. Conserver
les anciens paquets et sauvegardes privées. Sauvegarder la clé de signature hors
ligne ; sa perte interdit de signer de nouvelles mises à jour avec cette identité.

Références : [contrat](CONTRACT.md), [ADR0029](../architecture/decisions/ADR-0029-component-releases-updates.md),
[API tierce](../api/INTEGRATION-FR.md).
