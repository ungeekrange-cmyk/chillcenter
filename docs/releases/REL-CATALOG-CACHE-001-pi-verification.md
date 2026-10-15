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

## Addendum — déploiement correctif et vérification finale

### Correction de gouvernance

Le coordinateur avait sélectionné la lane **DEV_FAST** pour cette livraison familiale locale, ciblée et hors production cloud. La mention `DEV_STANDARD` de l'en-tête initial est une qualification incorrecte conservée avec la preuve initiale ; elle ne décrit ni le gate effectivement utilisé ni une triple revue revendiquée. Le delta correctif a reçu les revues code et sécurité ciblées consignées par le coordinateur, puis a été livré par le script opérateur exact approuvé SHA-256 `fd162e9920b0d9c3f0ca55c97cf132cb96f8909d4f8660da64709855c7f2cedc`.

### Correctif exact et seconde bascule

- **Source corrective** : `cfc87dec21aee553d72f5016f6c9dfe600a6e1f3`.
- **Binaire attendu** : SHA-256 `f85375ad4ac33fd0993151b547e8ca731d0ffa49441beb8b85756f0433f842cd`.
- **Déploiement terminé UTC** : `2026-10-15T02:31:51Z`.
- **Vérification finale UTC** : `2026-10-15T02:33Z`–`02:34Z`.
- **Verdict final** : **APPROVED** pour ce périmètre borné, soit `APPROVED_SCOPED` dans la terminologie du projet.

Le correctif initialise paresseusement à `1`, dans la transaction existante et avec `INSERT OR IGNORE`, uniquement les lignes de révision absentes des sagas héritées. Les révisions CAS existantes, médias, membres et métadonnées ne sont pas réinitialisés. Cette conclusion de comportement provient des revues ciblées du candidat exact ; la preuve live ci-dessous démontre que le parcours auparavant bloqué fonctionne désormais.

| Contrôle delta | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Service correctif | actif, sans restart | `active/running`, PID `6808` avant/après, `NRestarts=0` | PASS |
| Artefact installé/exécuté | `f85375ad…842cd` | fichier installé et `/proc/6808/exe` exacts | PASS |
| TLS | SPKI inchangé | `3018fb4d…740df` | PASS |
| Ressources/journal après sonde | stable, aucun warning+ | RSS `14 136` → `29 652` Kio ; compteur warning+ depuis bascule `0` | PASS borné |
| Rollback serveur précédent | binaire `07358f7` conservé | `/root/chillcenter-rollback/chill-server-07358f7-11cb14459c0f1339`, SHA `11cb1445…22424` | PASS |
| Sauvegarde corrective | privée, ensemble exact | `8bbb9079-37f5-4c0f-8a8d-cbab15974a22`, `0700`, format1/schéma3, 1 710 fichiers déclarés/réels, zéro symlink | PASS |

L'opérateur a produit `PRIVATE_BACKUP_VERIFIED_EXACT` pour cette nouvelle sauvegarde, incluant hashes et SQLite immutable `quick_check`. Le vérificateur a indépendamment contrôlé la racine privée et l'égalité de l'ensemble déclaré/réel sans recalculer tous les posters. Le rollback `9277ac4` antérieur est également préservé.

Les deux sidecars de la première sauvegarde ont été **déplacés**, jamais supprimés, par l'opérateur revu vers `/var/lib/chillcenter/backup-validation-quarantine-c8897485-a1b3-4a44-8afa-8711a32ec7f7`, racine root `0700`. La preuve opérateur confirme la base et le manifeste initiaux inchangés et l'ensemble exact après quarantaine. Cette réparation récupérable clôt le finding de readiness de la première tentative ; la sauvegarde corrective exacte fournit en plus le point de retour de la seconde bascule.

### Sonde critique finale

Le même binaire `UiClient` Mac7a déjà identifié a été exécuté une seule fois contre le serveur correctif, dans un nouveau dossier local privé `/private/tmp/chillcenter-cache-pi-check.ZPteiT`.

```text
CLIENT_CACHE_OK total=2496 local_browse_ms=4426 elapsed_ms=9123 \
pinned_tls=true server_unchanged=true warm_reopen=true household_writes=false
```

- Exit `0`, durée murale `9,16 s`, sans retry ni skip.
- Confiance TLS épinglée, téléchargement cohérent de 2 496 fiches, parcours local complet, fiche, sagas et playlists : PASS.
- Vérification serveur sur la révision connue : `unchanged=true` ; réouverture du même cache : PASS.
- Dossier `0700`, 42 fichiers, tous `0600`.
- Le libellé `household_writes=false` signifie que le harnais n'a envoyé aucune commande de mutation de profil, média, collection ou réglage. La lecture initiale peut exécuter l'initialisation interne et revue des seules lignes de révision manquantes ; elle ne prouve donc pas zéro écriture SQLite interne.

Après la sonde, PID, `NRestarts`, hash exécuté et compteur warning+ sont restés inchangés. Le parcours critique qui avait motivé `CHANGES_REQUESTED` est ainsi fermé sur l'artefact correctif exact.

### Disposition finale et limites maintenues

- **Rollback trigger met après correctif** : non.
- **Recommandation finale** : ne pas rollback ; ouvrir le gate post-déploiement pour le périmètre catalogue/cache vérifié.
- Le verdict ne couvre que l'identité du service, la confiance TLS, la compatibilité API, le miroir complet/fiche/sagas/listes, l'absence de changement de révision et la réouverture chaude.
- `tmdb-jobs.json` contenait zéro tâche au contrôle initial post-redémarrage ; aucune tâche TMDB/IA n'a été relancée pendant les deux tentatives.
- Toutes les exclusions de la section **Limites** restent en vigueur. Aucun résultat lecteur, vidéo, NAS contenu, GUI, fournisseur réel ou NFR non exécuté n'est transformé en succès.
