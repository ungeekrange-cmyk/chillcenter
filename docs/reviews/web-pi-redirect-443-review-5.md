# Revue indépendante n° 5 — redirection 443 → 8443 (15 octobre 2026)

Reviewer : agent distinct de l'auteur, lecture seule (aucune édition du dépôt, aucune connexion au Pi). Commit revu : `02c5b84`. Rapport reproduit tel que rendu ; seule la section « Traitement par l'auteur » finale est ajoutée.

---

REVUE INDÉPENDANTE N°5 (lecture seule, worktree web-pwa, HEAD 02c5b84). Aucun fichier du dépôt modifié (git status vide). Aucune connexion au Pi. VM Colima jetable utilisée puis `colima stop`. Scratch /tmp/review5-* supprimé.

## VERDICT : APPROVED_WITH_CONDITIONS
- Script de redirection, octets SHA-256 3e7f6d4f8b0bdfa33af09f7031f7862ddca992426507dbf0cd498a63fe1f4ef4 (vérifié identique). Aucune modification des octets requise ; les conditions sont documentaires ou des contrôles à ajouter au runbook.
- Delta Rust / binaire aarch64 SHA-256 a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527 (14765736 octets, vérifié) : APPROVED. Réserve : je n'ai pas recompilé, donc je ne prouve pas que ces octets proviennent du source 02c5b84.

### Conditions (à traiter avant le feu vert Pi pour l'étape 8)
- C1 (MEDIUM-1) Ajouter à l'étape 8 un contrôle lecture seule que chillcenter.local est l'adresse primaire de wlan0, c'est-à-dire sans le mot `secondary` : `ip -4 -o addr show dev wlan0`. Ou bien exécuter obligatoirement le test de bout en bout `openssl s_client -connect chillcenter.local:443` et lancer `--remove` s'il échoue. Voir finding 1.
- C2 (LOW-1) Corriger la ligne 124 du runbook (hash périmé `b804…`).
- C3 (LOW-2) Documenter les risques résiduels des findings 3, 4 et 5.

## Hashes et artefacts (exécuté, `shasum -a 256`)
- pi-redirect-443-operator.sh : 3e7f6d4f…4ef4 (conforme).
- pi-update-web-operator.sh : dcf18661…8e29 (conforme, inchangé).
- chillcenter-web.tar : 894fbe95…7118 (conforme).
- chill-server aarch64 : a7ab7b07…d527 (conforme), `stat -f %z` = 14765736 (conforme).

## Findings

### 1. MEDIUM-1 : `redirect to :8443` cible l'adresse primaire de l'interface d'entrée ; le script annonce un succès faux si l'IP est secondaire
- Preuve exécutée dans la VM :
  - Interface `r5h` avec 10.98.0.9/24 (primaire) et 10.98.0.1/24 (`scope global secondary`).
  - Service factice `chillcenter` (unité transitoire, python http.server) sur 10.98.0.1:8443. Depuis un netns client, `curl http://10.98.0.1:8443/` donne `8443:200`.
  - Installation avec CHILLCENTER_LAN_IP=10.98.0.1 : toutes les vérifications passent. Sortie : `preconditions_ok`, `redirect=10.98.0.1:443->8443`, `unit_state=enabled/active`, `service_active=active`. J'ai lancé le script par chemin de fichier, et non par stdin comme en production, car l'outil refusait la forme stdin ; les octets sont identiques.
  - Ensuite `curl http://10.98.0.1:443/` donne `secondary-443:000` (exit 7). Le trafic est redirigé vers 10.98.0.9:8443, où personne n'écoute.
- La précondition ligne 73 vérifie seulement que l'adresse appartient à l'hôte. La vérification finale (lignes 92-96) ne teste aucune connexion réelle.
- Sur le Pi, c'est probablement sans effet : wlan0 en DHCP avec une seule IPv4. Ce n'est pas prouvé, d'où la condition C1.
- Correction possible, non requise sur ces octets : `dnat to chillcenter.local:8443`, ou refuser l'adresse marquée `secondary`.

### 2. LOW-1 : hash périmé dans le runbook
- Fichier `docs/deployment/WEB-CLIENT-PI-RUNBOOK.md`, ligne 124 : « préflight : si `installed_binary_sha256=b804…` ». C'est l'ancien hash b80408da…, alors que le nouveau binaire est a7ab….
- Un opérateur suivant cette ligne conclurait à tort « pas déployé ».
- Le commit a remplacé `2412…` par `b804…` au lieu de `a7ab…`.
- Les autres occurrences sont cohérentes : lignes 58, 101, 110, 114 et 140 du runbook ; lignes 29 et 59 de WEB-CLIENT-PI.md.

### 3. LOW-2 : risque résiduel non documenté, nftables.service
- Le fichier /etc/nftables.conf par défaut commence par `flush ruleset`. Si quelqu'un active nftables.service plus tard, ou lance `systemctl restart nftables` ou `nft -f /etc/nftables.conf`, la table `ip chillcenter_web` disparaît.
- L'unité `chillcenter-redirect` ne se recharge pas d'elle-même. Elle resterait `active` (RemainAfterExit) alors que la règle n'existe plus.
- Ni l'ADR ni le runbook ne le mentionnent. Seul le commentaire d'en-tête du script dit « neither edited nor enabled ». Je n'ai pas exécuté de `flush ruleset` dans la VM : cela aurait cassé le réseau Docker de Colima.
- Boot : l'ordre de l'unité est correct sans `After=`. Sortie de `systemctl show` : `Requires=sysinit.target system.slice`, `After=basic.target sysinit.target system.slice systemd-journald.socket`, `Before=multi-user.target shutdown.target`, `DefaultDependencies=yes`. La règle ne dépend que du noyau nft et d'une adresse littérale, donc aucun problème.

### 4. LOW-3 : ExecStop échoue si la table a déjà disparu
- Preuve exécutée : après `nft delete table ip chillcenter_web` puis `systemctl stop`, l'unité passe à `failed (Result: exit-code)`. ExecStop sort avec le code 1.
- Aucun impact fonctionnel. Un préfixe `-` sur ExecStop aurait évité cet état.
- `--remove` n'est pas affecté : `disable --now ... || true`.

### 5. LOW-4 : fenêtres d'échec mineures
- Si `install` du fichier d'unité échoue après celui des règles (ligne 84, avant `trap failed ERR` en ligne 87), le fichier /etc/chillcenter/redirect-443.nft reste. Une relance dit « use --remove first » ; `--remove` règle le cas. Code relu, non exécuté.
- Si `systemctl disable` échoue silencieusement dans `remove_all`, un lien symbolique wants peut rester. Le contrôle « removal incomplete » ne le regarde pas. Non exécuté.
- Exécution via `bash -s` sur stdin : une connexion coupée en cours de transfert exécute les lignes complètes déjà reçues. Les fichiers /etc/chillcenter et unité ne sont créés qu'après la ligne 79, donc le risque est faible.
- Les permissions du dossier /etc/chillcenter (propriétaire, droits d'écriture) ne sont pas contrôlées avant que root n'exécute `nft -f` sur ce fichier. Il faut qu'il soit root:root non inscriptible ; je n'ai aucune donnée du Pi pour le confirmer.
- La liste d'outils de la ligne 50 omet `ip` (ligne 73). S'il manquait, le message « not an address of this host » serait trompeur, mais aucun changement n'aurait eu lieu (échec avant toute écriture).

### 6. Point positif : script (tâche 1), validation d'IP et robustesse, tout exécuté
- `--self-test` dans la VM : `redirect_self_test=PASS`.
- Le regex est ancré et sans zéros initiaux (`^(0|[1-9][0-9]{0,2})...$`). Octets ≤255 contrôlés. Rejetés : 0.x, 127.x, ≥224, nouvelle ligne, espaces, `;`, `192.168.01.42`.
- Un appel non-root avec une adresse se terminant par une nouvelle ligne répond `invalid LAN address` ; avec `chillcenter.local` propre, `must run as root`. Avec des chiffres arabes (`١٩٢.168.1.42`) : `invalid LAN address`.
- Injection de règle nft ou d'unité : impossible, car seuls des chiffres et des points passent, et `${lan_ip//./\\.}` n'est utilisé qu'après validation. Un `printf` est utilisé pour le rendu.
- Link-local (169.254.x.x) et adresses publiques sont acceptés par la validation ; seule la précondition « adresse locale » les borne. LOW, sans conséquence.
- Aucune modification avant toutes les préconditions : la première écriture est `mktemp` (ligne 79). Ordre : IP, root, outils, nft, symlinks, service actif, écoute 8443 sur l'IP, rien sur 443, adresse locale, table absente, fichiers absents, unité non activée.
- TOCTOU : si un process prend 443 entre le contrôle et l'activation, la redirection masque simplement ce listener. Non bloquant.
- `set -Eeuo pipefail` avec le trap ERR : le trap `failed` est posé après l'installation des fichiers (ligne 87). Avant, un échec ne déclenche que `cleanup_tmp` par le trap EXIT, avec les temporaires `.redirect-443.*`, ce qui est cohérent. Dans `failed`, `trap - ERR` évite les récursions, puis `remove_all` s'exécute.
- Service chillcenter et autres tables nftables : le script n'emploie jamais `flush ruleset` et ne touche pas /etc/nftables.conf. Dans la simulation, `inet cc_other` reste intacte à l'installation et au retrait.
- Persistance au redémarrage, preuve exécutée : installation, puis `colima stop` et `colima start` (nouveau boot, `uptime -s` = 16:36:08). La table `ip chillcenter_web` était présente avec la règle `ip daddr 10.98.0.1 tcp dport 443 redirect to :8443`, `systemctl is-active` = active, `is-enabled` = enabled. Les tables Docker (`nat`, `filter`) coexistent. C'est une preuve en VM Ubuntu, pas sur le Pi.
- `--remove` exécuté deux fois : `removed_utc=...` puis `service_active=inactive` à chaque fois (idempotent). Le dossier /etc/chillcenter est vide et aucun lien wants ne reste.
- IPv6 : la table est de famille `ip`, donc IPv4 seul, comme le service. Rien à signaler.
- Sécurité de l'ouverture du 443 : le même service est déjà joignable en 8443 par tout client du LAN, même TLS et même authentification. La seule nouveauté est le Host sans port (voir Rust). `ip daddr` limite aux paquets destinés au Pi.

### 7. Simulation « 40 PASS, 0 FAIL » : rejouée, exacte, mais elle ne prouve pas tout
- Commande : `colima ssh -- sudo -n bash <worktree>/scripts/pi-redirect-simulation.sh <worktree>`. Résultat : `== résultat : 40 PASS, 0 FAIL`. L'environnement était nftables v1.0.9, Ubuntu 24.04, noyau 6.8, et non Debian 13 / nft 1.1.3 / noyau 6.18.
- Limites : une seule IP sur l'interface ; pas de test de redémarrage dans le harnais (le mien est ci-dessus) ; aucun test de `--self-test`.
- Mutants exécutés (copie du dépôt dans /tmp de la VM, via le répertoire REPO ; le harnais calcule R7/R8 à partir de l'opérateur du REPO) :

| Mutant | Résultat |
| --- | --- |
| m1a : retire `ip daddr` du rendu seul | Détecté (12 FAIL) |
| m1b : retire `ip daddr` du rendu et de la vérification | Détecté seulement par accident (5 FAIL : les `sed` de R7/R8 ne s'appliquent plus, « sed sans effet », R8 fichiers installés). Aucun test comportemental. |
| m1c : même mutation en conservant le texte de la ligne 27 | **NON DÉTECTÉ : 40 PASS, 0 FAIL** |
| m2 : retire `trap cleanup_tmp EXIT` | Détecté (R8 temporaire résiduel, 39/1) |
| m3 : port 8444 dans le rendu | Détecté (12 FAIL) |
| m4 : retire `trap failed ERR` | Détecté (3 FAIL) |
| m5 : retire le contrôle 443 occupé | Détecté (5 FAIL) |
| m6 : `--remove` ne supprime pas l'unité | Détecté (7 FAIL) |
| m7 : retire ExecStop | Détecté (R3) |
| m12 : ajoute `nft flush ruleset` | Détecté (2 FAIL) |
| m13 : `enable` sans `--now` | Détecté (11 FAIL) |
| m14 : retire `[Install]` | Détecté (1 FAIL) |
| m18 : `failed` ne supprime pas fichiers/unité | Détecté (2 FAIL) |
| m8 : regex d'IP sans ancre de fin | NON DÉTECTÉ (40 PASS), masqué par les préconditions suivantes. Détecté par `--self-test` : `accepted what must be refused ... chillcenter.local.1`. |
| m9 : retire le contrôle de symlinks | NON DÉTECTÉ (40 PASS) |
| m10 : retire le contrôle « table existe déjà » | NON DÉTECTÉ (40 PASS), masqué par le contrôle des fichiers existants |
| m11 : retire l'interdiction loopback/0/≥224 | NON DÉTECTÉ (40 PASS), masqué par « rien n'écoute ». Détecté par `--self-test` (`accepted ... 127.0.0.1`). |
| m16 : retire le contrôle « chillcenter actif » | **NON DÉTECTÉ (40 PASS)** |
| m17 : retire le contrôle « adresse locale » | NON DÉTECTÉ (40 PASS), masqué par le contrôle d'écoute |
| m19 : `--remove` sans suppression explicite de la table | NON DÉTECTÉ (40 PASS), masqué par `disable --now` (ExecStop) |

- Conclusion : le chiffre 40/40 est exact, mais il valide surtout le chemin nominal et les refus évidents. Les non-détectés les plus importants sont m1c (la restriction `ip daddr`, qui est l'objet central de la règle) et m16. À ajouter dans le harnais : un test où un paquet vers une autre adresse locale n'est pas redirigé, et un test avec le service arrêté.

### 8. Delta Rust : APPROVED
- `git diff 623a768 02c5b84 -- crates/chill-server` : une condition dans `check_browser_origin_and_host` (port de `listen` OU 443, nom toujours exigé dans `tls_names`, comparaison insensible à la casse) et un nouveau test.
- Tests exécutés : `cargo test --locked --offline -p chill-server --lib` donne `test result: ok. 105 passed; 0 failed` (le nouveau test `default_https_port_is_accepted_for_a_redirected_443_but_names_and_other_ports_are_not` est `ok`). Dossier cible supprimé.
- Origin : comparé textuellement à `https://{Host}`. Donc `Host: chillcenter.local` avec `Origin: https://chillcenter.local:8443`, ou l'inverse, donne 403. Le test le couvre pour `localhost`. Ce cas précis (IP + 8443) est conclu par lecture du code, non rejoué en direct.
- DNS rebinding : un nom hors de `tls_names` reste refusé, quel que soit le port (`attacker.example:443` est testé).
- Comportement de `http::uri::Authority` (http 1.5.0), exécuté dans un petit programme scratch hors dépôt :
  - `localhost:` donne port_u16=None, donc accepté comme 443.
  - `localhost:65536` donne None, donc accepté comme 443 (avant ce delta, refusé).
  - `localhost:00443` et `localhost:+443` donnent Some(443), acceptés.
  - `localhost:0` donne Some(0), refusé.
  - `localhost.:443` : le nom est `localhost.`, absent de `tls_names`, donc refusé.
  - `user@localhost` : l'hôte lu est `localhost` (comportement antérieur au delta, inchangé).
  - Formes IPv6 crochets : l'hôte est lu avec crochets puis nettoyé, comme avant.
- LOW-5 : l'acceptation de `host:` (port vide) et de `host:65536` (port invalide traité comme 443) est une régression de rigueur sans exploitation : un navigateur ne peut pas forger Host, et le nom reste exigé. Une correction possible : distinguer « absent » de « invalide ».

### 9. Documentation et runbook (tâche 5)
- Étape 8 : l'ordre est correct (après les étapes 4 à 6, nouveau binaire déployé), les commandes et le hash 3e7f6d4f… sont corrects, et le retrait est présent. Le tableau d'ensemble en tête du runbook ne liste pas l'étape 8 (LOW).
- Le retour arrière binaire de l'étape 7 n'indique pas qu'il faut retirer la redirection d'abord : l'ancien binaire donne `HOST_REFUSED` sur 443. L'ADR le mentionne, pas le runbook (LOW).
- WEB-CLIENT-PI.md ligne 3 : « source du binaire 3a0d53a4 » est périmé. Le binaire a7ab… a été construit après le commit 1a4019f, qui modifie `crates/chill-server/src/api.rs` (LOW).
- WEB-CLIENT-PI.md ligne 58 : « 105 + 2 tests » est ambigu ; j'observe 105 tests au total (LOW).
- L'ADR parle de « règle DNAT » alors que le script utilise `redirect` (LOW).

## Ce que je n'ai pas pu vérifier
- Aucune exécution sur le Pi réel (interdit) : l'adresse primaire de wlan0, les permissions de /etc/chillcenter, le comportement avec nft 1.1.3, noyau 6.18 et Debian 13. Les modules nft_nat et nft_redir ne sont pas chargés sur le Pi (d'après les faits fournis) ; l'autochargement n'est prouvé qu'en VM.
- L'exécution par stdin (`bash -s`) du script d'installation : l'outil de revue la refusait pour la forme de la commande. J'ai exécuté le même fichier par chemin. La simulation fournie utilise la forme stdin et passe.
- L'effet réel d'un `flush ruleset` ou d'un `nftables.service` activé (non exécuté : risque pour la VM). Le raisonnement est tiré du contenu documenté de /etc/nftables.conf.
- La provenance du binaire a7ab… vis-à-vis du source (pas de recompilation) ; clippy ; un navigateur réel (Origin et Sec-Fetch-Site réels, certificat auto-signé).
- Les mutants m8, m10, m11, m17 et m19 ne sont pas détectés par le harnais ; m8 et m11 le sont par `--self-test` (exécuté). Je n'ai pas exécuté `--self-test` sur m10, m17 et m19, qui ne concernent pas la validation d'IP.

---

## Traitement par l'auteur (après le rapport)

| Constat | Traitement |
| --- | --- |
| C1 / MEDIUM-1 (adresse secondaire) | runbook étape 8 : contrôle lecture seule `ip -4 -o addr show dev wlan0` (sans `secondary`) et test de bout en bout obligatoire avec `--remove` en cas d'échec ; le relevé du 15 oct. 15:22 UTC montrait une seule adresse globale `wlan0=chillcenter.local/24` |
| C2 / LOW-1 (hash `b804…` ligne 124) | corrigé : `a7ab…` |
| C3 / LOW-2, LOW-3, LOW-4 | listés comme risques résiduels acceptés dans le runbook, étape 8 ; octets du script **non modifiés** (conforme au verdict) |
| Mutants non détectés m1c et m16 | harnais renforcé : R3b (autre adresse locale non redirigée) et R9 (service arrêté : refus) ; simulation **43 PASS, 0 FAIL** ; mutant m1c refait fidèlement (commentaire nft contenant le texte attendu) : détecté par R3b (attendu 000, obtenu 403). Restent non détectés par la simulation et **acceptés** : m8, m11 (détectés par `--self-test`), m9, m10, m17, m19 (masqués par d'autres contrôles) |
| LOW-5 (Host `host:` ou `host:65536` traité comme 443) | **accepté, non corrigé** : le nom reste exigé, un navigateur ne forge pas `Host`; correction possible plus tard (distinguer absent / invalide), elle changerait le binaire |
| Documentation (tableau d'ensemble, étape 7, source du binaire, « 105 + 2 », mot DNAT) | corrigés |
| Provenance du binaire non prouvée par le reviewer | le binaire a été construit après le commit `1a4019f` (`cargo zigbuild --release --locked --offline`) ; les sources n'ont pas changé depuis dans `crates/` |
