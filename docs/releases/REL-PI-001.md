# REL-PI-001 — Première installation Linux / NAS

14 octobre 2026 · état **PREPARING**, installation pas encore exécutée.

## Métadonnées et autorisation

- Produit/source revue : `126da0af167ee62588d92d90297f2dfc51962676`, carrier local `16cf62c8602319e7cd645161310dab1ba5a9c7ad` ; version0.1.0. Code/test/sécurité dans COMPLETION-FINAL-126da0a ; source255 testsPASS, bundleMac déjà livré.
- G-04 : approuvé avec réserves. Demande explicite de l'utilisateur : « Passe à l’installation de Chill Center sur le Pi et aux tests avec le NAS. »
- Cible vérifiée : admin@192.168.1.42, Raspberry Pi3 B+Rev1.3, Debian13.7 ARM64, RAM905MiB, glibc2.41, réseau Wi-Fi wlan0. Horloge synchronisée, Europe/Dublin. Sudo non interactif disponible.
- Service/configuration/données Chill Center absents avant installation ; ne pas remplacer un éventuel état apparu depuis sans nouvel inventaire.
- Déploiement : seule une version Linux ARM64 identifiable, sans pile Tauri/libmpv/transcodeur. Compte système non privilégié, données locales, HTTPS bindLAN explicite et montage NAS laissé intact. Pas de découverte cloud/expositionInternet.

## Pré-vérification réelle

Connexion SSH directe réussie après le refus d'agent historique ; aucune authentification contournée. CIFS automount : `/srv/volumes/video`, source `//192.168.1.92/Video`, ro/nosuid/nodev/noexec. Permissions0750, uid999/gid984 (`marvin-admin`), fichiers0640. Lecture admin échoue, lecture sudo réussit. Choix utilisateur demandé pour permettre au compte Chill Center de lire sans root ; aucun groupe/remontage modifié jusque-là.149fichiers vidéo /513753167933octets à profondeur2 ; ce n'est pas le nombre final du scan récursif. Services Cockpit/SSH/rpcbind préexistants préservés ; le port Chill Center sera vérifié libre avant choix.

Choix reçu : « Oui, utiliser ce groupe existant » à la question explicitant le possible accès à d'autres fichiers de ce groupe. L'utilisateur autorise le nouveau compte `chillcenter` à rejoindre `marvin-admin` (gid984), pas à modifier le montage, changer les autres comptes, lire/copier les identifiants NAS ou tourner enroot. Le bind prévu est192.168.1.42:8443, libre lors de l'inventaire. Cette configuration d'installation exacte fera partie des gates du livrable Linux.

## Artefact et gates

Compilation prévue sur Mac dans un environnement temporaire, Rust1.99 déjà verrouillé, cargo-zigbuild0.23.4/Zig0.17.0 stable vérifiés par sources primaires. Cible aarch64-unknown-linux-gnu avec plancherglibc2.28 (hôte2.41), CPU générique compatiblePi3 ; aucune version de crate modifiée. Créer manifest/hash, vérifier ELF64/AArch64/dépendances et exécution sur le vraiPi avant service. Artefact final, revue d'intégration indépendante et propriétaire de déploiement à rattacher avant installation ; pas d'auto-approval.

Sources primaires d'outillage : [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild), [release0.23.4](https://github.com/rust-cross/cargo-zigbuild/releases/tag/v0.23.4), [Zig0.17.0 stable](https://ziglang.org/download/), [Debian13 et cycle de support](https://www.debian.org/releases/trixie/). Outils installés dans `/private/tmp/chillcenter-linux-cross.1M5r5u/venv`, pas dansPython/Rust personnels ou sur lePi. Seul l'ancien cache de compilation de revue `/private/tmp/chillcenter-test-review-ded8-target` (inventaire CACHEDIR.TAG/tmp/debug, aucun.git/source) a été supprimé pour récupérer4,8Go ; entièrement recréable, rapports/sources/bundles conservés.

## Installation et retour

Compte/champs et unité issus du runbook LINUX-PACKAGING ; configuration locale `/etc/chillcenter/server.json`, données `/var/lib/chillcenter/data`, binaire `/usr/lib/chillcenter/chill-server`. IP et montage restent configurables. Installer un nouvel état seulement ; aucune migration Cinéfolio/duNAS. Secrets SMB non lus/copiés, cléTLS générée côté serveur en0600, jamais sortie dans les preuves.

Retour d'une première installation : arrêter/désactiver uniquement chillcenter.service si le smoke critique échoue, conserver binaire/configuration/données/compte pour diagnostic et correction ; ne rien supprimer du NAS ni toucher aux autres services. Pour toute mise à jour ultérieure, sauvegarde cohérente pré-migration avec service arrêté et utilisateur de service ; restaurer la sauvegarde si ancien schéma incompatible. Aucun rollback destructif improvisé.

## Vérification après installation — PENDING

Version/hash/config/unit/user/montageRO/TLS/health/bootstrap/profils, scan réel conservateur puis vidéoHTTPSRange/pistes/reprise/seeks/profilstransferts et deux/trois lectures pendant30min. Mesurer RSS/CPU/débit/température/mémoiredisponible/buffering ; seuils de l'architecture, sans inventer succès. Pas de déconnexion physique du NAS, coupure électrique ou reboot affectant Marvin sans accord spécifique ; simuler la perte côté client ou arrêter/reprendre uniquement le nouveau service lorsque sûr. Tests de restauration avec jeu isolé, jamais remplacer les données du foyer pendant le benchmark.

Un vérificateur indépendant distinct du déployeur sera chargé des vérifications finales, conformément aux compétences ship-deployer/release-verifier. Aucun critère matériel DONE avant preuves et gatepost-déploiement ; les tests non exécutés restent explicitement NOT_RUN.
