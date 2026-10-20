# Chill Center v0.7 — notes de version

**Français** | [English](REL-V0.7.en.md)

Distribution de quatre composants 0.7.0. La publication GitHub ne met pas à jour
les installations existantes : **aucun Pi, NAS ou client web déployé n'est
modifié**. Les tags, manifestes signés et `PROVENANCE.json` des paquets fixent
la source exacte de la livraison. La v0.6 et ses paquets restent conservés.

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
sont corrigés. Source runtime/packaging revue : `3e174b9`, avec revues
indépendantes code, sécurité et tests ; les rejets des candidats précédents
restent conservés dans `docs/reviews`.

Suite finale runtime : 435 tests Rust réussis, deux harnesses NFR préexistants
ignorés ; 359 tests JavaScript/web/documentation réussis. Les 19 tests Python
de packaging passent. Le delta updater passe 9 tests ciblés, dont
désérialisation réelle du SDK et signature Minisign avec clé de test éphémère.
Deux tests root Linux réels, matrice systemd TERM/KILL,
occupation/reprise et retour arrière réussis sur VM Ubuntu ARM64 isolée.

Le vrai parcours AppKit/SDK a vérifié : rejet de signature invalide, garde
lecteur avant toute requête, lecture débutant pendant la confirmation,
installation signée, vrai redémarrage, nouveau consentement pour une version
suivante et annulation sans téléchargement ni modification. Clés de test
éphémères uniquement. Les échecs QA antérieurs sont conservés, pas requalifiés.
BUG0115 (configuration SDK), BUG0116 (AppleDouble dans le TAR QA) et BUG0117
(minimum macOS incorrect) ont été corrigés avant publication.

[Preuves détaillées et limites FR/EN](../implementation/COMPONENT-UPDATES-FIXES-20261019.md).

## Installation et limites

Installation Mac manuelle : DMG en lecture seule, application et raccourci
Applications pour glisser-déposer, instructions FR/EN. Le `.app.tar.gz` signé
reste réservé au mécanisme de mise à jour interne ; le DMG ne le remplace pas.

Le DMG Apple Silicon fourni nécessite **macOS 27+** à cause des bibliothèques
multimédias embarquées ; il contient une app signée ad hoc, non notariée.
Vérifier la signature Minisign de chaque paquet téléchargé. Ne pas utiliser
un script téléchargé isolément : l'installateur
serveur utilise aussi les unités `deploy/updates`, le helper et les manifestes
signés. Le paquet serveur est Linux ARM64, glibc 2.31+, avec systemd et
Python 3.10+. L'installateur initial **refuse une installation existante** :
la migration d'un ancien serveur doit être préparée séparément. Les installateurs
web/thèmes exigent le serveur/helper 0.7. Lire les
[instructions FR/EN](../updates/GUIDE-FR.md) avant installation.

Usage réservé au réseau local de confiance, sans authentification. Ne pas
exposer le serveur à Internet. Aucun transcodage serveur, aucune écriture NAS.
Le bundle Mac est signé ad hoc, non notarié ; aucun certificat privé n'est
distribué. Les essais VM/Chromium ne prouvent ni Safari physique ni les débits
Pi/NAS, ni une coupure secteur ou un disque plein.

Les anciennes releases, sauvegardes et versions installées sont conservées.
Un retour arrière ne doit jamais contourner l'anti-rollback de signature.
