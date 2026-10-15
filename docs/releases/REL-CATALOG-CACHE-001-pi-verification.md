# Vérification post-déploiement indépendante — REL-CATALOG-CACHE-001

- **Mode de livraison** : `DEV_STANDARD`, politique `INTEGRATION_AND_RELEASE`
- **Mode de revue des preuves** : `EXECUTION_REVIEW`
- **Source client approuvée** : `7a0f480a286d00729af3d8f04b6300c4eeade499`
- **Source serveur compilée** : `07358f73b5a6e378f1c3c0aad45cf6e4efc65aaf`
- **Carrier** : `a5aa1741dc5d55336a1e19eff9b2de99aee77f12`
- **Binaire serveur attendu** : SHA-256 `11cb14459c0f1339a9bbf6889fa4e85e997b5214a7fc12e40e651e1a42122424`
- **Déployeur** : `/root`
- **Vérificateur** : `/root/catalog_pi_verifier`, distinct du déployeur, des builders et des reviewers du lot
- **Déployé UTC** : `2026-10-15T02:06:48Z`
- **Fenêtre de vérification initiale UTC** : `2026-10-15T02:09Z`–`02:17Z`
- **Verdict initial** : **CHANGES_REQUESTED**

Le binaire prévu est actif et stable, mais le parcours critique du nouveau client ne peut pas sélectionner le profil et construire son miroir : le snapshot catalogue et la lecture des sagas renvoient `422 INVALID_OPERATION` sur les données réelles. La création partielle d'un cache local n'est pas un succès. Ce rapport ne valide ni toutes les US ni les limites physiques historiques.

## Identité et stabilité du déploiement

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Service | actif, sans restart | `active/running`, PID `6451`, `NRestarts=0` avant et après la sonde | PASS |
| Artefact installé et exécuté | SHA serveur attendu | fichier installé et `/proc/6451/exe` : `11cb14459…22424` | PASS |
| Empreinte de configuration | inchangée | `ea24f2f0…83f9`, `root:chillcenter`, `0640`, mtime antérieure au déploiement | PASS |
| Unité / drop-in cwd | inchangés | `76f7d349…2e2` / `6bad2f8c…bf1a` | PASS |
| Ressources de base | bornées après smoke | RSS `14 140` puis `26 448` Kio ; objectif serveur au repos historique ≤100 Mio, sans prétendre à une mesure longue | PASS borné |
| Journaux | aucun nouveau signal warning+ | compteur journal systemd depuis le déploiement : `0` | PASS borné |
| NAS | montage vidéo inchangé et RO | ligne CIFS `ro,nosuid,nodev,noexec`; couche autofs séparée `rw` | PASS borné |

Les traces, alarmes et budget d'erreur ne sont pas configurés pour ce service familial local ; ils sont `NOT_AVAILABLE`, pas déclarés passés.

## TLS, API et compatibilité

Toutes les requêtes ont utilisé le certificat public du Pi avec vérification stricte, TLS ≥1.2 et jamais `-k`.

| Contrôle | Observé | Résultat |
| :--- | :--- | :--- |
| Identité TLS | SPKI `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df`, SAN IP `192.168.1.42`, validité `2026-10-14`–`2028-04-14` | PASS |
| API datée | `GET /api/2026-10-14/server` → `200`, version `2026-10-14`, version déclarée supportée | PASS |
| Alias compatible | `GET /api/v1/server` → `200`, `api_version=1` | PASS |
| OpenAPI servi | SHA-256 `01423e6a3ace79453671a59cb67674db2c575b27bd295a1c2a83965906d9acae` | PASS |
| Profils / bibliothèque / préférences | `200` sur le premier profil existant, sans afficher identifiant, nom ni contenu | PASS |
| Snapshot catalogue | `POST .../catalog/snapshot` → `422 INVALID_OPERATION` | **FAIL** |
| Collections | playlists `200`, sagas `422 INVALID_OPERATION` | **FAIL** |

Le contrôle ciblé en lecture seule du coordinateur a ensuite confirmé l'incompatibilité : 37 sagas réelles, dont 30 sans ligne `collection_revisions`; zéro membre sans média et zéro membre sans identité stable. `Store::sagas()` exige la révision absente et le snapshot appelle cette méthode. La correction appartient au builder/debugger, pas au vérificateur.

## Sonde réelle `UiClient`

- Source inspectée : `/private/tmp/chillcenter-cache-release.AXF2T8/probe/src/main.rs`, SHA-256 `3771dcb19bf3964d10ea1523d48d00b8dffbaeaa4a32937a6aa4f27c57caeb27`.
- Manifeste : SHA-256 `00c01428a2489bc34cd81d6e55225742810f54908f36010816d4510d3041471c` ; dépendance directe au `chill-client` du checkout carrier.
- Binaire exécuté : `/private/tmp/chillcenter-root-target/debug/chillcenter-cache-connection-probe`, SHA-256 `ea60872bc3aa24ec417034ea6283c12c599b0b4f599d2685fe3fd00c0f2f093f`.
- Client privé neuf : `/private/tmp/chillcenter-cache-pi-check.FZw8IO`, dossier `0700`, fichiers `0600`.

Commande critique :

```text
chillcenter-cache-connection-probe \
  https://192.168.1.42:8443 \
  /private/tmp/chillcenter-cache-pi-check.FZw8IO \
  3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df
```

| Exécution | Durée | Résultat | Assertion significative |
| :--- | :--- | :--- | :--- |
| Sonde initiale | `4,34 s` | exit non nul, panic contrôlé sur `422 INVALID_OPERATION` | `CLIENT_CACHE_OK` absent : FAIL |
| Diagnostic, même dossier et même parcours | environ `35 s` | même `422`; backtrace `main.rs:17:168` | échec localisé dans `UiCommand::SelectProfile`; reproductible, pas flaky |

La sonde a établi la confiance TLS et créé une base de cache locale, mais n'a pas atteint les assertions catalogue complet, fiche, sagas, playlists, révision inchangée et réouverture chaude. Ces assertions sont **FAIL/NOT_REACHED**, jamais `PASS`. Aucun profil, réglage, scan, tâche fournisseur, bail ou média du foyer n'a été créé ou modifié.

## Sauvegarde et rollback

- Rollback binaire présent : `/root/chillcenter-rollback/chill-server-9277ac4-1eb24e97bcdf8f66`, SHA-256 `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`, root:root `0755`.
- Sauvegarde privée : `/var/lib/chillcenter/data/backups/c8897485-a1b3-4a44-8afa-8711a32ec7f7`, racine `0700`.
- Manifeste format `1`, schéma `3`, racine de données attendue, 1 709 entrées uniques, catalogue et configuration déclarés, tailles déclarées exactes, aucun lien symbolique.
- Le hash du catalogue correspond au manifeste et `PRAGMA quick_check` en Python SQLite `mode=ro&immutable=1`, `query_only=ON` retourne `ok`. Le binaire `sqlite3` autonome est absent du Pi ; son indisponibilité n'est pas un échec applicatif.
- Au contrôle initial, deux fichiers SQLite non déclarés étaient présents : `catalog.sqlite-wal` (0 octet) et `catalog.sqlite-shm` (32 768 octets). Le validateur de restauration compare l'ensemble exact des fichiers et refuserait cette racine en l'état. Leur origine n'est pas attribuée par ce rapport ; le vérificateur ne les a ni modifiés ni supprimés.
- État fournisseur post-redémarrage limité aux compteurs autorisés : `tmdb-jobs.json` contient zéro tâche ; aucune relance automatique ou manuelle n'a été effectuée.

## Disposition initiale

- **Rollback trigger met** : oui pour l'acceptation de cette release, car le parcours critique catalogue/cache échoue sur le service réellement déployé.
- **Recommandation** : ne pas remettre le nouveau client tant que la compatibilité des sagas n'est pas corrigée, redéployée et revalidée. Si une correction rapide et revue n'est pas promue, le déployeur doit appliquer la procédure de rollback autorisée ; le vérificateur ne mute pas l'environnement.
- La présence initiale des deux sidecars doit être corrigée de façon récupérable par le déployeur et revérifiée avant de considérer la sauvegarde comme prête à restaurer.
- Le service stable et l'alias v1 permettent au déployeur de choisir une correction ciblée plutôt qu'un rollback immédiat, mais ne changent pas le verdict **CHANGES_REQUESTED**.

## Limites

Aucun player/GUI, vidéo, Range NAS, bail, lecture, son, HDR/HEVC, veille, multi-écran, benchmark Dune/trois flux, scan, TMDB ou IA n'a été lancé, conformément à la restriction utilisateur. Aucun secret, clé privée, configuration en clair, titre, identifiant de profil ou contenu du catalogue n'a été affiché. La validation visuelle humaine et les NFR matériels historiques restent `NOT_RUN` ici.
