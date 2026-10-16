# Vérification de release indépendante — CC-UX-002 / API 2026-10-14

- **Mode** : `DEV_STANDARD`, politique `INTEGRATION_AND_RELEASE`
- **Source compilée approuvée** : `9277ac4d51281b3e6d72475294b1e7822cb8e913`
- **Carrier documentaire cible** : `d25744102c9b2ff3fe69669c482bd561b2b5dff4`
- **Environnement visé** : remise locale Mac/Linux puis Pi DEV `chillcenter.local`
- **Deployer** : `/root/ux_api_delivery`
- **Verifier** : `/root/ux_api_post_verify`, distinct du deployer, builder, debugger et des trois reviewers
- **Revue UTC** : `2026-10-14T17:45:49Z`
- **Verdict canonique release complète** : **BLOCKED**
- **Sous-verdict remise locale** : **APPROVED_LOCAL**
- **Sous-verdict Pi** : **BLOCKED_PI**

Le blocage est externe et explicite : la barrière d'exécution a refusé l'interruption courte nécessaire au remplacement du service sans nouvelle confirmation humaine informée. Aucun contournement n'a été tenté. Le nouveau binaire est seulement staged ; le Pi exécute toujours R7. Le gate post-déploiement ne peut donc pas passer à `VERIFIED`.

## Validité des entrées

- Le target local est propre à `d2574410`; `9277ac4d` en est un ancêtre direct. Le delta carrier contient uniquement manifeste, contexte/ADR/statut/preuves, trois rapports et gate ; aucune source compilée, API ou entrée de déploiement n'a changé.
- Les rapports code, sécurité et tests/intégration sont trois `APPROVED` distincts liés exactement à `9277ac4d`. La revue tests est en `INTEGRATION_REVIEW` et contient des assertions significatives : 289 Rust, 47 Node, clippy strict, validation OAS et deux parcours Tauri frais sans retry/skip. Ses exclusions restent explicitement `NOT_RUN`.
- `shasum -a 256 -c MANIFEST.sha256` : **471/471 OK**, zéro échec.

## Preuves locales

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Bundle publié | `artifacts/macos/Chill Center-9277ac4.app` | présent ; diff récursif vide avec l'artefact source privé | PASS |
| Exécutable Mac | `c19a229be382597e644e0c5e6d12011ab3f7afe7c8ec4de73e0620f98a542c2c` | hash identique source et destination | PASS |
| Architecture/fermeture | 75 Mio, 48 dylibs ARM64, exécutable ARM64 | 75 Mio ; 48 ; zéro dylib non-ARM64 ; exécutable ARM64 | PASS |
| Signature | ad hoc, deep/strict | `codesign --verify --deep --strict` valide ; `Signature=adhoc`, aucun TeamIdentifier | PASS |
| Loader embarqué | libmpv bundle, API 2.5 | `BUNDLE_LIBRARY_OK .../Contents/Frameworks/libmpv.dylib API 2.5` | PASS |
| Absence d'accès AppSupport | sortie avant initialisation Tauri/config | branche `--verify-bundle` quitte avant `app_config_dir`; aucune ouverture UI/réseau/réglages | PASS |
| Ancien bundle 624 | octets signés préservés | exécutable `aa32c0958bf9b818a7f635bf1bf65e90dc9099ab24b4275d5ec7a6392733bd9d` | PASS |
| Archive Linux publiée | `a202c94eaf616132eedb7fa8213d4e0c07638ea5d9c8ba44523974c22770552a` | hash exact | PASS |
| Binaire dans l'archive | `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181` | hash exact ; build-manifest lie `9277ac4d`, Rust 1.99, Zig 0.17, glibc 2.28 | PASS |
| OpenAPI dans source/archive | `d12c606e4d922c1a52f0fe6f12f72940b1498c442dce7ba43ac9f83f946778f4` | hash exact dans les deux | PASS |

### Finding local non bloquant

`artifacts/linux/chillcenter-server-9277ac4-pi64.tar.gz.sha256` contient le bon hash, mais nomme `chillcenter-linux-9277ac4.tar.gz` au lieu du nom publié. La commande standard `shasum -a 256 -c` retourne donc exit 1. Les octets de l'archive ont été validés indépendamment contre le hash approuvé ; ce défaut de sidecar ne remet pas en cause `APPROVED_LOCAL`, mais doit être corrigé par le propriétaire documentaire/packaging, sans reconstruire ni modifier les artefacts compilés.

## Pi observé en lecture seule

Contrôle UTC `2026-10-14T17:44:30Z`, via le master SSH utilisateur existant, laissé ouvert. Aucune mutation, fixture, lecture de nom de profil, création de profil ou bail.

| Contrôle | Observé | Résultat |
| :--- | :--- | :--- |
| Plateforme | Debian 13, ARM64 | PASS ancien environnement |
| Service actuel | active/running, PID 3016, `NRestarts=0` | PASS ancien R7 |
| Binaire actif | `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6` | **OLD / non déployé** |
| Stage nouveau | `/var/tmp/chill-server-9277ac4.new` = `1eb24e97...181` | PASS staging seulement |
| Identité/confinement | UID 997, GID 983, groupe supplémentaire GID 984 ; `NoNewPrivileges`, capabilities vides, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, `PrivateDevices` | PASS ancien service |
| Chemins systemd | `ReadWritePaths=/var/lib/chillcenter`, `ReadOnlyPaths=/srv/volumes/video` | PASS ancien service |
| Montage NAS | ligne autofs `rw` superposée et ligne CIFS `ro,nosuid,nodev,noexec` | PASS borné ; **aucune affirmation de host globalement RO** |
| Configuration/état | config présente `0640` root:GID983 ; state `0700` UID997:GID983 ; DB et certificat public présents `0600` | PASS présence/droits ; contenu non dumpé |
| TLS ancien service | SAN IP `chillcenter.local`, SPKI `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df` | PASS |
| Requête stricte | certificat public récupé via SSH, `curl --cacert`, sans `-k`, `/api/v1/server` | PASS, `api_version=1` sur ancien R7 |

La clé TLS privée n'a été ni lue, ni copiée, ni affichée. La base et la configuration n'ont pas été ouvertes. L'absence de déploiement et le hash actif ancien empêchent toute revendication de préservation byte-for-byte après mise à jour : cette mise à jour n'a pas eu lieu.

## Contrôles post-déploiement non exécutés

| Contrôle requis | Statut | Motif |
| :--- | :--- | :--- |
| binaire actif `1eb24e97...181`, PID stable après remplacement | NOT_RUN | Pi non déployé |
| `/api/2026-10-14/server` et OpenAPI servie exacte | NOT_RUN | ne pas tester/annoncer l'API datée sur R7 |
| catalogue + acquisition + Range 206/32 octets avec profil QA synthétique | NOT_RUN | Pi non déployé ; aucun payload QA fourni ; mutation de profil interdite au verifier |
| logs/erreurs post-release et compatibilité runtime | NOT_RUN | aucune nouvelle release active |
| fournisseurs TMDB/IA réels | NOT_RUN | aucune clé/coût autorisé |
| nouveau benchmark 3 flux / deux Dune / Wi-Fi | NOT_RUN | exclusion explicite ; réserves historiques inchangées |

## Rollback et disposition

- **Rollback trigger met** : Non. Aucun remplacement n'a été effectué ; l'ancien serveur et l'ancien bundle restent disponibles.
- **Recommandation** : ne pas rollback et ne pas poursuivre sans autorisation. Demander la confirmation humaine explicite pour la courte interruption Pi, puis laisser le deployer distinct exécuter stop/sauvegarde/remplacement/start selon le runbook approuvé.
- Après signal `DEPLOYED`, une vérification post-déploiement indépendante doit confirmer le binaire actif, stabilité PID/restarts, confinement, TLS strict, API datée/OpenAPI, et le parcours catalogue/Range uniquement avec payload QA synthétique fourni par le deployer.
- Les réserves physiques historiques restent inchangées. Aucun passage à `VERIFIED`/`DONE` n'est autorisé par ce rapport.
