# Revue de sécurité indépendante R2 — découverte/API 0.7.1

- **Candidat figé revu** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Base de comparaison** : `50733ec`
- **Branche observée** : `codex/mac-server-discovery`
- **Auteur / reviewer indépendant** : `/root` / `/root/discovery_security_review`
- **Date** : `2026-10-20`
- **Verdict release/production** : **CHANGES_REQUESTED**
- **ADR-0030 / ADR-0031** : décisions **APPROVED** ; leur mise en production reste bloquée par l'opérateur Pi

Cette revue couvre la source exacte ci-dessus et l'exécution manuelle de
`scripts/pi-server-binary-operator.py` par un mainteneur root. Aucun Pi, NAS,
service, paquet, réglage, certificat, secret ou état cloud n'a été contacté ou
modifié. Les trois rapports non suivis déjà présents au gel sont hors candidat.

## Constats bloquants

### SEC-R2-01 — HIGH — le stop gate ne bloque ni ne détecte le travail serveur réel

`quiet()` contrôle une hypothétique arborescence `data_dir/leases` et recherche le
nom du binaire dans `ss` (`scripts/pi-server-binary-operator.py:95-106`). Or les
leases de lecture sont volatiles dans `PlaybackService` (`crates/chill-server/src/playback.rs:101-108`),
et aucun producteur de `data_dir/leases` n'existe. Une lecture en pause peut garder
une lease sans socket HTTP momentanément visible. Surtout, une nouvelle mutation
ou acquisition peut être admise entre le dernier sondage et `systemctl stop`.

Le serveur possède déjà le protocole autoritatif prévu pour ce cas : drapeau
root-owned `/run/chillcenter-update/maintenance`, refus des nouveaux travaux,
comptage des mutations et preuve `updates/quiescence.json` liée à la génération
(`crates/chill-server/src/updates_api.rs:16-65,162-170,222-240`). L'opérateur ne
l'emploie pas. Il peut donc interrompre playback, import/export, fingerprint ou
écriture active. La release doit créer durablement la maintenance, attendre une
preuve fraîche de même génération avec `busy=false` et `active_mutations=0`, puis
la nettoyer sur succès, refus et récupération. Des tests doivent couvrir une
lease sans socket, une mutation tardive et une preuve stale/mauvaise génération.

### SEC-R2-02 — HIGH — la récupération root fait confiance au journal avant ses gardes de chemin

`main()` appelle directement `recover()` à `scripts/pi-server-binary-operator.py:169`,
avant les contrôles root-owned `0700`, chemins absolus, fichiers réguliers et pins
des lignes 171-175. `journal_load()` (`:28-35`) n'impose ni propriétaire/mode du
journal, ni confinement canonique ; `binary` peut être tout chemin absolu et
`sha()` suit les liens. `recover()` (`:142-162`) peut alors arrêter le service
nommé par la CLI et déplacer un `old_copy` contrôlé vers un chemin binaire arbitraire.

Avant tout `systemctl` ou `os.replace`, recovery doit valider le rollback root
root-owned `0700`, le journal root-owned non inscriptible par groupe/autres, la
contenance canonique de `old_copy`/`new_stage`, les types sans symlink et hashes,
et lier `binary` et `service` du reçu aux valeurs explicites attendues. Ajouter des
tests adversariaux pour rollback root permissif, journal/sources symlinkés,
`binary` externe divergent et service divergent.

## Constats complémentaires

### SEC-R2-03 — MEDIUM — des octets sont écrits avant le reçu durable

Le candidat est copié vers `stage` aux lignes 182-183 puis le journal n'est créé
et fsync qu'à la ligne 184. Une coupure entre les deux laisse un stage orphelin,
sans reçu récupérable ; le prochain déploiement refuse ensuite `stage.exists()` à
la ligne 181. Écrire/fsync le reçu planifié avant la première mutation Pi, et faire
gérer à recovery un stage absent ou partiel, satisfait la contrainte de journal
durable sans transformer une coupure en intervention manuelle ambiguë.

### SEC-R2-04 — MEDIUM — la sauvegarde validée n'est pas liée à l'état pré-stop

`validate_backup()` calcule le snapshot sémantique du catalogue sauvegardé mais
jette le résultat (`scripts/pi-server-binary-operator.py:141`). Le snapshot live
`before` (`:176`) n'est donc jamais comparé à la sauvegarde créée à `:191`.
Manifest, hashes, confidentialité et cohérence interne peuvent passer sans preuve
que l'artefact restaure exactement l'état pré-swap. Retourner ce snapshot et le
comparer à `before` ; lier aussi les fichiers config/cert/key/library attendus au
backup lorsque présents. Le binaire 0.7.0 épinglé et son verrou offline réduisent
la probabilité, mais ne remplacent pas la preuve exigée de rollback.

## Contrôles sans constat ouvert

- **mDNS hostile et exposition locale** : `_chillcenter._tcp.local.` publie UUID,
  HTTPS, API, version et port uniquement. Le parser refuse UUID/API/protocole/port
  invalides, texte hors borne, loopback, IP publique et IPv6 link-local ; il borne
  5 s, 16 serveurs et 8 adresses. Une annonce LAN reste un indice usurpable/DoS,
  jamais une autorité.
- **TLS/identité** : le résultat construit seulement une autorité IP privée et
  passe par le formulaire `connect` ordinaire. SAN, validation certificat, SPKI
  approuvé et UUID renvoyé par l'API restent obligatoires ; aucun bypass TOFU,
  SAN, SPKI ou rotation de clé n'a été ajouté.
- **IPC/navigation** : `DiscoveryCommand` accepte seulement `browse` avec champs
  inconnus refusés (`apps/desktop/src/discovery_bridge.rs:14-29`) et possède un
  single-flight indépendant. Les noms distants utilisent `textContent`. La
  navigation WebView reste limitée aux deux origines Tauri locales
  (`apps/desktop/src/main.rs:786-791`) ; aucune URL, commande shell ou chemin
  arbitraire n'entre par le nouvel IPC.
- **Cycle de vie/ressources** : browse est 4 s avec hard cap 5 s ; stop/shutdown
  sont exécutés au drop. L'annonceur est désenregistré au drop, son échec reste
  non fatal. `mdns-sd 0.21.6` a ses features par défaut désactivées et utilise des
  canaux/paquets bornés dans la source registry inspectée.
- **API unique** : seul `/api/2026-10-14` est monté ; GET/POST/HEAD v1 répondent
  JSON 404. Les sources Mac/server/domain courantes émettent le préfixe daté. La
  seule compatibilité v1 restante transforme localement une ancienne référence
  d'artwork avant émission réseau (`crates/chill-client/src/lib.rs:305-321`). Le
  web de production est inchangé ; seul son test bridge change.
- **Origine/Host/auth** : aucune nouvelle session, authentification, autorisation
  ou frontière de tenant. Le LAN familial reste le périmètre sans auth approuvé.
  Le middleware same-origin/Host enveloppe encore le routeur unique ; aucun
  redirect v1 ni mutation de repli n'est ajouté.
- **État protégé** : l'opérateur ne télécharge rien, exige des SHA-256 explicites,
  vérifie ELF64/AArch64 et n'écrit pas web/config/TLS/key/settings/NAS. Les
  invariants couvrent unité systemd, web complet, config, certificat, clé,
  token TMDB et library. Le TCP HTTPS est lié avant l'annonce mDNS
  (`crates/chill-server/src/main.rs:111-137`).
- **Secrets/logs** : aucun nouvel indicateur de secret de production n'a été
  trouvé. Les erreurs mDNS sont génériques ; le script capture stdout/stderr et
  n'imprime pas les contenus sensibles ni les hashes d'invariants.

## Dépendances et provenance

`mdns-sd = 0.21.6` est épinglé exactement, `default-features = false` ;
`if-addrs = 0.15.0` est verrouillé avec checksums crates.io. L'arbre transitif
ajouté est registry-only. Licences inspectées dans les manifests locaux :
`mdns-sd` Apache-2.0 OR MIT, `if-addrs` MIT OR BSD-3-Clause,
`socket-pktinfo` MIT. La vérification officielle de version courante a été fournie
au reviewer. `cargo audit`, `cargo deny` et `gitleaks` ne sont pas installés :
aucun PASS advisory/licence exhaustif n'est revendiqué.

## Preuves reproductibles

| Commande | Résultat | Limite |
| --- | --- | --- |
| `git rev-parse HEAD` / `git diff --check 50733ec fdb80b5` | PASS ; SHA exact, diff propre | Rapports non suivis exclus |
| `CARGO_TARGET_DIR=/private/tmp/chillcenter-security-review-r2-target cargo test -p chill-discovery --locked --offline` | PASS, 5/5 | Pas d'essai LAN/macOS physique |
| `cargo test -p chill-server --locked --offline dated_contract_is_canonical_and_retired_alias_is_json_404 -- --nocapture` | PASS, 1/1 | Test route ciblé |
| `node scripts/validate-api-contract.mjs` | PASS, 80 chemins, OpenAPI datée | Validation statique/fixtures |
| `node --test web/test/bridge.test.js apps/desktop/tests/native_discovery.dom.test.js` | PASS, 26/26 | DOM simulé |
| `python3 -m unittest scripts/test_pi_server_binary_operator.py` | PASS, 10/10 | Ne couvre pas SEC-R2-01/02/03/04 ni systemd/Pi réel |
| `cargo tree -p chill-discovery --locked --offline` | PASS ; registry/checksummé, sans source Git | Pas de base RustSec locale |
| scan ciblé de clés/tokens dans le commit | PASS après exclusion des fixtures de détection | Heuristique, pas `gitleaks` |

Le test LAN réel, l'autorisation macOS Local Network, le build/bundle Mac et le
binaire ARM64 restent des preuves release séparées. La production et toute
écriture Pi restent interdites tant que les quatre constats ne sont pas corrigés,
retestés puis revus sur un nouveau SHA immuable.
