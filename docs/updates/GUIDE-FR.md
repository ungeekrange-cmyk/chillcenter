# Versions et mises à jour

**Français** | [English](GUIDE-EN.md)

Statut : candidat 0.7.0 en développement et revue, **pas une livraison**.
Le seul bundle Mac publié à ce stade est [v0.6](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.6).
Les installateurs et le helper ne doivent pas être utilisés en production avant
la validation de sécurité et les essais de déploiement documentés.

## Quatre composants

| Composant | Version source | Tag de release | Installation |
| --- | --- | --- | --- |
| Mac Apple Silicon | `versions.json.mac` | `mac-vX.Y.Z` | Archive `.app`, puis confirmation native « Installer maintenant » |
| Serveur Linux | `versions.json.server` | `server-vX.Y.Z` | Installateur administrateur local, service non-root |
| Client web/PWA | `versions.json.web` | `web-vX.Y.Z` | Fichiers statiques sur le serveur, sans remplacer son binaire |
| Ambiances | `versions.json.themes` | `themes-vX.Y.Z` | Pack de données et images, sans reconstruire l’application |

L’API reste `/api/2026-10-14` et conserve `/api/v1`. Les versions produit ne sont
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
Les artefacts seront publiés avec manifeste signé, signature et provenance.
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
