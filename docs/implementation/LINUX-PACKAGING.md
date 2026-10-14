# Paquet serveur Linux

Ce livrable prépare une archive locale du serveur Chill Center. Il ne
constitue pas une installation Pi vérifiée : le script ne déploie rien, ne
crée aucun compte, ne monte aucun NAS et n'active ni ne démarre systemd.

## Construire

Les cibles acceptées sont explicites :

```text
scripts/package-linux.sh --target aarch64-unknown-linux-gnu \
  --output /chemin/absolu/chillcenter-pi64.tar.gz
scripts/package-linux.sh --target armv7-unknown-linux-gnueabihf \
  --output /chemin/absolu/chillcenter-pi32.tar.gz
scripts/package-linux.sh --target host-linux \
  --output /chemin/absolu/chillcenter-linux-test.tar.gz
```

`host-linux` est réservé à un hôte Linux et résout son triple Rust. Le build
utilise `--release --locked --offline` et refuse une cible Rust absente. Pour
une compilation croisée, l'opérateur doit fournir lui-même une toolchain et un
linker compatibles ; le script ne les télécharge pas et ne prétend pas avoir
créé un binaire Pi si l'édition de liens échoue. Il vérifie ensuite le type ELF
et l'architecture. `Cargo.lock`, la licence MIT et un manifeste avec commit,
versions d'outils, cible et hash du binaire sont inclus. L'archive n'embarque
ni libmpv, ni transcodeur : le serveur sert les octets originaux.

## Préparer l'hôte, sans publication Internet

Les commandes ci-dessous sont un runbook à exécuter seulement sur l'hôte
choisi. Remplacer l'adresse d'exemple par une adresse LAN explicite ; ne jamais
utiliser `0.0.0.0`, ouvrir un port sur le routeur ou publier le service.

1. Installer Raspberry Pi OS/Debian compatible avec la cible ELF choisie et
   synchroniser l'horloge avant le premier démarrage TLS.
2. Créer le compte système `chillcenter`, sans shell, avec le groupe nécessaire
   pour lire le montage. Créer `/var/lib/chillcenter` en `0700`, appartenant à
   ce compte. Le `data_dir` reste `/var/lib/chillcenter/data` : son parent est
   ainsi inscriptible par le même UID pour les arbres frères de restauration.
3. Monter le NAS sur `/srv/volumes/video` en lecture seule. Les identifiants SMB
   éventuels restent dans un fichier root `0600` externe à Chill Center ; ils
   ne vont ni dans `server.json`, ni dans l'archive, ni dans une sauvegarde.
4. Extraire l'archive dans un dossier temporaire, vérifier le fichier
   `.sha256`, puis installer le binaire, l'unité et la documentation aux chemins
   contenus dans l'archive. Copier `server.json.example` vers
   `/etc/chillcenter/server.json`, mode `0640`, propriétaire `root:chillcenter`,
   et adapter `listen`, `tls_names` et `video_root`.
5. Si le montage n'est pas `/srv/volumes/video`, créer un drop-in systemd qui
   réinitialise puis remplace `RequiresMountsFor`, `ConditionPathIsMountPoint`
   et `ReadOnlyPaths` avec le chemin exact de `video_root`. Le chemin du JSON et
   les protections systemd doivent toujours désigner la même racine.
6. Examiner `systemd-analyze verify chillcenter.service`, recharger systemd,
   puis activer/démarrer explicitement seulement après validation locale. Le
   paquet lui-même ne fait aucune de ces actions.

L'unité utilise un utilisateur non privilégié, `ProtectSystem=strict`, un seul
répertoire inscriptible local et une vue NAS en lecture seule. Elle attend le
réseau, la synchronisation de l'heure et le montage. Ces réglages ne remplacent
pas un pare-feu LAN ni la vérification que le routeur n'expose aucun port.

## Sauvegarde, mise à jour et rollback hors ligne

Toutes les commandes opérationnelles s'exécutent service arrêté et sous l'UID
`chillcenter`. Elles refusent un verrou actif ; elles ne tuent jamais le
service. Exemple après `systemctl stop chillcenter` :

```text
sudo -u chillcenter env HOME=/var/lib/chillcenter \
  /usr/lib/chillcenter/chill-server --backup \
  --config /etc/chillcenter/server.json
```

Créer cette sauvegarde avant de remplacer le binaire. Conserver l'ancien
binaire séparément. Un rollback binaire seul est autorisé uniquement si son
schéma SQLite reste compatible. Sinon, toujours service arrêté et avec le même
UID, restaurer la sauvegarde pré-migration avec `--restore ...
--confirm-replace`. En présence d'un journal, utiliser `--recover ...
--confirm-replace`. Le fichier statique `/etc/chillcenter/server.json` n'est
pas restauré automatiquement ; conserver sa copie administrative séparément.

Les sauvegardes contiennent la clé TLS privée et restent locales, en droits
restrictifs. Les anciens arbres `.data.pre-restore-*` et candidats de
récupération sont conservés : leur suppression ou archivage est une décision
manuelle après vérification. Les vidéos et le NAS ne sont jamais inclus.

## Preuves restant nécessaires

- Installation, démarrage et arrêt sur un vrai Pi 3 : `NOT_RUN`.
- Montage NAS réel, droits lecture seule et absence de secrets : `NOT_RUN`.
- TLS avec horloge réelle, accès LAN et absence d'exposition Internet :
  `NOT_RUN`.
- Débit, mémoire, Wi-Fi et deux/trois lectures simultanées : `NOT_RUN`.
- Builds ARMv7/aarch64 : réussis uniquement lorsqu'une toolchain correspondante
  est effectivement disponible ; un test local/factice ne les prouve pas.
