# Revue indépendante des tests R3 — opérateur serveur 0.7.1

- **Mode** : `EXECUTION_REVIEW` ciblé (`DEV_STANDARD`)
- **Candidat exact** : `f48b351724cb9e62b03fa199599ecb4bb24638a0`
- **Source runtime inchangée** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Base historique** : `50733ecf749225449c1aac5a6064125c3a9d249c`
- **Branche** : `codex/mac-server-discovery`
- **Reviewer** : `/root/discovery_test_review`, distinct du builder ; aucun code ni test modifié
- **Environnement** : macOS 27.0.1 arm64 ; Python 3.9.6
- **Verdict** : **APPROVED** pour la source opérateur et les preuves mock/temporaire décrites ci-dessous

Le constat R2 `TEST-R2-F-001` est fermé : le mode `recover` valide désormais le dossier privé, le journal root/0600, la cible binaire, le service et les deux hashes avant toute inspection du drapeau de maintenance ou action service. La reproduction adverse R2 avec un dossier `0777` est refusée, sans arrêt de service et sans modifier la cible.

`git diff --quiet fdb80b5..f48b351 -- Cargo.toml Cargo.lock crates apps web versions.json` confirme qu'aucun runtime Rust, Mac ou web n'a changé. Les résultats R2 serveur/domaine/client restent donc ceux de la même source runtime : serveur 143 PASS/1 NFR manuel ignoré, domaine 74 PASS, client 52 PASS/1 benchmark manuel ignoré. Aucun Cargo n'a été relancé en R3.

## Preuves reproductibles

| Preuve | Sortie | Durée | Résultat |
| :--- | :---: | :---: | :--- |
| `python3 -m unittest -v scripts/test_pi_server_binary_operator.py` | 0 | 0,14 s | **14/14 PASS**, aucun skip/retry |
| Fixture indépendante temporaire : vrai `main/recover`, rollback `0777`, journal forgé valide, cible externe | 0 | incluse dans 0,07 s | **PASS** : `private_root_0700_required`, cible inchangée, aucun appel service |
| Fixture indépendante temporaire : récupération `prepared` valide, dossier 0700, journal 0600/root simulé et pins concordants | 0 | incluse dans 0,07 s | **PASS** : `recovery=PASS`, état `rollback_complete`, aucun stop/start |
| Fixture indépendante temporaire : quiescence occupée/absence de preuve liée à la génération | 0 | incluse dans 0,07 s | **PASS** : refus avant `service stop` et avant backup ; récupération appelée |
| Fixture indépendante temporaire : receipt backup distinct du snapshot validé | 0 | incluse dans 0,07 s | **PASS** : le dictionnaire sémantique différent est refusé avant tout `durable_replace`; le chemin receipt n'est pas utilisé comme oracle |
| Fixture indépendante temporaire : nonce/drapeau maintenance discordant | 0 | incluse dans 0,07 s | **PASS** : retour `False` non strict, refus strict, fichier jamais supprimé |
| Fixture indépendante temporaire : remplacement entre deux répertoires | 0 | incluse dans 0,07 s | **PASS** : octets destination exacts et deux appels `fsync` des parents |
| Lot des six fixtures ci-dessus | 0 | 0,07 s | **6/6 PASS** |
| Fixture indépendante temporaire : service du journal différent du CLI | 0 | incluse dans 0,06 s | **PASS** : `journal_cli_binding_refused` avant maintenance, activité ou service |
| Fixture indépendante temporaire : hash ancien CLI différent du journal | 0 | incluse dans 0,06 s | **PASS** : même refus avant toute action |
| Lot des deux bindings ci-dessus | 0 | 0,06 s | **2/2 PASS** |

Toutes les fixtures indépendantes ont utilisé des `TemporaryDirectory`, des fichiers factices et des mocks pour `systemctl`, l'identité root ou les chemins `/run` lorsque nécessaire. Aucun service, drapeau système, Pi, NAS ou binaire installé n'a été touché.

## Qualité des assertions

- Les 14 tests versionnés couvrent le journal atomique, les permissions du journal, les chemins confinés, les ELF refusés, le manifeste backup, les symlinks web, le couple `(receipt, snapshot validé)`, les deux fsync de remplacement, la récupération après swap ou binaire absent, le binaire inconnu et l'abandon `quiescing` sans stop.
- La reproduction adverse passe par le vrai parseur CLI et le vrai `main()`. Elle affirme à la fois le motif de refus, l'absence d'appel service et l'intégrité des octets de la cible ; un simple `raises` isolé n'aurait pas suffi.
- Le cas positif passe par `main()` et `recover()` avec un journal réellement mode 0600. Seul l'UID root est simulé, car l'hôte de revue n'exécute pas ces fixtures en root.
- Les tests de binding appellent le vrai `recover()` et prouvent que service/hash discordants sont rejetés avant `owned_maintenance`, `active` ou `service`.
- Le test de quiescence vérifie l'ordre d'appel du déploiement : ni stop ni backup avant la preuve. La récupération après cet abandon est mockée dans cette fixture, tandis que le vrai chemin `quiescing` est couvert par le test versionné qui affirme zéro action service.
- Le test backup distingue explicitement le chemin receipt du dictionnaire de snapshot ; un snapshot différent bloque avant les renames.
- Le test nonce affirme la non-suppression dans les branches stricte et non stricte. Le test fsync vérifie les appels sur les deux parents, mais ne mesure pas la persistance physique après coupure de courant.

## Fermeture R2

| Finding | État R3 | Preuve |
| :--- | :--- | :--- |
| `TEST-R2-F-001` — récupération depuis un rollback non privé | **CLOSED** | Dossier `0777` refusé par le vrai `main()` avant actions ; journal exige 0600/root ; `validate_recovery` lie binaire, service, ancien SHA et nouveau SHA aux arguments CLI ; discordances service/hash refusées avant effets. |

## Limites et gates runtime

- Exécution réelle en root sur Linux/Pi, `systemctl`, `runuser`, `ss`, backup SQLite réel, drapeau `/run`, crash/coupure et rollback disque réel : **NOT_RUN**. Les fixtures valident la logique et l'ordre, pas ces frontières OS.
- La quiescence réelle via API et pulse fichier lié à la génération : **NOT_RUN** sur un serveur déployé.
- GUI/WebView macOS, permission Local Network et annonce/navigation mDNS réelle 0.7.1 : inchangées et toujours **NOT_RUN** avant remise.
- Aucun player loop, aucune écriture Pi/NAS, aucun déploiement et aucune modification de test/source par le reviewer.

## Disposition

Le SHA `f48b351724cb9e62b03fa199599ecb4bb24638a0` ferme le bloqueur de test opérateur R2 dans la portée mock/temporaire et peut rejoindre les autres revues exactes. Une utilisation sur Pi exige encore la procédure autorisée, les contrôles root/Linux réels, la sauvegarde privée et la vérification post-déploiement ; cette revue ne les déclare pas passés.
