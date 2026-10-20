# Revue indépendante des tests R4 — opérateur serveur 0.7.1

- **Mode** : `EXECUTION_REVIEW` ciblé (`DEV_STANDARD`)
- **Candidat exact** : `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82`
- **Source runtime Rust/Mac/web inchangée** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Prédécesseur opérateur** : `f48b351724cb9e62b03fa199599ecb4bb24638a0`
- **Reviewer** : `/root/discovery_test_review`, distinct du builder ; aucun code ni test modifié
- **Environnement** : macOS 27.0.1 arm64 ; Python 3.9.6
- **Verdict** : **APPROVED** pour le gate opérateur pré-Pi, dans la portée fichiers temporaires réels + frontières root/service mockées

Le delta R4 ferme les lacunes de preuve de R3 autour du chemin `/run`, de l'umask, de la propriété du drapeau de maintenance, de la quiescence liée à une génération et des sorties succès/échec du vrai `main()`. Les 21 tests versionnés et deux probes adverses indépendantes passent au SHA exact.

`git diff --quiet fdb80b5..aba6402 -- Cargo.toml Cargo.lock crates apps web versions.json` retourne 0 : aucun runtime Rust, Mac ou web n'a changé. Les preuves R2 sur cette source runtime restent applicables (serveur 143 PASS/1 NFR manuel ignoré, domaine 74 PASS, client 52 PASS/1 benchmark manuel ignoré). Aucun Cargo n'a été relancé en R4.

## Preuves reproductibles

| Commande / preuve | Sortie | Durée | Résultat |
| :--- | :---: | :---: | :--- |
| `python3 -m unittest -v scripts/test_pi_server_binary_operator.py` | 0 | 0,49 s (unittest 0,20 s) | **21/21 PASS**, 0 fail/skip/retry |
| Probe adverse temporaire : drapeau maintenance étranger préexistant puis `acquire_maintenance` avec un autre nonce | 0 | incluse dans 0,005 s | **PASS** : `preexisting_maintenance_refused`, octets étrangers inchangés, fichier temporaire supprimé |
| Probe adverse temporaire : API prête mais preuve fichier avec mauvaise génération | 0 | incluse dans 0,005 s | **PASS** : `generation_bound_quiescence_unproven` malgré `busy=false`, `active_mutations=0`, `quiescing=true` côté API |
| Lot des deux probes adverses | 0 | 0,005 s | **2/2 PASS** |

Les tests n'ont écrit que dans des `TemporaryDirectory`. `MAINTENANCE_PATH` a été remplacé par le seam prévu ; aucune écriture sous `/run`, aucun appel systemd/runuser/ss réel, aucun Pi/NAS et aucun binaire installé n'ont été touchés.

## Inspection des assertions

### Frontière filesystem et maintenance

- `root_filesystem()` ne remplace que l'UID observé et `chown`; les fichiers, modes, liens physiques, renames et fsync sont réels dans le dossier temporaire.
- Sous `umask 077`, le parent nouvellement créé finit exactement en `0755`, le drapeau en `0600`, le nonce publié est relu via `owned_maintenance`, puis seul l'inode possédé est supprimé.
- Un parent préexistant `0777` est refusé avant publication. Les contrôles source exigent en plus UID root, répertoire réel et absence de symlink.
- Un nonce étranger échoue dans `owned_maintenance` et `release_maintenance`; le fichier demeure. La probe indépendante étend ce contrôle au cas d'un drapeau étranger déjà présent avant l'acquisition et vérifie aussi l'absence de fuite du fichier temporaire.

### Quiescence et ordre des effets

- Le test positif écrit une preuve réelle contenant le `st_mtime_ns` du drapeau publié et exige simultanément le même état non occupé dans l'API et le fichier.
- La probe négative fournit une API entièrement prête mais une génération fichier différente : l'opérateur refuse. Un statut API seul ne peut donc pas autoriser le stop.
- Le chemin `quiescing` de récupération retire le drapeau correspondant au nonce du journal sans appeler stop/start lorsque l'ancien service est sain.

### Déploiement et récupération

- Le test vrai `main()` succès exerce journal, staging, copie binaire, chmod, fsync, maintenance, états `quiescing/stopping/swapped/complete`, stop/start mockés, remplacement réel du binaire et retrait final du drapeau possédé. Il affirme les octets finaux et l'état du journal, pas seulement l'absence d'exception.
- Le test d'échec force un backup refusé puis une récupération elle-même en échec : le drapeau reste publié et le journal reste `stopping`, empêchant un faux succès ou la réouverture silencieuse de l'admission.
- Les tests historiques conservent les refus de dossier/journal non privés, les bindings binaire/service/hashes, le binaire inconnu, les copies rollback confinées, les manifestes backup et les symlinks web.
- `durable_replace` exerce de vrais renames entre parents distincts et deux appels fsync. Le staging R4 fsync désormais le fichier après `chown/chmod`, puis son dossier parent.

## Findings

Aucun finding de test ouvert au SHA `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82` dans cette portée. Les constats R2/R3 ciblés sont couverts par les refus et parcours ci-dessus.

## Limites avant swap réel

- L'UID root est simulé ; aucune exécution réelle en root Linux/Pi n'est revendiquée.
- `systemctl`, `runuser`, `ss`, TLS réel, API réelle, backup SQLite réel, disponibilité du service après restart et rollback de processus réel sont **NOT_RUN** ici.
- Les appels `fsync` sont effectués sur le filesystem temporaire, mais aucune coupure d'alimentation/crash noyau n'est simulée.
- La fraîcheur positive utilise l'horloge locale temporaire ; latence et propagation du vrai pulse serveur restent à vérifier lors de l'opération autorisée.
- GUI/WebView macOS, permission Local Network et annonce/navigation mDNS réelle 0.7.1 restent des gates de validation séparés.
- Aucun player loop, aucune écriture Pi/NAS, aucun déploiement et aucune modification source/test par le reviewer.

## Disposition

Le SHA exact `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82` satisfait le gate de tests opérateur pré-Pi et peut rejoindre les revues code/sécurité R4. L'approbation n'est pas une preuve de déploiement : le backup privé, le swap binaire réel, le rollback et les vérifications LAN/post-déploiement doivent encore être exécutés et consignés sur le Pi dans la procédure autorisée.
