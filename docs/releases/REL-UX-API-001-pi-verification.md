# Vérification post-déploiement indépendante — CC-UX-002 / API 2026-10-14

- **Mode** : `DEV_STANDARD`, politique `INTEGRATION_AND_RELEASE`
- **Source approuvée** : `9277ac4d51281b3e6d72475294b1e7822cb8e913`
- **Carrier cible propre** : `a3f969f2d6c5d9a3c70beadbb1872bce0556fa8e`
- **Opérateur exact approuvé** : V4 SHA-256 `55c6be484d50a8065115ae4b1bf867377c24b255640c033e5fc2faf6f3289587`
- **Deployer** : `/root/ux_api_delivery`
- **Verifier** : `/root/ux_api_post_verify`, distinct du deployer, builder, debugger et reviewers
- **Déployé UTC** : `2026-10-14T19:12:40Z`
- **Vérifié UTC** : `2026-10-14T19:15:04Z` à `2026-10-14T19:16:30Z`
- **Verdict** : **APPROVED**

Ce verdict approuve la release DEV effectivement active et le parcours de connexion/catalogue borné décrit ci-dessous. Il ne transforme pas les exclusions physiques, fournisseur ou lecteur en succès.

## Provenance et exactitude du déploiement

- Le checkout cible est propre à `a3f969f2`; `9277ac4d` en est un ancêtre et les carriers ultérieurs ne changent pas les sources compilées.
- Le rapport opérateur V4 exact est `APPROVED`; les versions V2/V3 restent historiques et non approuvées.
- Le fichier installé **et** `/proc/5145/exe` ont le SHA-256 attendu `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`.

## Preuves runtime Pi

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Service | nouveau binaire actif et stable | `active/running`, PID `5145` inchangé sur deux lectures, `NRestarts=0` | PASS |
| Heure de démarrage | déploiement annoncé | systemd `2026-10-14T19:12:39Z`, cohérent avec fin opérateur `19:12:40Z` | PASS |
| Identité | non-root, GID NAS approuvé | UID `997`, GID `983`, groupes `983 984` | PASS |
| Confinement | politique R7 conservée | capabilities vides, `NoNewPrivileges=yes`, `ProtectSystem=strict`, `ProtectHome/PrivateTmp/PrivateDevices=yes` | PASS |
| Chemins systemd | état local RW, NAS RO | `ReadWritePaths=/var/lib/chillcenter`, `ReadOnlyPaths=/srv/volumes/video` | PASS |
| Unité fournisseur | inchangée | `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2` | PASS |
| Drop-in cwd | inchangé | `6bad2f8c529705b246253bb8d593d7ae58155b2eb23bacac7afb404e01a5bf1a` | PASS |
| Configuration | inchangée selon comparaison V4 | hash courant `ea24f2f0a98db2b0a080ab70113a2fbe8eb1006a20e2eae646d115448f3783f9`, fichier régulier `0640` root:GID983, mtime antérieur au déploiement | PASS |
| TLS public | SAN/IP et pin conservés | SAN `192.168.1.42`; SPKI `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df` | PASS |
| NAS | CIFS lecture seule | superposition observée : autofs `rw`, ligne CIFS `ro,nosuid,nodev,noexec` | PASS borné |
| Journaux | pas de nouveau signal warning+ | compteur systemd journal depuis déploiement : `0` | PASS |

La ligne autofs `rw` interdit d'affirmer que l'hôte entier est monté en lecture seule ; la preuve porte sur la ligne CIFS `ro` et `ReadOnlyPaths` de l'unité.

## Sauvegarde pré-déploiement

Racine privée : `/var/lib/chillcenter/data/backups/62584da8-ce12-45b6-9e1a-50e6accb5767`.

- Data root, parent `backups` et racine UUID : `0700`, UID `997`, GID `983`.
- Manifeste format `1`, schéma `3`, data root attendu, 5 entrées uniques ; catalogue et configuration déclarés et présents.
- 6 fichiers réels = 5 entrées + manifeste ; tailles déclarées exactes ; aucun symlink ; manifeste publié en dernier.
- `PRAGMA quick_check` exécuté avec SQLite `mode=ro&immutable=1` et `query_only=ON` : `ok`.
- Manifeste/configuration `0600`. Le catalogue feuille est `0644`, mais reste effectivement privé derrière les trois ancêtres `0700`; même note non bloquante que la sauvegarde précédente.

Les fichiers TLS privés de la sauvegarde n'ont été ni lus, ni hachés, ni copiés par le verifier. Aucun contenu de profil ou du catalogue n'a été affiché.

## TLS, compatibilité et contrat servi

Toutes les requêtes utilisent le certificat public obtenu par le canal SSH strict, `curl --cacert`, TLS ≥1.2 et **jamais** `-k`.

| Endpoint/contrôle | Observé | Résultat |
| :--- | :--- | :--- |
| `GET /api/2026-10-14/server` | HTTP `200`, `api_version=2026-10-14`, version listée comme supportée | PASS |
| `GET /api/v1/server` | HTTP `200`, `api_version=1` | PASS compatibilité |
| `GET /api/2026-10-14/openapi.json` | HTTP `200`, SHA-256 `d12c606e4d922c1a52f0fe6f12f72940b1498c442dce7ba43ac9f83f946778f4` | PASS |

## Sonde client Rust indépendante

Provenance : source `/private/tmp/chillcenter-connection-probe.F7F5yV/src/main.rs`, SHA-256 `977335e0a9ba6393ba480783a7e6c5bd4d319a4f69557e80edb96035ec287e11`, approuvée dans la revue opérateur. Binaire ARM64 exécuté : `/private/tmp/chillcenter-root-target/debug/chillcenter-connection-probe`, SHA-256 `1537c080fe3934311e26a361a0babb806d03404906dbdbe68a050c3a809037e6`.

Commande :

```text
/private/tmp/chillcenter-root-target/debug/chillcenter-connection-probe \
  /private/tmp/chillcenter-connection-probe.F7F5yV/client-independent-verifier
```

Le dossier n'existait pas avant l'exécution ; il a été créé localement en `0700`, avec fichier client `0600`.

| Assertion | Résultat |
| :--- | :--- |
| Confiance initiale exige puis vérifie le SPKI approuvé | PASS, `tls_pin=verified` |
| Connexion par le même `UiClient` Rust que Tauri | PASS, `api=2026-10-14`, 1 profil compté sans nom affiché |
| Lecture catalogue | PASS, 48 items, `has_more=true` |
| Fermeture/réouverture sur confiance locale mémorisée | PASS, `CLIENT_REOPEN_CONNECTED` |
| Exécution | exit `0`, `0.99 s`, aucun retry/skip | PASS |

Le serveur est resté PID `5145`, `NRestarts=0`, hash actif exact et sans warning après la sonde.

## Limites explicites

- Cette sonde utilise le code Rust `UiClient` réel et le service Pi réel ; ce n'est ni un mock ni une preuve de simple contrat.
- Ce n'est **pas** un lancement/clic de l'application macOS, une inspection visuelle, une lecture média, un Range NAS, un bail playback, un scan ou une modification de réglage/profil serveur.
- Aucun fournisseur TMDB/IA réel, son humain, veille/multi-écran, HDR/HEVC, RSS maximal ou nouveau benchmark deux Dune/trois flux n'a été exécuté. Les réserves historiques de performance restent ouvertes.

## Rollback et disposition

- Rollback binaire préservé : `/root/chillcenter-rollback/chill-server-r7-9a17ffe55e3d`, SHA-256 `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`, fichier root régulier `0755`.
- **Rollback trigger met** : Non. Aucun signal critique, erreur TLS/API, restart ou divergence d'artefact n'a été observé.
- **Recommandation** : ne pas rollback. Ce rapport ouvre le gate post-déploiement pour le périmètre DEV approuvé ; le propriétaire documentaire peut le consigner sans changer les artefacts compilés.
- Le master SSH utilisateur reste ouvert ; le verifier n'a effectué aucune mutation distante.
