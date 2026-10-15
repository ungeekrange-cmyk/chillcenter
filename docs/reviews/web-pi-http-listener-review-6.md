# Revue indépendante n° 6 — listener HTTP clair (ADR-0018) (15 octobre 2026)

Reviewer : agent distinct de l'auteur, lecture seule (aucune édition du dépôt, aucune connexion au Pi, injoignable à ce moment). Commit revu : `47d42ec`. Rapport reproduit tel que rendu ; seule la section « Traitement par l'auteur » finale est ajoutée.

---

# Revue indépendante n°6 : listener HTTP clair (ADR-0018)

Commit revu : 47d42ec, branche codex/web-pwa. Je n'ai modifié aucun fichier du dépôt (`git status` vide en fin de revue). Les fichiers scratch Mac sous /tmp/review6-* et le répertoire /tmp/review6-vm dans la VM sont supprimés, `colima stop` est fait. Je n'ai jamais contacté le Pi.

## Verdict : APPROVED_WITH_CONDITIONS

| Objet | SHA-256 vérifié | Verdict |
|---|---|---|
| scripts/pi-enable-http-operator.sh | e5f687d9…ddd14 (identique à l'attendu) | APPROVED_WITH_CONDITIONS (C1, C2) |
| scripts/pi-redirect-80-operator.sh | c0f8e000…837e9e (identique) | APPROVED (mêmes risques résiduels que la revue 5) |
| scripts/pi-redirect-443-operator.sh (référence) | 3e7f6d4f…4ef4 (identique) | inchangé |
| scripts/pi-update-web-operator.sh | dcf18661…8e29 (identique, inchangé depuis la revue 4) | inchangé |
| Binaire ARM64 (delta Rust) | 92917152b3a954fe682c0a788011bfa389aa72d3f4ca5824609bfcb3ede379a0, 14884936 o (vérifié, ELF aarch64 stripped) | APPROVED_WITH_CONDITIONS (C3) |
| Archive web | f8a051ae…9518 (identique) | APPROVED |

Conditions avant le go sur le Pi :
- C1 : accepter ou corriger les findings 2 et 3 sur le script d'activation. Une correction change les octets et impose une nouvelle revue.
- C2 : l'exécution reste réservée à une fenêtre où l'opérateur a un accès console au Pi. Le script ne se protège pas contre une coupure de session SSH pendant la fenêtre à risque (finding 3).
- C3 : documenter dans l'ADR que le listener HTTP est vulnérable à une saturation par connexions inactives (finding 1), ou le corriger avant déploiement.

## Findings

1. **MEDIUM, listener HTTP saturable par 16 connexions TCP inactives. Mesuré.**
   - Cause : le second `LimitedAcceptor(16)` n'a pas de délai de poignée de main (le TLS en a un de 5 s). Le sniff de protocole de hyper-util `auto` n'a pas de timeout, donc `header_read_timeout(5s)` ne s'applique pas à une connexion qui n'envoie aucun octet.
   - Essai dans la VM, avec le vrai binaire (config loopback `http_listen` 127.0.0.1:18080, `listen` 127.0.0.1:18443) et 16 sockets ouvertes puis laissées muettes pendant 8 s :
     - Port HTTP 18080 : `still open after 8s: 16 of 16`, puis `probe failed: ConnectionResetError(104, 'Connection reset by peer')`.
     - Port HTTPS 18443 : `still open after 8s: 0 of 16`, la sonde atteint le serveur TLS.
   - Conséquence : un appareil du LAN, ou le preconnect spéculatif de plusieurs navigateurs, peut rendre le port HTTP indisponible tant que les sockets restent ouvertes. Le HTTPS n'est pas touché (pools séparés), le processus non plus. C'est une indisponibilité, pas une compromission, mais ADR-0018 écrit « mêmes limites » et ce n'est pas exact.
   - Correction suggérée : timeout sur la première lecture, ou n'accepter qu'avec un délai d'inactivité sur le pool HTTP.
   - À écarter du reste : le principe du second listener est sain.

2. **MEDIUM, `revert_now` n'est pas robuste si la restauration échoue (analyse du code, non exécuté).**
   - Ligne 98 : `install … "$backup" "$candidate.restore" && mv …`. Si `install` échoue (disque plein, EIO), la liste `&&` ne déclenche ni `errexit` ni ERR. Le script enchaîne alors sur `systemctl restart` avec la nouvelle configuration toujours en place.
   - Si cette configuration est invalide, on obtient le cas redouté : configuration cassée et service arrêté.
   - Correction : préparer `$candidate.restore` avant le `mv` de la ligne 104, comme `.final`, de sorte que la récupération se réduise à un `mv` atomique. Vérifier aussi le hash de la sauvegarde après la copie (voir finding 4).
   - Mon essai de simulation de cet échec (mutant « restauration depuis /nonexistent ») s'est enchaîné sur un état résiduel d'une étape précédente de la simulation et n'est pas concluant. Je ne présente pas ce point comme démontré.

3. **MEDIUM/LOW, aucun trap sur HUP/INT/TERM (analyse, non exécuté).**
   - Le trap ERR n'arme la récupération que sur échec d'une commande. Une coupure SSH ou un Ctrl-C entre le `mv` (ligne 104) et la fin de la vérification n'exécute que l'EXIT trap `cleanup`, qui ne supprime que des temporaires.
   - Le résultat n'est catastrophique que si la nouvelle configuration est invalide en plus de l'interruption. La configuration est déjà validée (hash, JSON via python3 s'il est présent, diff d'une ligne). Le risque est faible mais réel.
   - Correction : `trap revert_now HUP INT TERM` après la ligne 103, ou ignorer ces signaux pendant la fenêtre.
   - `systemctl restart` n'a pas non plus de timeout propre ; il est borné par les timeouts de l'unité.

4. **LOW, sauvegarde non revérifiée contre le hash (TOCTOU théorique).**
   - Le hash de la configuration est vérifié ligne 58, puis le fichier est relu aux lignes 89 et 93. `cmp` ligne 94 compare config et sauvegarde, pas la sauvegarde au hash attendu.
   - Exploitable seulement par quelqu'un qui peut écrire dans /etc/chillcenter, donc pratiquement root. Un `sha256sum "$backup" == "$previous_sha"` après la copie règle le point.

5. **LOW, mutants non détectés par la simulation d'activation.** Voir le tableau plus bas. Les deux angles morts réels :
   - M5 : en cas de récupération automatique, la restauration perd propriétaire et mode (root:nogroup 640 devient root 600) sans que E4 ne le voie. E4 ne contrôle que les octets. Un service qui tourne sous un utilisateur non root ne pourrait alors plus lire sa configuration au redémarrage. Les tests E1 et E6 couvrent le chemin nominal et `--revert`, pas la récupération automatique.
   - M6 : suppression du garde « une ligne ajoutée, aucune supprimée » (ligne 91), non testée.

6. **LOW, ADR-0018 sous-estime la surface.**
   - Le tableau d'options dit « pour le web seulement », alors que le routeur HTTP expose toute l'API, y compris `/admin/*`.
   - La liste des données en clair omet les secrets : `PUT /admin/tmdb` (clé TMDB), export et import d'archives. Le client web ne les expose pas (grep sur web/src : aucune occurrence de `admin/tmdb`), mais l'API sur le port 8080 les accepte de n'importe quel appareil du LAN.
   - Ajouter : ne jamais saisir de clé TMDB ou d'archive via le port HTTP.
   - Le reste (écoute en clair sur le LAN, usurpation du serveur par un appareil du réseau) est dit honnêtement.

7. **LOW, doc.**
   - L'en-tête de pi-redirect-80-operator.sh dit « Mechanically derived from pi-redirect-80-operator.sh ». C'est une auto-référence, il faut lire 443. Cela change les octets, donc le hash. À corriger seulement si vous refaites une passe, sinon à tolérer.
   - PI-LIVE-STATE ne cite pas le nouveau binaire 92917152 (à ajouter une fois déployé).
   - PI-LIVE-STATE n'avertit pas qu'après l'étape 10 le hash de `server.json` change : tout script d'un autre agent qui pine `ea24f2f0…` refusera de tourner, ce qui est le comportement voulu mais gagnerait à être écrit.
   - Tout script qui réécrit `server.json` supprimerait `http_listen`.
   - Le reste de PI-LIVE-STATE est exact et honnête : état Pi non revérifié, `deny_unknown_fields`, `flush ruleset`, noms réservés, hashes.

## Réponses aux questions

### 1. Delta Rust
- **Portée du routeur HTTP.**
  - `validate` impose adresse locale (privée, link-local, loopback), port non nul et différent du HTTPS.
  - Le pair est de plus filtré par `is_local_address` dans `LimitedAcceptor` (code lu).
  - Rien d'exposable hors LAN, sauf une redirection de port faite sur la box ou un NAT en masquerade (risque déjà présent en HTTPS, documenté dans l'ADR).
- **CSRF et cross-origin sur la branche `(None, Some(origin))` en Http.**
  - Dans les navigateurs, `Origin` est toujours posée sur une requête non GET/HEAD (spec Fetch, « append a request Origin header »). Elle vaut `null` en iframe sandboxée, après redirection cross-origin, ou avec `no-referrer`. `Origin: null` est refusée (test existant), de même que http://autre, https://même-hôte-port, un autre port, et `""`.
  - Les formulaires cross-site, `fetch no-cors` POST et EventSource cross-origin portent une `Origin` étrangère et sont refusés.
  - HTTP vers HTTPS : l'origine https://…:8443 ne correspond pas à http://Host, donc refus.
  - DNS rebinding : le `Host` doit appartenir à `tls_names` avec le bon port, donc refus (test).
  - Reste accepté : GET sans en-tête. Les GET cross-site ne sont pas lisibles (pas de CORS). J'ai relu les routes GET (stream, events, download d'export, scan status) : aucun effet de bord. Le GET `/media/{id}/stream` reste embeddable cross-site mais non lisible, comme avant.
- **HTTPS inchangé.** La branche `(None, Some)` est gardée par `scheme == Http`, le défaut sans extension est `Https`, le préfixe, le port par défaut et le port d'écoute HTTPS sont identiques. Le test `https_listener_still_refuses_an_origin_without_fetch_metadata` passe.
- **Sémantique de `tokio::select!`.** Si un listener se termine (erreur de bind comprise, car `serve` lie au premier poll), `run()` retourne `Err` ou `Ok` et l'autre future est abandonnée : le processus s'arrête, pas de demi-service. Une panique dans une tâche de connexion n'arrête pas les listeners.
- **Ressources.** Deux sémaphores de 16 : au pire 32 connexions simultanées, négligeable en RAM. Aucun deadlock identifié. Seule régression : le finding 1.
- **Tests exécutés :** `cargo test --locked --offline -p chill-server --lib` donne `108 passed; 0 failed`. Le répertoire target /tmp/review6-target a été supprimé. Je n'ai pas reconstruit le binaire : je n'ai vérifié que son hash, sa taille et son format ELF aarch64. Je ne peux donc pas affirmer que ces octets sont reproductibles depuis la source.

### 2. Web
- `node --test web/test/*.test.js` : 51 pass, 0 fail.
- `uuidV4` produit un UUID RFC 4122 v4 valide : bits de version et de variante corrects (`b[6]` en `0x4_`, `b[8]` en `[89ab]_`), formatage correct, source `getRandomValues` (CSPRNG, disponible hors contexte sécurisé), `randomUUID` préféré s'il existe.
- Le test couvre le cas non sécurisé.
- Aucun autre usage de `crypto` sensible au contexte sécurisé dans web/src, et le Service Worker n'est enregistré qu'en https.

### 3. pi-enable-http-operator.sh
- **Chemins de récupération.**
  - `trap revert_now ERR` est armé juste avant le `mv`.
  - `wait_listening` et les `[[ ]]` de vérification, appelés en commande simple, déclenchent bien errexit puis revert (E4 et E5 le confirment en VM).
  - `revert_now` retire d'abord `trap - ERR`, donc pas de récursion.
  - Voir les findings 2 et 3 pour les deux trous.
- **Ordre des traps.** L'EXIT trap `cleanup` (ligne 88) précède l'ERR trap de récupération (ligne 103), cohérent.
- **Mode `--revert`.**
  - Les erreurs dans ce mode sortent en non-zéro avec la configuration d'origine déjà en place, ce qui est acceptable.
  - Testé en VM : mauvais hash courant refusé (2), sauvegarde invalide refusée (2), succès conserve propriétaire et mode.
- **Injection via CHILLCENTER_LAN_IP.**
  - La regex ancrée, l'interdiction des zéros de tête, la borne à 255 et le refus de `0`, `127` et `≥224` couvrent `;` et `\n`.
  - Le `--self-test` passe dans la VM (`enable_http_self_test=PASS`).
  - La simulation vérifie `10.99.0.1;ls`, refus (2).
- **Pipeline `diff | grep -c || true`.** Correct sous pipefail : le compte est imprimé même si le statut est non nul.
- **Format du fichier de configuration.**
  - Le `awk` n'accepte que `^{[ \t]*$`. Avec CRLF ou une accolade non seule sur sa ligne, il sort en code 3 avant toute modification, donc refus sûr.
  - Une configuration sans retour à la ligne final fait échouer le garde du diff.
  - Ces cas CRLF, tabulations et `{` non seul ne sont PAS exécutés (le garde d'environnement refusait mes commandes de test). C'est du raisonnement sur la regex, pas un résultat de test.
  - Absence de python3 : la validation JSON est simplement ignorée.
  - Le `awk` de la VM est celui d'Ubuntu 24.04 (mawk par défaut) ; non vérifié sur le Pi.
- **Connexions établies.** Le nom du processus est `chill-server` (12 caractères), sous la limite de 15 de `comm`. L'échec est « fail-open » (0 si `ss` échoue), mais le test E3 prouve que le contrôle fonctionne avec `-p`. Il refuse aussi si le serveur a une connexion sortante établie (sens sûr).
- **Propriétaire et mode.** Préservés par `install` en nominal (E1, E6), pas en récupération automatique, non testé : finding 5.

### 4. pi-redirect-80-operator.sh
Le diff complet avec pi-redirect-443-operator.sh ne contient que les renommages et ports attendus : en-tête et commentaires, `table=chillcenter_http`, `service_port=8080`, `redirect-80.nft`, `chillcenter-redirect-http.service`, `dport 80`, regex d'occupation du port 80, noms des `mktemp`, chaînes de vérification et de sortie, unité ADR-0018, attendus du self-test. Aucun changement de logique. Seule anomalie : le commentaire du finding 7. Les risques résiduels de la revue 5 (adresse secondaire, `flush ruleset` si nftables.service est activé plus tard) s'appliquent inchangés.

### 5. Simulations rejouées dans la VM (Ubuntu 24.04 aarch64, systemd 255)
- pi-enable-http-simulation.sh : `39 PASS, 0 FAIL`, avec la ligne d'en-tête affichant le hash e5f687d9…ddd14.
- pi-redirect-http-simulation.sh : `42 PASS, 0 FAIL`.
- pi-redirect-simulation.sh : `43 PASS, 0 FAIL`.
- web-e2e-linux.sh : `36 PASS, 0 FAIL`.
- Limite : l'unité de la simulation d'activation est `Restart=no`, ce qui diffère de la vraie unité.

Mutants du script d'activation, exécutés dans la VM contre la simulation :

| Mutant | Résultat |
|---|---|
| M1 : pas d'auto-revert (`trap - ERR`) | détecté (4 FAIL) |
| M2 : configuration restaurée sans redémarrage | détecté par un seul test (E4 « HTTP coupé ») |
| M3 : contrôle des connexions établies désactivé | détecté (E3) |
| M4 : sauvegarde supprimée | détecté (E1, E4, E5, E6) |
| M5 : restauration automatique sans propriétaire ni mode | **NON détecté (39 PASS)** |
| M6 : garde du diff (une ligne ajoutée) supprimé | **NON détecté (39 PASS)** |
| M7 : vérification « HTTP API = 200 » supprimée | **NON détecté (39 PASS)** ; les contrôles suivants (page, Origin 403) en couvrent l'essentiel, donc faible gravité |
| M8 : `--revert` sans redémarrage | détecté (1 test) |
| M9 : `--revert` sans contrôle de la sauvegarde | détecté (1 test) |

### 6. Artefacts
- Archive : 60 membres, uniquement fichiers ordinaires (56) et dossiers (4), tous sous `web/`, aucun chemin absolu ni `..`, aucun lien symbolique.
- `node scripts/build-web.mjs <tmp>` compare à l'extraction : `diff -r` ne montre que `SHA256SUMS` en plus dans l'archive.
- pi-update-web-operator.sh : dcf18661…8e29, inchangé.
- Binaire : hash et taille conformes, non reconstruit.

### 7. Docs
- **Runbook, étapes 9 à 12.** Hashes et commandes cohérents avec ce que j'ai calculé. Ordre correct : binaire, activation, redirection, tests. Le chemin du certificat est cohérent avec l'étape 5. Remarque : après un `--revert` de l'étape 10, la redirection du port 80 reste en place et pointe vers un port fermé. Ce n'est pas dangereux mais mérite une ligne.
- **ADR-0018** : voir finding 6, plus le statut `PROPOSED` normal à ce stade.
- **PI-LIVE-STATE** : voir finding 7.

## Ce que je n'ai pas pu vérifier
- Le Pi réel, le comportement de la vraie unité systemd du Pi (`Restart=`, durcissement), le `awk` et le `ss` du Pi.
- La reproductibilité du binaire à partir de la source (non reconstruit).
- Les cas CRLF, tabulations et accolade non seule : raisonnés, non exécutés (voir section 3).
- Les scénarios de coupure par signal (finding 3) et d'échec de restauration (finding 2) : raisonnés, non démontrés.
- Le comportement des navigateurs mobiles et des appareils de la maison sur http://192.168.1.42 (non prouvé par ce que j'ai exécuté ; l'ADR le reconnaît).
- La suite complète des tests Rust hors `--lib`, clippy, et les tests Node de l'archive déployée sur la cible.

---

## Traitement par l'auteur (après le rapport)

Les octets revus (`47d42ec` : binaire `92917152…`, scripts `e5f687d9…` et `c0f8e000…`) sont **remplacés** par ceux ci-dessous ; la revue n° 6 ne couvre donc plus les octets à déployer, **une revue de contrôle (n° 7) est requise**.

| Constat | Traitement |
| --- | --- |
| 1 (saturation par connexions muettes, mesurée) | **corrigé et mesuré** : `FirstByteAcceptor` (délai de 5 s sur le premier octet, `peek`) + `http1_only` sur le listener HTTP. Cause racine relevée en cours de travail : hyper ne démarre son délai d'en-têtes qu'après un octet reçu, `http1_only` seul ne suffisait donc pas (essai Linux : `idle_closed=0/16`, puis `16/16` avec l'accepteur). Test unitaire Rust + contrôle d'essai Linux « 16 connexions muettes fermées, service disponible » |
| 2 (restauration non robuste) | corrigé : le fichier de restauration est préparé et son hash vérifié **avant** de toucher à la configuration ; la récupération se réduit à un `mv` atomique |
| 3 (aucun trap HUP/INT/TERM) | corrigé : `trap revert_now ERR HUP INT TERM` ; scénario E5c (signal TERM dans la fenêtre : configuration d'origine restaurée, service actif) |
| 4 (sauvegarde non revérifiée) | corrigé : `sha256sum` de la copie contre le hash épinglé |
| 5 (mutants M5, M6 non détectés) | corrigé dans la simulation : E4 et E5 contrôlent propriétaire et droits après récupération ; E5b (insertion doublée refusée avant toute modification). M7 (vérification `API = 200`) **accepté**, couverte par les contrôles suivants |
| 6 (surface sous-estimée, secrets en clair) | corrigé : routes `/admin/tmdb*`, `/admin/exports*`, `/admin/imports*` refusées en HTTP clair (`HTTPS_ONLY`), test Rust et contrôles d'essai Linux ; ADR-0018 corrigé (surface exposée, disponibilité) |
| 7 (commentaire, PI-LIVE-STATE, redirection 80 après `--revert`) | corrigés ; runbook : accès console conseillé pour l'étape 10 et retrait de la redirection 80 après un `--revert` |

Nouveaux octets : binaire `ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7` (14 890 920 o), archive web `24839b3d81f21632818903c5c097471a98086748303ac304477f23e480d00df2`, `pi-enable-http-operator.sh` `fb88ceffb9558a7b4988f0a929e3696760952b6102f5fb3e83decabb616c748b`, `pi-redirect-80-operator.sh` `07d49c26e46e307f0f06841c452a0a11c8609e540b110fb5c288e038da30f3a0`. Résultats après correction : 110 tests Rust, essai Linux 40/40, simulation d'activation 46/46, redirections 42/42 et 43/43.
