# Revue indépendante des tests R2 — API unique, découverte et opérateur 0.7.1

- **Mode** : `EXECUTION_REVIEW` (`DEV_STANDARD`)
- **Candidat exact** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Base** : `50733ecf749225449c1aac5a6064125c3a9d249c`
- **Branche** : `codex/mac-server-discovery`
- **Reviewer** : `/root/discovery_test_review`, distinct des auteurs ; aucun code ni test modifié
- **Environnement** : macOS 27.0.1 arm64 ; Rust/Cargo 1.99.1 ; Python 3.9.6
- **Verdict** : **CHANGES_REQUESTED**

Les tests Rust API/domaine/client et le pont découverte sont verts au SHA exact. L'opérateur manuel échoue toutefois une frontière de récupération root critique : le mode `recover` accepte un dossier de rollback modifiable par un tiers et fait confiance à son journal avant les validations communes. La fixture adverse a réellement remplacé la cible externe. Aucun déploiement ne doit utiliser cet opérateur avant correction et nouvelle revue exacte.

## Preuves reproductibles

| Commande / preuve | Sortie | Durée | Résultat | Note |
| :--- | :---: | :---: | :--- | :--- |
| `CARGO_TARGET_DIR=/private/tmp/chillcenter-v07-mac-tests-target cargo test -p chill-server -p chill-domain --lib --locked --offline` sous sandbox | 101 | ≈103 s | Domaine **74/74 PASS** ; serveur 142 PASS, 1 FAIL, 1 ignored | L'unique échec est antérieur à l'assertion : bind loopback refusé par le sandbox (`EPERM`) dans `silent_connections_are_dropped_and_return_their_permit`. |
| Même commande avec permission loopback | 0 | 15,94 s | Domaine **74/74 PASS** ; serveur **143/143 PASS**, 1 NFR RSS manuel ignored | Retry environnemental unique et justifié ; le test auparavant refusé passe. |
| `CARGO_TARGET_DIR=/private/tmp/chillcenter-v07-mac-tests-target cargo test -p chill-client --lib --locked --offline` | 0 | 5,43 s | **52 PASS**, 0 fail, 1 benchmark mémoire ignored | Couvre notamment HTTPS/TOFU/SPKI, chemins API canoniques, cache et anciennes références d'affiches. |
| `CARGO_TARGET_DIR=/private/tmp/chillcenter-v07-mac-tests-target cargo test -p chillcenter-desktop discovery_bridge --locked --offline` | 0 | 44,23 s | **1 PASS**, 0 fail, 37 filtrés | IPC découverte fermé à `browse`. |
| `python3 -m unittest -v scripts/test_pi_server_binary_operator.py` | 0 | 0,19 s | **10/10 PASS** | Tests unitaires fournis : journal, ELF, backup, symlink web et chemins de restauration internes. |
| Fixture adverse CLI `recover`, euid root simulé, `systemctl` mocké, rollback mode `0777`, journal valide pointant une cible externe | 3 (oracle de sécurité) | 0,70 s | **FAIL candidat** : `recovery=PASS`, `target_bytes old`, `refused False` | Aucun réseau, service ou Pi touché. La fixture sort 3 parce que l'absence de refus viole l'oracle. |

Preuves exactes remises par le coordinateur et contrôlées pour la matrice, sans nouvelle exécution dans cette instance après découverte du bloqueur : DOM découverte native **7/7** (dont clic serveur enregistré annulant la réponse tardive), web bridge/mirror/parity **45/45** avec réseau strictement canonique et booléen d'affiche historique, validateur OpenAPI **80 chemins PASS** avec schémas update typés, documentation bilingue **23/23**. Ces résultats favorables ne compensent pas la récupération root fail-open.

## Qualité des assertions

- Les tests serveur exercent le préfixe daté canonique, les anciennes routes GET/POST/HEAD en 404 JSON, les enveloppes d'erreur, Host/Origin, admission de maintenance et DTO de mise à jour. Les assertions portent sur les codes, corps et effets, pas uniquement sur la construction du routeur.
- Les 74 tests domaine et 52 tests client confirment l'absence de régression des contrats métier et de confiance. Les deux ignorés sont des bancs mémoire manuels explicitement identifiés, pas des checks fonctionnels silencieusement comptés comme passes.
- La régression découverte vérifie l'allowlist IPC. La preuve DOM R2 ajoute la course « browse en vol → clic serveur enregistré → réponse tardive » qui manquait à R1.
- Les dix tests opérateur sont utiles mais ne passent jamais par la branche CLI `main()` de `recover`. Ils valident des helpers après que les entrées ont déjà été acceptées ; ils ne contrôlent ni le mode/ownership du `rollback_dir` en récupération, ni l'égalité entre le chemin `binary` du journal et `--binary`.
- La fixture adverse appelle le vrai `main()` et le vrai `recover()` avec seulement l'identité root et les appels service simulés. Elle démontre un effet observable : un journal dans un dossier `0777` pilote le remplacement d'un fichier externe.

## Finding bloquant

### TEST-R2-F-001 — HIGH — `recover` fait confiance à un journal hors racine privée

Dans `scripts/pi-server-binary-operator.py`, `main()` exécute :

```python
if a.mode=="recover": recover(a); return
```

avant `private_dir(a.rollback_dir)`, les validations de chemins absolus et les contrôles de fichiers réguliers appliqués à `preflight/deploy`. `journal_load()` autorise par conception `binary` hors du dossier de rollback. Avec un dossier de rollback mode `0777`, une entrée pouvant écrire ce journal choisit donc une cible absolue ; l'opérateur root la remplace par `old_copy` si les hashes concordent.

Action requise, route builder opérateur :

1. valider avant toute récupération que `rollback_dir` est absolu, non symlink, root-owned et exactement `0700` ;
2. lier le `binary` du journal à la cible explicite `--binary` prévalidée, ou à une allowlist équivalente indépendante du journal ;
3. refuser journal/copies symlinkés, cible inattendue et dossier non privé avant `active()`, `service()` ou `os.replace()` ;
4. ajouter des régressions CLI pour dossier `0777`, symlink, propriétaire incorrect, cible journal différente de `--binary`, puis conserver un cas positif root-private `0700` ;
5. produire un nouveau SHA et répéter les revues code, sécurité et tests exactes.

## Limites runtime conservées

- GUI/WebView/AppKit macOS réelle, permission Local Network et accessibilité : `NOT_RUN`.
- Annonce et navigation mDNS réelles entre serveur 0.7.1 et Mac 0.7.1 : `NOT_RUN`, à vérifier après déploiement autorisé seulement.
- Opérateur sur Pi/root/systemd réel, sauvegarde réelle et rollback réel : `NOT_RUN`; le défaut présent interdit cette étape.
- Aucun player loop, aucune écriture Pi/NAS, aucun changement de service et aucun déploiement pendant cette revue.

## Disposition

Ne pas promouvoir, publier ni utiliser l'opérateur de `fdb80b502db45d10c5384104912f5f246678c32a`. Les suites API, domaine, client et découverte sont favorables ; le rejet porte exclusivement sur la frontière de récupération root démontrée. Tout correctif produit un nouveau candidat et rend ce verdict exact historique.
