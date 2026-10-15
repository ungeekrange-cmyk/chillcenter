# Runbook d'exécution — déploiement du client web sur le Pi (prêt à coller, NON exécuté)

Révision `CC-WEB-RUNBOOK-001`, 15 octobre 2026. Complète [WEB-CLIENT-PI.md](WEB-CLIENT-PI.md). **Rien de ce document n'a été exécuté sur le Pi.** Il n'autorise rien : il faut ton feu vert explicite (étape 0) et une connexion SSH maître que **toi** tu ouvres.

Lire d'abord les conventions : le script est envoyé par **stdin** (`bash -s`), jamais par une chaîne citée imbriquée (leçon de [BUG-0051](../debug/BUG-0051-pi-update-output-quoting.md)) ; TLS toujours strict (jamais `-k`) ; si une sortie est vide ou ambiguë, **ne pas relancer à l'aveugle** : exécuter le préflight (étape 3) pour lire l'état réel.

## Qui fait quoi

| Étape | Qui | Écrit sur le Pi ? |
| --- | --- | --- |
| 0 Feu vert | toi | non |
| 1 Ouvrir la connexion maître | **toi**, dans un Terminal (mot de passe/agent éventuels) | non |
| 2 Contrôle local des hashes | toi ou moi | non |
| 3 Préflight lecture seule | toi ou moi (via ta connexion) | **non** (root, mais lecture seule) |
| 4 Copie de deux fichiers | toi ou moi | **oui : 2 fichiers dans `/var/tmp`** |
| 5 Exécution du script V8 | toi ou moi | **oui : le déploiement** (arrêt bref du service, sauvegarde privée, binaire, dossier web) |
| 6 Vérification et nettoyage | toi ou moi | supprime les 2 fichiers de `/var/tmp` |
| 7 Retour arrière | seulement si nécessaire | oui |
| 8 Redirection du port 443 (ADR-0017) | toi ou moi, **feu vert séparé**, après les étapes 4 à 6 | **oui : une table nftables et une unité systemd** |

Tu peux coller toi-même chaque bloc. Ou bien tu m'autorises à les exécuter à travers **ta** connexion maître : dans ce cas écris explicitement « **Feu vert déploiement V8** » **et** le chemin du socket (étape 1), et je n'exécute que les étapes 2 à 6, dans l'ordre, en m'arrêtant à la première sortie inattendue.

## Étape 0 — conditions du feu vert (à vérifier avant de le donner)

- [ ] Personne ne regarde de vidéo pendant ≈ 1 minute (le script **refuse** d'arrêter s'il existe une connexion cliente établie, code 42 ; le service est arrêté quelques dizaines de secondes sinon). Les profils de la maison peuvent avoir besoin de relancer leur lecture après.
- [ ] Tu as lu `scripts/pi-preflight-readonly.sh` (28 lignes, lecture seule : aucune écriture, aucune clé privée lue). Ce petit script n'a **pas** eu de revue indépendante (contrairement au script V7) ; c'est pourquoi il ne contient que des commandes de lecture.
- [ ] Tu acceptes les risques résiduels documentés (revues 1 à 3) : exécution jamais faite sur Debian 13 avec systemd réel, durée d'interruption non mesurée, et les LOW-1/2/3 de la [revue n° 3](../reviews/web-pi-deploy-v7-review-3.md).

## Étape 1 — ouvrir la connexion SSH maître stricte (toi, dans un Terminal)

Même convention que les livraisons précédentes ([REL-PI-001](../releases/REL-PI-001.md)) : dossier privé `0700`, socket `0600`, vérification stricte de la clé d'hôte, compte `admin`, durée 2 h.

```bash
CPDIR="$(mktemp -d /private/tmp/chillcenter-ssh.XXXXXX)" && chmod 700 "$CPDIR"
ssh -MNf -o StrictHostKeyChecking=yes -o ControlPersist=2h -S "$CPDIR/control" admin@192.168.1.42
echo "SOCKET=$CPDIR/control"
ssh -O check -S "$CPDIR/control" admin@192.168.1.42
```

Attendu : une ligne `SOCKET=/private/tmp/chillcenter-ssh.XXXXXX/control` et `Master running (pid=…)`. **Si l'authentification échoue** (agent qui ne signe plus, clé d'hôte inconnue ou changée), **arrête-toi** : ne contourne ni la vérification de clé d'hôte ni l'agent ; dis-moi le message. Si tu m'autorises à exécuter, donne-moi la ligne `SOCKET=…` (je vérifie qu'elle est dans un dossier `0700`, que le socket est `0600` et à toi).

Pour le reste des blocs, une fois dans le même Terminal :

```bash
CP="$CPDIR/control"                     # ou colle ici le chemin SOCKET=… ci-dessus
R=/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter/.claude/worktrees/web-pwa
pi()     { ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o ControlMaster=no -o ControlPath="$CP" admin@192.168.1.42 "$@"; }
pi_scp() { scp -o BatchMode=yes -o StrictHostKeyChecking=yes -o ControlMaster=no -o ControlPath="$CP" "$@"; }
```

## Étape 2 — contrôle local des hashes (aucun accès au Pi)

Un seul bloc ; il doit afficher quatre `OK`. **Un seul `MISMATCH` = on s'arrête** (un fichier a changé depuis la revue).

```bash
cd "$R"
chk() { [ "$(shasum -a 256 "$1" | cut -d' ' -f1)" = "$2" ] && echo "OK       $1" || echo "MISMATCH $1"; }
chk scripts/pi-update-web-operator.sh                                   dcf1866111ef756b53113adce323f90960f5edbf00835204da78097fd6528e29
chk target/aarch64-unknown-linux-gnu/release/chill-server               a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527
chk artifacts/web-pi/chillcenter-web.tar                                894fbe952a58bdae01527e8d8fa646e87977b9f1aea5bd996d770bacb71f7118
chk scripts/pi-preflight-readonly.sh                                    2ee4a5a9d93a882d738d0f11e256f6aad702c25555a0c788d6f5b73196d50d44
wc -c < scripts/pi-update-web-operator.sh                               # 17916
bash scripts/pi-update-web-operator.sh --self-test                      # parser_self_test=PASS
```

## Étape 3 — préflight **lecture seule** sur le Pi

```bash
pi 'sudo -n env true && echo sudo_env_ok'
pi 'sudo -n /bin/bash -s --' < "$R/scripts/pi-preflight-readonly.sh"
```

Valeurs attendues (sinon **stop**, ne passe pas à l'étape 4) :

| Ligne | Attendu |
| --- | --- |
| `sudo_env_ok` | présent (sudoers autorise `env`) |
| `run_as_uid` | `0` |
| `installed_binary_sha256` | `cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3` |
| `service_active` | `active` ; `NRestarts=0` |
| `chillcenter_uid` | `997` |
| `established_chill_server_sockets` | `0` |
| `listening` | une adresse se terminant par `:8443` |
| `cert_spki_sha256` | `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df` |
| `rollback_dir` | `700:0:0` et contient `chill-server-r7-9a17ffe55e3d` |
| `web_dir` | `absent` (ou `PRESENT` : alors l'ancien dossier sera conservé) |
| `incoming_files_present` | vide |
| `MISSING_TOOL=…` | aucune ligne |
| espace libre | ≥ 65 536 Kio sur chaque ligne |

Les lignes `config_sha256` et `unit_sha256` sont à noter : le script les revérifie à la fin (elles ne doivent pas changer). Un `established_chill_server_sockets` ≠ 0 signifie quelqu'un qui regarde : attendre.

## Étape 4 — copie des deux fichiers (écriture : 2 fichiers dans `/var/tmp`)

```bash
cd "$R"
pi_scp target/aarch64-unknown-linux-gnu/release/chill-server admin@192.168.1.42:/var/tmp/chill-server-web.new
pi_scp artifacts/web-pi/chillcenter-web.tar                   admin@192.168.1.42:/var/tmp/chillcenter-web.tar
pi 'chmod 600 /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar; sha256sum /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar'
```

Les deux hashes affichés par le Pi doivent être **exactement** `a7ab7b07633c916c…d527` (binaire) et `894fbe952a58bdae…7118` (archive). Sinon : `pi 'rm -f /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar'` et stop.

## Étape 5 — exécution du script V8, **par stdin** (le déploiement)

Relire à voix haute avant de coller : l'URL est `https://192.168.1.42:8443` (le script ne vérifie que le **port**, LOW-1), les deux hashes sont ceux de l'étape 4.

```bash
cd "$R"
[ "$(shasum -a 256 scripts/pi-update-web-operator.sh | cut -d' ' -f1)" = dcf1866111ef756b53113adce323f90960f5edbf00835204da78097fd6528e29 ] && echo "script V8 conforme" || echo "SCRIPT MODIFIÉ : STOP"
pi "sudo -n env OLD_BINARY_SHA256=cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3 NEW_BINARY_SHA256=a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527 WEB_ARCHIVE_SHA256=894fbe952a58bdae01527e8d8fa646e87977b9f1aea5bd996d770bacb71f7118 CHILLCENTER_TLS_CERT_PATH=/var/lib/chillcenter/data/tls/certificate.pem CHILLCENTER_BASE_URL=https://192.168.1.42:8443 /bin/bash -s --" < scripts/pi-update-web-operator.sh
echo "code de sortie : $?"
```

**Succès** (code 0) : 7 lignes `deployed_utc`, `service_pid`, `installed_hash=a7ab…d527`, `rollback_hash=cef5…b3c3`, `web_archive_hash=894f…7118`, `web_previous=none`, `backup_path=/var/lib/chillcenter/data/backups/<uuid>`.

| Code de sortie / signe | Signification | À faire |
| --- | --- | --- |
| **0** | Déployé et vérifié | étape 6 |
| **2** | Refus **avant tout arrêt** (hash, archive, URL, port, sonde TLS, outil manquant, disque, rollback) | lire le message ; **rien n'a changé** ; corriger la cause |
| **42** | Une connexion cliente est établie | **rien n'a changé** ; attendre et recommencer |
| **1 ou 3** avec des lignes `RECOVERY:` | Échec après l'arrêt ; **restauration automatique** | lire les deux dernières lignes : `service is active` + `answers over strict TLS` = **ancien état rétabli** ; `ERR line N: …` dit ce qui a échoué ; **ne pas relancer** avant d'avoir compris |
| `RECOVERY: SERVICE IS NOT ACTIVE` ou `could NOT be restored` | La restauration n'a pas pu tout rétablir | **étape 7 (manuelle) immédiatement** |
| **129 / 130 / 143** | Interrompu (SSH coupé, Ctrl-C) | la récupération s'est exécutée ; refaire le **préflight** (étape 3) pour lire l'état réel |
| **1 sans aucune ligne `RECOVERY`**, sortie fermée | Probablement LOW-2 : déployé, résumé non imprimé | préflight : si `installed_binary_sha256=a7ab…` et service actif, c'est déployé ; vérifier l'étape 6 |
| Sortie vide, SSH tombé | État inconnu | préflight (étape 3) ; **ne pas relancer le script** tant que l'état n'est pas lu |

## Étape 6 — vérification et nettoyage

Lecture seule, depuis le Mac (TLS strict avec le certificat public du Pi ; l'empreinte doit rester `3018fb4d…40df`). Le script a déjà fait ses propres vérifications depuis le Pi ; celles-ci les recoupent depuis ton poste :

```bash
cd "$R"
pi 'sudo -n sha256sum /usr/lib/chillcenter/chill-server; systemctl is-active chillcenter; ls -la /usr/lib/chillcenter/web | head -5'
# Certificat public lu par TLS (aucune clé privée), empreinte à comparer :
openssl s_client -connect 192.168.1.42:8443 </dev/null 2>/dev/null | openssl x509 -pubkey -noout | openssl pkey -pubin -outform DER | openssl dgst -sha256
# Nettoyage des deux fichiers entrants (ils t'appartiennent, pas besoin de sudo) :
pi 'rm -f /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar; ls /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar 2>&1 | head -2'
```

Attendu : hash binaire `a7ab7b07…`, `active`, un dossier `web` root, empreinte `3018fb4d…`, deux fichiers absents. Je peux ensuite refaire la sonde Node en lecture seule (catalogue, Range MKV/MP4/AVI, décodeur) contre le serveur réel.

**Voir l'interface** : ouvre `https://192.168.1.42:8443/` dans le navigateur. Le certificat est auto-signé : une page d'avertissement apparaît. Deux options, à ton choix :
- **Exception ponctuelle** pour cette adresse : la page se charge ; **Service Worker et installation PWA seront probablement refusés**.
- **Faire confiance au certificat dans le trousseau macOS** (Trousseaux d'accès → certificat → « Toujours approuver » pour SSL) : la PWA est installable. Cela modifie la confiance **système** pour ce certificat : à ne faire que si tu as vérifié son empreinte (`3018fb4d…`) ; c'est un geste à faire par toi.

Fermer la connexion maître quand tout est fini : `ssh -O exit -S "$CP" admin@192.168.1.42`.

## Étape 7 — retour arrière **manuel** (seulement en cas de besoin)

Base de données, configuration, TLS et NAS ne sont **pas** touchés par le déploiement : le retour ne concerne que le binaire et le dossier web. L'ancien binaire est dans `/root/chillcenter-rollback/chill-server-cef54cd0e8ba` (hash `cef54cd0…b3c3`) ; le plus ancien R7 reste dans `chill-server-r7-9a17ffe55e3d` (`9a17ffe5…36a6`).

```bash
pi 'sudo -n systemctl stop chillcenter'
pi 'sudo -n install -o root -g root -m 0755 /root/chillcenter-rollback/chill-server-cef54cd0e8ba /usr/lib/chillcenter/chill-server'
pi 'sudo -n sha256sum /usr/lib/chillcenter/chill-server'          # doit afficher cef54cd0e8baf45f…24b3c3
pi 'sudo -n rm -rf --one-file-system /usr/lib/chillcenter/web'     # (ou, s'il existait avant : mv -T /root/chillcenter-rollback/web-<horodatage> /usr/lib/chillcenter/web)
pi 'sudo -n systemctl start chillcenter'
pi 'systemctl is-active chillcenter'
```

Puis refaire le préflight (étape 3) : binaire `cef54cd0…`, service actif, `web_dir=absent`. La sauvegarde privée créée par le déploiement (`backup_path` imprimé) reste sur le Pi, `0700`, jamais copiée ni affichée ; la restaurer (`--restore … --confirm-replace`) n'est **pas** nécessaire pour ce retour, car aucune donnée n'est modifiée.

## Ce que ce runbook ne garantit pas

Comportement sur Debian 13 réel avec systemd réel, durée réelle d'interruption, mémoire du Pi 3B+ sous charge, CSP et Service Worker dans un vrai navigateur sur le certificat auto-signé : voir [WEB-CLIENT-PI.md §8](WEB-CLIENT-PI.md) et les trois revues dans `docs/reviews/`.

## Étape 8 — port 443 (redirection vers 8443), **après** la revue n° 5 et ton feu vert séparé (ADR-0017)

Ne se fait qu'une fois le déploiement des étapes 4 à 6 réussi (le nouveau binaire accepte `Host` sans port ; l'ancien le refuse). Le script ne touche ni le service, ni sa configuration, ni `/etc/nftables.conf`.

Contrôle préalable **lecture seule** (condition C1 de la [revue n° 5](../reviews/web-pi-redirect-443-review-5.md)) : `redirect to :8443` vise l'adresse **primaire** de l'interface d'entrée ; si `192.168.1.42` était une adresse secondaire de `wlan0`, le script annoncerait un succès faux.

```bash
pi 'ip -4 -o addr show dev wlan0'     # doit montrer 192.168.1.42/24 SANS le mot « secondary »
```

(Le relevé du 15 octobre 15:22 UTC montrait une seule adresse globale, `wlan0=192.168.1.42/24`.) Le test de bout en bout ci-dessous est **obligatoire** : s'il échoue, lancer `--remove` aussitôt.

```bash
cd "$R"
[ "$(shasum -a 256 scripts/pi-redirect-443-operator.sh | cut -d' ' -f1)" = 3e7f6d4f8b0bdfa33af09f7031f7862ddca992426507dbf0cd498a63fe1f4ef4 ] && echo "script redirection conforme" || echo "SCRIPT MODIFIÉ : STOP"
pi 'sudo -n /bin/bash -s -- --self-test' < scripts/pi-redirect-443-operator.sh          # redirect_self_test=PASS
pi 'sudo -n env CHILLCENTER_LAN_IP=192.168.1.42 /bin/bash -s --' < scripts/pi-redirect-443-operator.sh
```

Succès : `redirect=192.168.1.42:443->8443`, `unit_state=enabled/active`, `service_active=active`. Ensuite, depuis le Mac : `openssl s_client -connect 192.168.1.42:443` doit montrer l'empreinte `3018fb4d…`, et `https://192.168.1.42/` s'ouvre dans le navigateur.

**Retour arrière** : `pi 'sudo -n env CHILLCENTER_LAN_IP=192.168.1.42 /bin/bash -s -- --remove' < scripts/pi-redirect-443-operator.sh`. Le serveur continue de répondre sur `:8443`. Le trafic émis par le Pi lui-même n'est pas redirigé (sondes locales sur `:8443`).

**Risques résiduels acceptés (revue n° 5)** :
- Activer plus tard `nftables.service`, ou lancer `nft -f /etc/nftables.conf` / `systemctl restart nftables`, exécute le `flush ruleset` du fichier par défaut : la table `chillcenter_web` disparaît alors que l'unité reste « active » (`RemainAfterExit`). Remède : `systemctl restart chillcenter-redirect`.
- Si la table a déjà disparu, `systemctl stop chillcenter-redirect` met l'unité en `failed` (ExecStop sort en 1) ; sans conséquence fonctionnelle.
- Un échec entre la pose du fichier de règles et celle de l'unité laisse `/etc/chillcenter/redirect-443.nft` ; `--remove` règle le cas.
- Le script ne contrôle pas les droits de `/etc/chillcenter` avant que root n'y charge un fichier de règles : le dossier doit être `root:root` non inscriptible (à vérifier avec `pi 'stat -c %a:%u:%g /etc/chillcenter'`).
- Un retour arrière **du binaire** (étape 7) vers l'ancien build refuserait `Host` sans port : retirer d'abord la redirection (`--remove`), sinon le client web sur 443 répondrait `HOST_REFUSED`.

## Étapes 9 à 12 — écoute HTTP non chiffrée pour le client web (ADR-0018) — **codées et simulées, pas encore déployées**

Chaque étape exige : la revue indépendante n° 6 des octets exacts, puis ton feu vert. Même convention SSH (étape 1). Rien de tout cela n'a été exécuté sur le Pi.

| Fichier | SHA-256 |
| --- | --- |
| Binaire ARM64 (avec `http_listen`) | `ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7` (14 890 920 octets) |
| Archive web (repli `randomUUID`) | `24839b3d81f21632818903c5c097471a98086748303ac304477f23e480d00df2` |
| `scripts/pi-update-web-operator.sh` (V8, **inchangé**) | `dcf1866111ef756b53113adce323f90960f5edbf00835204da78097fd6528e29` |
| `scripts/pi-enable-http-operator.sh` | `fb88ceffb9558a7b4988f0a929e3696760952b6102f5fb3e83decabb616c748b` |
| `scripts/pi-redirect-80-operator.sh` | `07d49c26e46e307f0f06841c452a0a11c8609e540b110fb5c288e038da30f3a0` |

9. **Nouveau binaire et archive** : étapes 2 à 6 avec le script V8, `OLD_BINARY_SHA256=a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527` (binaire actuellement installé), `NEW_BINARY_SHA256=ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7`, `WEB_ARCHIVE_SHA256=24839b3d81f21632818903c5c097471a98086748303ac304477f23e480d00df2`. Le nouveau binaire sans `http_listen` se comporte comme l'actuel.
10. **Activer le listener** : `pi 'sudo -n env CHILLCENTER_LAN_IP=192.168.1.42 NEW_BINARY_SHA256=ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7 CONFIG_SHA256=ea24f2f0a98db2b0a080ab70113a2fbe8eb1006a20e2eae646d115448f3783f9 CHILLCENTER_TLS_CERT_PATH=/var/lib/chillcenter/data/tls/certificate.pem /bin/bash -s --' < scripts/pi-enable-http-operator.sh`. Refuse (42) si un client est connecté. Sauvegarde privée : `/root/chillcenter-rollback/server.json.ea24f2f0a98d`. Récupération automatique en cas d'échec. **Retour arrière** : même commande avec `CONFIG_SHA256=<hash affiché après activation> PREVIOUS_CONFIG_SHA256=ea24f2f0…3783f9` et `-s -- --revert`.
11. **Rediriger le port 80** : `pi 'sudo -n env CHILLCENTER_LAN_IP=192.168.1.42 /bin/bash -s --' < scripts/pi-redirect-80-operator.sh` (table `chillcenter_http`, unité `chillcenter-redirect-http.service`) ; retrait : `-s -- --remove`.
12. **Tests Mac → Pi** : `http://192.168.1.42/` depuis le Mac et un téléphone du Wi-Fi ; lecture d'un film MP4, MKV et AVI ; `https://192.168.1.42:8443` pour le client Mac inchangé.

Après un `--revert` de l'étape 10, la redirection du port 80 (étape 11) pointe vers un port fermé : la retirer aussi (`--remove`). L'exécution de l'étape 10 est réservée à une fenêtre où tu as un accès console au Pi (un redémarrage du service y est fait).

**Ordre impératif** : 9, 10, 11 (le script de redirection exige que le service écoute déjà sur `192.168.1.42:8080`). Un retour du binaire (étape 7) vers un build sans `http_listen` exige d'abord le `--revert` de l'étape 10.

