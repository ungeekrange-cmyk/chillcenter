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
| 5 Exécution du script V7 | toi ou moi | **oui : le déploiement** (arrêt bref du service, sauvegarde privée, binaire, dossier web) |
| 6 Vérification et nettoyage | toi ou moi | supprime les 2 fichiers de `/var/tmp` |
| 7 Retour arrière | seulement si nécessaire | oui |

Tu peux coller toi-même chaque bloc. Ou bien tu m'autorises à les exécuter à travers **ta** connexion maître : dans ce cas écris explicitement « **Feu vert déploiement V7** » **et** le chemin du socket (étape 1), et je n'exécute que les étapes 2 à 6, dans l'ordre, en m'arrêtant à la première sortie inattendue.

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
chk target/aarch64-unknown-linux-gnu/release/chill-server               b80408da05e9f013c897c4599805494282a63afb3630f8e305faa90e0939c89f
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

Les deux hashes affichés par le Pi doivent être **exactement** `b80408da05e9f013…c89f` (binaire) et `894fbe952a58bdae…7118` (archive). Sinon : `pi 'rm -f /var/tmp/chill-server-web.new /var/tmp/chillcenter-web.tar'` et stop.

## Étape 5 — exécution du script V8, **par stdin** (le déploiement)

Relire à voix haute avant de coller : l'URL est `https://192.168.1.42:8443` (le script ne vérifie que le **port**, LOW-1), les deux hashes sont ceux de l'étape 4.

```bash
cd "$R"
[ "$(shasum -a 256 scripts/pi-update-web-operator.sh | cut -d' ' -f1)" = dcf1866111ef756b53113adce323f90960f5edbf00835204da78097fd6528e29 ] && echo "script V8 conforme" || echo "SCRIPT MODIFIÉ : STOP"
pi "sudo -n env OLD_BINARY_SHA256=cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3 NEW_BINARY_SHA256=b80408da05e9f013c897c4599805494282a63afb3630f8e305faa90e0939c89f WEB_ARCHIVE_SHA256=894fbe952a58bdae01527e8d8fa646e87977b9f1aea5bd996d770bacb71f7118 CHILLCENTER_TLS_CERT_PATH=/var/lib/chillcenter/data/tls/certificate.pem CHILLCENTER_BASE_URL=https://192.168.1.42:8443 /bin/bash -s --" < scripts/pi-update-web-operator.sh
echo "code de sortie : $?"
```

**Succès** (code 0) : 7 lignes `deployed_utc`, `service_pid`, `installed_hash=b804…c89f`, `rollback_hash=cef5…b3c3`, `web_archive_hash=894f…7118`, `web_previous=none`, `backup_path=/var/lib/chillcenter/data/backups/<uuid>`.

| Code de sortie / signe | Signification | À faire |
| --- | --- | --- |
| **0** | Déployé et vérifié | étape 6 |
| **2** | Refus **avant tout arrêt** (hash, archive, URL, port, sonde TLS, outil manquant, disque, rollback) | lire le message ; **rien n'a changé** ; corriger la cause |
| **42** | Une connexion cliente est établie | **rien n'a changé** ; attendre et recommencer |
| **1 ou 3** avec des lignes `RECOVERY:` | Échec après l'arrêt ; **restauration automatique** | lire les deux dernières lignes : `service is active` + `answers over strict TLS` = **ancien état rétabli** ; `ERR line N: …` dit ce qui a échoué ; **ne pas relancer** avant d'avoir compris |
| `RECOVERY: SERVICE IS NOT ACTIVE` ou `could NOT be restored` | La restauration n'a pas pu tout rétablir | **étape 7 (manuelle) immédiatement** |
| **129 / 130 / 143** | Interrompu (SSH coupé, Ctrl-C) | la récupération s'est exécutée ; refaire le **préflight** (étape 3) pour lire l'état réel |
| **1 sans aucune ligne `RECOVERY`**, sortie fermée | Probablement LOW-2 : déployé, résumé non imprimé | préflight : si `installed_binary_sha256=2412…` et service actif, c'est déployé ; vérifier l'étape 6 |
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

Attendu : hash binaire `24120568…`, `active`, un dossier `web` root, empreinte `3018fb4d…`, deux fichiers absents. Je peux ensuite refaire la sonde Node en lecture seule (catalogue, Range MKV/MP4/AVI, décodeur) contre le serveur réel.

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
