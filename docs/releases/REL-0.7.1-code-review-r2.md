# Revue code indépendante R2 — découverte/API/opérateur serveur 0.7.1

- **Mode** : `DEV_STANDARD`, revue delta source exacte, non mutante
- **Base** : `50733ecf749225449c1aac5a6064125c3a9d249c`
- **Candidat revu** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Delta R2 depuis la revue précédente** : `ad64644f27a45f111ce6bc07b4f3c79f654262d5..fdb80b5`
- **Reviewer** : `/root/discovery_code_review`, distinct de l'auteur
- **Verdict** : **CHANGES_REQUESTED**

Le candidat ferme les quatre constats de la revue R1 : la socket HTTPS est liée
avant l'annonce (`crates/chill-server/src/main.rs:110-137`), un clic de serveur
enregistré invalide une navigation tardive et possède un septième test DOM
(`apps/desktop/src/native_discovery.js:23-31`,
`apps/desktop/tests/native_discovery.dom.test.js:12-17`), ADR-0030 qualifie le
helper wildcard de capacité réutilisable non atteignable par la configuration
actuelle (`docs/architecture/decisions/ADR-0030-lan-server-discovery.md:17-24`),
et ADR-0030/0031 figurent dans l'index. La connexion découverte continue de
soumettre le formulaire ordinaire ; aucun contournement TOFU/SAN/SPKI n'a été
introduit. Le retrait réseau de `/api/v1`, les appels runtime datés et la
migration de lecture des anciennes références d'affiche sont cohérents dans le
code exercé.

Les blocages ci-dessous concernent principalement le nouvel opérateur root
`scripts/pi-server-binary-operator.py`. Ils interdisent son utilisation sur le
Pi ; aucun Pi, NAS, service ou donnée réelle n'a été touché pendant cette revue.

## Constats actionnables

### CR2-F-001 — HIGH / P1 — la preuve d'inactivité ignore le protocole de quiescence du serveur

`quiet()` vérifie un hypothétique dossier `data_dir/leases` puis une photographie
de `ss` (`scripts/pi-server-binary-operator.py:95-106`). Or les baux de lecture
sont uniquement en mémoire (`crates/chill-server/src/playback.rs:73-108`) et
aucun producteur serveur n'écrit ce dossier. Entre deux heartbeats/requêtes Range,
un bail actif de 30 secondes peut donc n'avoir ni entrée disque ni socket visible.
De plus, la seconde vérification est suivie de hashes puis de l'arrêt sans fermer
l'admission (`scripts/pi-server-binary-operator.py:185-191`) : une nouvelle
lecture ou mutation peut commencer dans cette fenêtre.

Le serveur fournit déjà la frontière prévue : flag root de maintenance,
réservation des mutations avant observation, refus des nouvelles opérations,
calcul `busy()` incluant baux/scan/jobs/import/export, et pulse de quiescence
fraîche liée à la génération (`crates/chill-server/src/updates_api.rs:16-65`,
`:162-170`, `:222-240`). L'opérateur ne l'utilise pas. Cela contredit aussi
ADR-0029, qui exige d'attendre les lectures/travaux et de fermer la course avant
le swap (`docs/architecture/decisions/ADR-0029-component-releases-updates.md:127-132`).

**Impact** : l'opération dite idle peut arrêter le serveur pendant une lecture
active, une pause conservant le bail, ou une mutation admise après le dernier
poll. Le déploiement peut interrompre la lecture et violer la garantie de
préservation des travaux.

**Correction requise** : créer le flag de maintenance root, attendre une pulse
fraîche dont la génération correspond, `busy=false` et `active_mutations=0`,
revérifier cette preuve immédiatement avant l'arrêt, puis retirer le flag sur
tous les chemins succès/échec/recovery. Tester les courses acquisition/heartbeat,
requête Range entre sockets, mutation concurrente et nettoyage après interruption.

### CR2-F-002 — HIGH / P1 — `recover` agit avant de valider sa racine et ignore les pins CLI

Le mode recovery est appelé à `scripts/pi-server-binary-operator.py:169`, avant
les contrôles de chemins/racine privée/pins des lignes `170-175`. `recover()` lit
alors dans le journal le chemin `binary` et les hashes à utiliser
(`:142-160`). Il ne vérifie ni que la racine est non-symlink, root:root et `0700`,
ni que le journal est root-owned/`0600`, ni que `journal.binary == --binary`, ni
que `journal.old_sha == --old-sha`; ces arguments obligatoires sont donc ignorés
en recovery. Le journal autorise volontairement `binary` hors de sa racine
(`:28-35`).

**Impact** : une invocation root avec une racine erronée ou contrôlable par un
autre compte peut arrêter le service demandé puis déplacer/remplacer un chemin
absolu choisi par le journal avec des octets dont le hash est également déclaré
par ce journal. Même sans adversaire, un journal appartenant à une autre
transition peut restaurer le mauvais binaire malgré les pins fournis sur la ligne
de commande.

**Correction requise** : appliquer les contrôles root/`0700`, chemins absolus et
pins avant tout recovery ; exiger un journal régulier root-owned/`0600`; lier ses
champs au `--binary`, `--old-sha`, `--new-sha`/transition attendus et au service,
tout en gardant `old_copy`/`new_stage` confinés à la racine privée. Ajouter les
tests négatifs racine world-writable/symlink, propriétaire/mode journal, binaire
et hash CLI divergents, journal d'un autre service.

### CR2-F-003 — MEDIUM / P1 — le swap et le rollback ne sont pas durables face à une interruption machine

La copie candidate est fermée mais jamais `fsync` avant publication
(`scripts/pi-server-binary-operator.py:180-184`). Le swap puis le rollback
emploient `os.replace` sans synchroniser le répertoire contenant le binaire
(`:155-162`, `:191`). `atomic_json()` synchronise seulement le répertoire du
journal (`:23-27`), qui peut être différent de celui du binaire.

**Impact** : après panne de courant/crash OS, le journal durable peut annoncer
`swapped` ou `rollback_complete` alors que le nom ou les données du binaire ne le
sont pas. La récupération peut démarrer sur un fichier absent/tronqué, ou croire
un rollback terminé avant la persistance de la restauration.

**Correction requise** : synchroniser les octets/métadonnées du stage avant le
swap, puis `fsync` les deux répertoires concernés après chaque rename et avant
l'avancement durable du journal. Appliquer la même règle au rollback et ajouter
une injection de faute à chaque frontière copie/rename/fsync/journal.

### CR2-F-004 — MEDIUM / P2 — la documentation active recommande encore l'alias supprimé

ADR-0031 ne préserve comme histoire que les ADR/release reports
(`docs/architecture/decisions/ADR-0031-single-canonical-api.md:20-32`). Pourtant
les documents courants affirment encore que `/api/v1` est conservé ou le donnent
comme contrat actif : `docs/architecture/OVERVIEW.md:67-69`,
`docs/architecture/OVERVIEW.en.md:65-68`,
`docs/architecture/CONTRACTS.md:22-30`, `docs/updates/GUIDE-FR.md:19-21`,
`docs/updates/GUIDE-EN.md:19-21`, `docs/implementation/PLAYBACK-API.md:3-9`,
`docs/implementation/ARCHIVE-API.md:3-8` et
`docs/implementation/EXPORT-JOBS.md:3-7`. Ils contredisent les guides API mis à
jour et le comportement 404 vérifié.

**Impact** : opérateurs et intégrateurs suivant l'overview, le guide de mise à
jour ou les contrats d'implémentation construisent des requêtes qui échouent en
0.7.1. La documentation ne présente donc pas encore une API uniforme.

**Correction requise** : migrer les documents actifs vers `/api/2026-10-14` et
réserver les mentions v1 aux preuves/rapports explicitement historiques. Ajouter
un contrôle documentaire borné qui refuse une revendication active de rétention.

## Preuves exécutées sur le candidat exact

| Contrôle | Résultat |
| --- | --- |
| `git diff --check 50733ec..fdb80b5` | **PASS** |
| `cargo fmt --all -- --check` | **PASS** |
| `python3 -m unittest scripts/test_pi_server_binary_operator.py` | **PASS 10/10** ; aucun test de quiescence réelle, trust de recovery CLI ou durabilité inter-répertoires |
| `node scripts/validate-api-contract.mjs` | **PASS**, 80 chemins, OpenAPI `2026-10-14` |
| `node --test apps/desktop/tests/native_discovery.dom.test.js` | **PASS 7/7**, dont la nouvelle course saved-server |
| `node --test web/test/*.test.js` | **PASS 182/182**, dont aucune requête réseau legacy depuis une affiche cachée |
| `cargo test -p chill-discovery -p chill-client -p chill-server` | **NOT_RUN** : attente d'un build partagé concurrent, arrêtée sans attribuer de PASS |

Les preuves R1 restent réutilisables uniquement pour les octets découverte
inchangés : `chill-discovery` 5/5 et commande IPC ciblée 1/1 au SHA `ad64644`.
Les verdicts test/sécurité R1 sont obsolètes pour le SHA combiné R2 et ne sont pas
présentés comme ses approbations. Les tests LAN 0.7.1, autorisation Local Network,
GUI macOS, build Mac et binaire Linux ARM64 restent des gates release `NOT_RUN`.

## Verdict

**CHANGES_REQUESTED** pour
`fdb80b502db45d10c5384104912f5f246678c32a`. Les quatre constats de découverte
R1 sont résolus, mais `CR2-F-001` et `CR2-F-002` bloquent immédiatement tout usage
du nouvel opérateur privilégié ; `CR2-F-003` bloque sa qualification comme chemin
de remplacement récupérable. Aucun merge/déploiement Pi ne doit consommer cet
opérateur. Toute correction source exige un nouveau SHA et une nouvelle revue
exacte ; ce rapport reste attaché au candidat figé.
