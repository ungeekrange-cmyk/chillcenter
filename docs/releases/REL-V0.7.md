# Chill Center v0.7 — notes de version candidates

**Français** | [English](REL-V0.7.en.md)

**Non publiée.** Les sources et paquets candidats ne remplacent pas la v0.6
livrée. La publication attend la validation native Mac ; aucune installation
sur le Pi n'est prévue dans cette demande.

## Composants 0.7.0

- Mac Apple Silicon, serveur Linux ARM64, client web responsive/PWA et thèmes
  possèdent chacun leur version et leur flux de release.
- Les versions réellement installées et la version API du serveur sont visibles
  dans Configuration, sans bloquer le catalogue pendant les vérifications.
- Vérification GitHub manuelle, vérification seule ou mise à jour automatique
  configurable. Serveur : 02 h locale par défaut, installation différée pendant
  l'utilisation. Mac : confirmation native « Installer maintenant » obligatoire.
- Paquets de thèmes déclaratifs indépendants de l'application : saisons,
  Halloween et Noël, variantes claires/sombres, calendrier/priorités documentés,
  validation et repli embarqué.
- Installateurs Linux inspectables, manifestes Minisign, compatibilité API/base,
  séquences monotones, slots et récupération après interruption. L'API datée
  `2026-10-14` et son alias existant sont conservés.

## Correctifs et vérification

Les huit constats de sécurité et les quatre défauts fonctionnels de l'updater
sont corrigés dans le candidat. Revues code et sécurité favorables sur
`3913267`, sans changement du client web déployé, du Pi, de sa configuration
ni du NAS. La revue tests ne conserve qu'un blocage : le parcours AppKit réel.

Suite de référence : 433 tests Rust réussis, deux harnesses NFR préexistants
ignorés ; 359 tests JavaScript/web/documentation réussis. Le delta natif passe
8 tests ciblés, dont désérialisation réelle du SDK et signature Minisign avec
clé de test éphémère. Deux tests root Linux réels, matrice systemd TERM/KILL,
occupation/reprise et retour arrière réussis sur VM Ubuntu ARM64 isolée.

Le premier lancement natif a révélé un champ obligatoire de clé publique absent
dans la configuration du SDK : BUG0115 corrigé avant publication. Le parcours
de consentement, installation et redémarrage reste à valider humainement.

[Preuves détaillées et limites FR/EN](../implementation/COMPONENT-UPDATES-FIXES-20261019.md).

## Installation et limites

Les paquets ne seront annoncés installables qu'après leur signature et leur
publication. Ne pas utiliser un script téléchargé isolément : l'installateur
serveur utilise aussi les unités `deploy/updates`, le helper et les manifestes
signés. Lire les [instructions FR/EN](../updates/GUIDE-FR.md) avant installation.

Usage réservé au réseau local de confiance, sans authentification. Ne pas
exposer le serveur à Internet. Aucun transcodage serveur, aucune écriture NAS.
Le bundle Mac est signé ad hoc, non notarié ; aucun certificat privé n'est
distribué. Les essais VM/Chromium ne prouvent ni Safari physique ni les débits
Pi/NAS, ni une coupure secteur ou un disque plein.

Les anciennes releases, sauvegardes et versions installées sont conservées.
Un retour arrière ne doit jamais contourner l'anti-rollback de signature.
