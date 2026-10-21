# Revue de sécurité finale — Chill Center 0.7.1

- **Mode** : revue indépendante, lecture seule, delta opérationnel `DEV_STANDARD`
- **Branche / porteur observé** : `codex/mac-server-discovery` / `3a58d16c9ccf071480d7c4a27f68a8ffa9550336`
- **Source runtime exacte** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Opérateur exact** : `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82`
- **Date de revue** : `2026-10-21T09:41:29Z`
- **Reviewer indépendant** : `/root/v071_final_security`
- **Verdict** : **APPROVED** pour passer au déploiement Pi contrôlé et réversible décrit ci-dessous

Cette approbation porte sur le runtime Bonjour/API, l'opérateur binaire et les
artefacts 0.7.1 exacts. Elle prend acte de l'autorisation humaine explicite du
21 octobre 2026 d'ajouter uniquement `AF_NETLINK`, de créer la sauvegarde privée
fraîche puis de mettre à niveau le serveur. Les documents de `3a58d16` qui disent
encore « USER_APPROVAL_REQUIRED » décrivent l'ancien point d'arrêt ; ils ne sont
pas une preuve d'exécution.

Aucun Pi, NAS, service, réglage, certificat, secret ou état distant n'a été lu ou
modifié par cette revue. Les modifications de thème/maquette non commitées et le
futur candidat Pâques/Saint-Valentin sont **hors périmètre et non approuvés**.

## Résultat consolidé

Aucun constat `CRITICAL`, `HIGH`, `MEDIUM` ou `LOW` n'est ouvert sur les trois
révisions exactes ci-dessus.

| Surface | Résultat | Preuve principale |
| --- | --- | --- |
| Bonjour/mDNS hostile | PASS | Type fixe `_chillcenter._tcp.local.`, browse 4 s avec plafond dur 5 s, 16 serveurs et 8 adresses, UUID/texte/port bornés, IP publiques/loopback/IPv6 link-local refusées. TXT limité à protocole, API, UUID, version et port. |
| Confiance TLS/identité | PASS | Une annonce reste un indice non fiable. L'autorité découverte repasse par le formulaire `connect` ordinaire ; SAN, certificat auto-signé valide, alias approuvé, SPKI et UUID renvoyé par l'API restent obligatoires. Aucun redirect ni proxy ; aucune approbation de clé par mDNS. |
| IPC/WebView | PASS | La commande native accepte uniquement `{action:"browse"}` avec champs inconnus refusés, single-flight séparé, aucune URL/commande/chemin fourni au Rust. Les noms distants utilisent `textContent`. Navigation maintenue sur les origines Tauri locales. |
| Course de démarrage | PASS source | Le socket HTTPS est lié et non bloquant avant l'annonce. Un choix manuel, serveur enregistré, connexion active ou résultat tardif invalide la génération de découverte. |
| API unique | PASS | Seul `/api/2026-10-14` est monté ; GET/POST/HEAD `/api/v1/*` donnent JSON 404. La compatibilité restante transforme localement d'anciennes références d'illustration avant émission réseau ; elle ne sert pas une seconde API. |
| Auth/session/CSRF/redirect | PASS dans le périmètre familial accepté | Aucun ajout d'authentification, session, cookie, tenant ou redirect. L'API LAN sans auth reste le risque explicitement accepté. Le middleware Host/Origin/Sec-Fetch continue d'entourer le routeur unique ; DNS rebinding, origine croisée et secrets/archives sur HTTP sont refusés. |
| Logs et secrets | PASS ciblé | L'erreur mDNS est générique. L'opérateur ne journalise ni clé, token, empreinte privée, contenu d'invariant ni stdout/stderr de sauvegarde. Le scan HEAD n'a trouvé aucun fichier secret/clé suivi ni secret à haute confiance ; l'unique motif détecté est une fixture de test synthétique. |
| Opérateur root | PASS | Candidat ELF AArch64 et anciens/nouveaux SHA-256 épinglés, URL HTTPS sans credentials/chemin/query, journal root `0600`, rollback root `0700`, chemins liés à la CLI, maintenance nonce/inode, quiescence générationnelle, sauvegarde sémantique, renames/fsync et conservation de maintenance après rollback incomplet. Aucun téléchargement ni écriture web/config/TLS/NAS. |
| Dépendances | PASS avec limite | `mdns-sd 0.21.6` est épinglé exactement, `default-features=false`; `if-addrs 0.15.0` et tout le transitif sont verrouillés registry/checksummés. `cargo audit` n'est pas installé : aucune affirmation d'audit RustSec exhaustif. |
| Packaging | PASS pour distribution privée épinglée | Linux ELF AArch64 : `8cf14421630cfd552e3af4ed340de107c0570046cc5ecb2784936ca8c552491c`. DMG : `a752700170d40b137d74597beb073f29bb4f5aebcd157c6f9ee244c9ac908bfd`, `hdiutil verify` valide. Le DMG reste ad hoc et non notarisé : ce verdict n'autorise pas une publication publique non signée. |

## Exception AF_NETLINK et rollback exact

`AF_NETLINK` est nécessaire à `getifaddrs()` sous le confinement Pi constaté.
L'ajout élargit la famille de sockets disponible au processus, mais ne donne ni
`CAP_NET_ADMIN` ni autre capability. Les contrôles `User=chillcenter`,
`NoNewPrivileges=true`, capabilities vides, NAS en lecture seule et protections
filesystem/kernel doivent rester byte/effectivement identiques.

Le changement autorisé doit être une seule drop-in détenue par l'opération, par
exemple `20-discovery-netlink.conf`, contenant exactement :

```ini
[Service]
RestrictAddressFamilies=AF_NETLINK
```

Il ne faut ni réécrire l'unité fournisseur, ni vider la liste avec une
affectation vide, ni ajouter une capability. Les affectations multiples de cette
directive étant cumulatives, le contrôle avant restart doit prouver que
l'ensemble effectif est exactement l'ancien ensemble `AF_UNIX AF_INET AF_INET6`
plus `AF_NETLINK`, sans autre delta d'unité.

L'opérateur `aba6402` protège et restaure **le binaire**, pas cette drop-in. Son
empreinte d'unité est prise après le changement autorisé et détecte ensuite toute
dérive, mais un rollback automatique du binaire peut redémarrer 0.7.0 avec
`AF_NETLINK` encore permis. Le rollback complet doit donc :

1. préserver avant écriture les octets/hashes et l'inventaire exacts de l'unité
   et de toutes les drop-ins existantes ;
2. ne supprimer que la drop-in créée par cette opération, après preuve de son
   chemin, propriétaire, mode et hash ; ne jamais toucher la drop-in existante ;
3. exécuter `daemon-reload`, puis redémarrer le binaire 0.7.0 restauré — supprimer
   le fichier sans redémarrer ne retire pas la famille du processus déjà lancé ;
4. vérifier santé HTTPS stricte 0.7.0, hash actif/installed, unité effective sans
   `AF_NETLINK`, capabilities toujours vides et toutes les empreintes protégées.

La sauvegarde privée fraîche et validée doit exister avant le premier swap ou
arrêt irréversible. Si sa validation échoue, aucun swap n'est autorisé. Si la
récupération binaire ou le retrait de la drop-in n'est pas prouvé, conserver la
maintenance et le journal non résolu ; ne pas annoncer un rollback réussi.

## Preuves reproduites

| Commande / inspection | Résultat | Limite |
| --- | --- | --- |
| `cargo test -p chill-discovery --locked --offline` | PASS 5/5 | Pas de LAN réel |
| `cargo test -p chill-server --locked --offline dated_contract_is_canonical_and_retired_alias_is_json_404 -- --nocapture` | PASS 1/1 | Test route ciblé |
| Trois tests ciblés Host/Origin/HTTP-secrets | PASS 3/3 | Routeur local |
| `node --test web/test/bridge.test.js apps/desktop/tests/native_discovery.dom.test.js` | PASS 26/26, dont découverte 7/7 | DOM simulé, pas WebKit physique |
| `node scripts/validate-api-contract.mjs` | PASS, 80 chemins | Statique/OpenAPI |
| `python3 -m unittest -v scripts/test_pi_server_binary_operator.py` | PASS 21/21 | UID/root/systemd/Pi simulés |
| `git diff --check 50733ec..3a58d16` | PASS | N'inspecte pas les changements de thème non commitées |
| `cargo tree -p chill-discovery --locked --offline` | PASS, transitif registry verrouillé | Pas de base advisory |
| scan noms + motifs secrets sur les fichiers suivis de HEAD | PASS ; fixture synthétique seule | Heuristique, pas `gitleaks` |
| `shasum -a 256` des deux artefacts | PASS, hashes documentés exacts | Artefacts ignorés par Git |
| `file` Linux + `hdiutil verify` DMG | PASS, ELF64 AArch64 et image valide | Pas de notarisation Apple |

Environnement de reproduction : Rust/Cargo `1.99.1`, Node `24.13.0`, Python
`3.9.6`, macOS arm64. Les tests ont visé les fichiers Bonjour/API/opérateur qui
n'avaient aucun delta de worktree ; les changements concurrents de thème ont été
explicitement exclus.

## Annexe — triage GitHub Advanced Security initial

Le premier scan communiqué par le coordinateur ne signale aucun secret ni alerte
Dependabot ouverte. Son analyse CodeQL était encore en cours au moment du triage ;
ce paragraphe classe les dix emplacements assainis fournis, sans prétendre avoir
interrogé ou modifié GitHub depuis cette revue.

### Correctifs source à conserver

- **Alerte 1, `py/insecure-protocol`, fixture root ligne 353 — hardening** :
  `ssl.create_default_context(cafile=...)` vérifiait déjà le certificat et les
  versions sûres par défaut sur le Python observé, vers un endpoint loopback de
  VM jetable. L'ajout explicite de `minimum_version = TLSv1_2` est correct,
  testable et supprime l'ambiguïté de runtime/analyse statique. Ne pas dismiss
  comme faux positif avant que le rerun voie le correctif.
- **Alerte 8, `js/regex-injection`, `component-release.mjs` — constat réel de
  robustesse release, fermé par le delta proposé** : la valeur CLI `component`
  atteignait un `RegExp` dynamique. L'identité de composant était refusée plus
  tard avant signature, ce qui bornait l'impact à l'outil local (déni de service
  ou audit de tags incorrect), mais l'entrée ne devait pas devenir une expression
  régulière. L'allowlist `mac|server|web|themes` avant le ledger et le prédicat à
  préfixe fixe + semver fixe ferment le chemin. Test ciblé, entrées ReDoS/newline
  incluses : **PASS 6/6**. Ne pas dismiss ; fermer par commit puis rerun CodeQL.

### Recommandation explicite de dismissal comme faux positifs

- **Alertes 2–3, ligne 82, modes `0755`** : binaires root-owned updater/serveur,
  volontairement lisibles/exécutables par le compte service non privilégié ;
  aucune écriture group/other et aucun secret dans les payloads.
- **Alerte 4, ligne 90, `server.json` `0640`** : fichier root-owned, groupe du
  service explicitement choisi pour la lecture de sa configuration ; aucun droit
  pour `other`, aucun droit d'écriture groupe.
- **Alerte 5, ligne 94, `updates.json` `0640`** : fixture synthétique root-only /
  groupe root sur VM jetable ; aucun credential, aucun droit `other` ou écriture
  groupe. Le helper root doit le lire.
- **Alerte 6, ligne 120, `/etc/chillcenter` `0750`** : répertoire root-owned,
  groupe du service uniquement pour traverser vers sa configuration `0640` ;
  aucun accès `other` et aucune écriture groupe.
- **Alerte 7, ligne 137, payload candidat `0755`** : exécutable synthétique
  root-owned que systemd doit lancer sous `chillcenter`; aucun bit d'écriture
  group/other et aucun contenu secret.

Ces six sinks n'accordent donc ni écriture non privilégiée ni lecture publique de
secret. Ils sont confinés par les gardes cumulatives du harness : variable
explicite, EUID root, hostname exact `colima-v07-security`, systemd PID 1, racines
fixes et hôte initialement vierge. Les états/keys/jetons restent `0700`/`0600` et
les configurations `0640`. **Recommandation : dismiss alertes 2–7 comme
`false positive` avec ces raisons exactes**, sans affaiblir les modes uniquement
pour satisfaire l'analyseur.

- **Alertes 9–10, `js/missing-origin-check`, `audio-worker.js:7` et
  `mpeg4-worker.js:7`** : les deux listeners vivent dans des
  `DedicatedWorkerGlobalScope`, créés exclusivement par `new Worker(...)` depuis
  la page même origine. Ce ne sont ni des listeners `window`, ni des
  `SharedWorker`, ni des service workers ; le canal appartient au seul objet
  `Worker` créateur et `MessageEvent.origin` n'est pas une frontière
  d'autorisation utile dans ce contexte. Les scripts/WASM sont résolus
  relativement à `self.location`. Une compromission same-origin aurait déjà les
  privilèges de l'application et un test d'origine dans le worker ne la réduirait
  pas. **Recommandation : dismiss alertes 9–10 comme `false positive`**, en
  conservant le caractère Dedicated Worker dans les tests/architecture.

Un rerun CodeQL doit confirmer la disparition des alertes 1 et 8. Cette annexe
n'autorise pas à dismiss une nouvelle alerte ou un emplacement différent par
analogie.

### Triage Rust du run CodeQL `39578834412`

Le coordinateur a ensuite communiqué la fin réussie des cinq analyses CodeQL
(Actions, C++, JavaScript, Python et Rust) et six alertes Rust `HIGH` nouvelles.
La présente classification a été faite sur la source distante exacte
`121cef82368724c8b5b4d8b46ff8a20d1882805b`, au moyen de
`git show <sha>:<path>` ; elle ne dépend donc pas du worktree de thème/opérateur
en mouvement. Aucun état GitHub n'a été modifié par le reviewer.

| Alerte | Sink CodeQL | Traçage indépendant | Classification |
| --- | --- | --- | --- |
| 11 | `archive_api.rs:1797`, `File::open` | Le seul fragment contrôlé est `id`, refusé avant le `join` si `Uuid::parse_str(&id)` échoue. La version verrouillée `uuid 1.28.0` n'accepte que 32 chiffres hexadécimaux, la forme UUID tiretée de 36 octets, cette forme entre accolades, ou la forme `urn:uuid:` à préfixe fixe. Aucun `/`, `\\`, composant `..` ou NUL ne peut donc atteindre `exports/{id}.zip`. Le répertoire `exports` dérive du `data_dir` service protégé. | **Faux positif ; dismissal recommandé.** |
| 13 | `export_jobs.rs:293`, `remove_file` | `cancel` exécute le même parse UUID, puis exige une correspondance exacte avec une clé de la table de jobs. Ces clés sont générées côté serveur par `Uuid::new_v4().to_string()`. Une représentation UUID alternative, quoique syntaxiquement valide, ne correspond pas à la clé canonique et ne peut atteindre le sink ; une clé correspondante ne contient aucun séparateur. | **Faux positif ; dismissal recommandé.** |
| 12 | `config.rs:312`, ouverture du stage | Le stage est `parent/.chillcenter-<UUID aléatoire>.tmp`, ouvert avec `create_new` et mode `0600`. `parent` provient uniquement de destinations internes décrites ci-dessous, sous `data_dir`; aucune route ne fournit le chemin destination à ce helper. | **Faux positif ; dismissal recommandé.** |
| 14 | `config.rs:322`, nettoyage du stage | Le nettoyage vise uniquement le même nom de stage UUID créé par le helper. Il n'utilise ni un nom client ni la valeur du fichier de configuration et ne suit pas une destination fournie par l'API. | **Faux positif ; dismissal recommandé.** |
| 15–16 | `config.rs:317`, `rename(stage, path)` | Tous les appels actifs fixent le nom destination dans le code, ou utilisent un UUID généré côté serveur : `library.json`, `imports/<UUID>.plan`, `tls/service-key.pem`, `tls/certificate.pem`, `tls/schedule.json`, `tmdb-jobs.json` et `tmdb-token`. Les racines de bibliothèque reçues sur le réseau deviennent des **octets sérialisés** dans `library.json`, jamais le chemin passé au writer. | **Faux positifs ; dismissal recommandé.** |

Pour les alertes 12 et 14–16, `atomic_private_write` est certes un helper
`pub(crate)` acceptant un `&Path`, ce qui explique la taint générique de
l'analyseur. L'inventaire exhaustif des appels à cette révision ne révèle
cependant aucun chemin client. Au démarrage, `prepare_data_dir` exige un chemin
absolu non racine sans `.`/`..`, refuse chaque composant symlink, refuse les
parents non root/service ou insécurisés, exige le propriétaire EUID final et
force `0700`. Le processus applique ensuite `umask(077)` et prend le `DataLock`
avant TLS, SQLite et les writers. `create_new(0600)`, le nom de stage UUID et le
`rename` complètent la protection. Une course menée par le même UID service
supposerait déjà la capacité de lire/modifier les données et clés du service ;
elle n'est pas offerte au client LAN non authentifié.

Pour l'alerte 11, conserver la chaîne originale après validation permet des noms
non canoniques (UUID simple, accolades ou URN) **dans** `exports`, mais ne permet
pas d'en sortir. Les exports réels sont créés sous la forme canonique côté
serveur ; une forme alternative retourne normalement 404. Canonicaliser
explicitement l'UUID avant le `join` serait un hardening de lisibilité/analyse
statique, pas le correctif d'une traversée exploitable dans cette révision. De
même, un symlink écrit dans `exports` par le compte service relèverait d'une
compromission locale de ce compte, qui possède déjà les archives ; ce scénario
ne transforme pas l'alerte LAN en `HIGH`.

Preuves ciblées reproduites sur le worktree courant, dont les corps concernés
sont identiques à `121cef8` (seules des URI de tests d'archive ont été renommées
vers l'API datée) :

- `cargo test -p chill-server --locked --offline config::tests::` : **PASS
  12/12**, dont refus des symlinks/modes dangereux, publication atomique et
  nettoyage après erreur ;
- test `archive_preview_refuses_traversal_unknown_versions_and_bad_checksums_without_database_writes` :
  **PASS 1/1** ;
- test `export_route_streams_checked_portable_zip_without_secrets_or_paths` :
  **PASS 1/1**, incluant l'ID export invalide retourné 404.

**Recommandation indépendante : dismiss 11–16 comme `false positive`, avec les
raisons propres à chaque sink ci-dessus.** Cette recommandation ne ferme pas les
alertes 1 et 8 : leurs correctifs doivent toujours être observés par un rerun
CodeQL. Elle n'autorise ni dismissal automatique futur, ni publication du futur
candidat combiné thèmes/Blob/operator avant son gel et sa revue delta.

## Gate de release restant

Cette approbation autorise l'opérateur distinct à effectuer l'opération exacte,
pas à déclarer la release vérifiée. Le gate post-déploiement exige encore : hash
du binaire actif 0.7.1, API datée et v1=404, TLS strict, web/config/TLS/data/NAS
inchangés, famille effective exacte, utilisateur/capabilities inchangés, service
stable, annonce mDNS et browse Mac réels, puis vérification indépendante. Toute
intégration de thème ultérieure crée un nouveau SHA et requiert une revue delta ;
elle n'hérite pas de ce verdict.
