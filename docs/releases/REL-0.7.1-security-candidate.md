# Revue de sécurité du candidat Chill Center 0.7.1

- **Mode** : `FULL_ASSURANCE`, revue indépendante en lecture seule
- **Branche** : `codex/mac-server-discovery`
- **Source candidate exacte** : `c603f29db2936ee40f7bf66484187b4e237d9bfd`
- **Baseline runtime déjà approuvée** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Baseline opérateur binaire déjà approuvée** : `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82`
- **Reviewer indépendant** : `/root/v071_final_security`
- **Revue achevée** : `2026-10-21T10:17:12Z`
- **Verdict source** : **APPROVED_SCOPED** pour Mac Apple Silicon, serveur ARM,
  web et thèmes ; desktop Linux exclu

Le verdict approuve uniquement l'arbre source exact `c603f29…` comme entrée des
builds immuables 0.7.1 et de la préparation de l'opération Pi déjà autorisée. Il
n'est ni une preuve de build, ni une approbation d'anciens paquets, ni une preuve
de publication ou de déploiement. Aucun Pi, NAS, service, réglage, secret, release
GitHub ou fichier web déployé n'a été lu ou modifié pendant cette revue.

Le risque LAN sans authentification reste le choix produit explicite F-18. Ce
candidat n'ajoute aucune authentification, session, cookie, tenant, IAM, callback,
redirect ou frontière cloud/secrets. L'administration demeure accessible aux
participants du LAN domestique ; ce risque accepté n'est pas étendu par le delta.

## Historique immuable du gate

Le gel `1cf7b42826b58719aed16603f6e47d66d580cc2a` a reçu
**CHANGES_REQUESTED**. Ce verdict historique n'est pas requalifié :

| ID / constat sur `1cf7b42…` | Niveau au gate | Scénario et actif | Fermeture vérifiée |
| --- | --- | --- | --- |
| `SEC-CAND-001` — le lecteur desktop refusait le répertoire renderer 2 `assets/decorations`, donc un pack pourtant validé/installé retombait sur l'embarqué. | HIGH, intégrité fonctionnelle de release | Un pack signé 0.7.1 pouvait être accepté par l'installateur mais non consommé par le client ; actif : cohérence du flux de release. | `792f02b` ajoute le répertoire exact, aligne le plafond d'entrées à 133 et prouve installation + chargement externe sans fallback. |
| `SEC-CAND-002` — le validateur installateur acceptait des répertoires supplémentaires et son plafond ne comptait pas le troisième sous-répertoire. | MEDIUM | Un arbre signé malformé pouvait franchir un contrôle plus large que le consommateur ; actif : confinement du pack de données. | `792f02b` impose la whitelist exacte `assets/{logos,backgrounds,decorations}` et le plafond `MAX_ASSETS + 5`. |
| `SEC-CAND-003` — Rust acceptait `calendar`/`decoration` sur un thème statique alors que le renderer JS le refusait. | MEDIUM | Un pack signé pouvait être accepté à l'installation puis rejeté par le renderer ; actif : cohérence de validation. | `f52c630` aligne les grammaires et ajoute deux tests négatifs appariés. |
| `SEC-CAND-004` — le parent de la drop-in Bonjour pouvait être root-owned mais inscriptible par groupe/autres. | MEDIUM | Un acteur local pouvant écrire le parent pouvait influencer le chemin privilégié avant création ; actif : sandbox systemd. | `f52c630` refuse `mode & 0o022`; les cas `0775` et `0777` sont testés. |
| `SEC-CAND-005` — après succès natif puis échec total des deux fallbacks, les URL Blob/CSS précédentes restaient actives. | LOW | Après indisponibilité du pack, des ressources visuelles retirées restaient référencées ; actif : cycle de vie des objets Blob et état UI. | `c603f29` révoque les Blob au dernier échec, applique `apply(null)`, efface fond/décoration, et limite le fallback legacy aux six thèmes historiques. |

`c603f29…` descend de ces trois corrections. Aucun constat `CRITICAL`, `HIGH`,
`MEDIUM` ou `LOW` ne reste ouvert **dans le périmètre des artefacts 0.7.1
livrés**. Une alerte dépendance `MEDIUM` réelle reste visible hors périmètre dans
le graphe du desktop Linux non distribué ; voir l'annexe Dependabot.

## Surfaces de menace et résultat

| Surface | Résultat sur `c603f29…` | Éléments probants |
| --- | --- | --- |
| API, auth, session, CSRF, redirects, tenants | PASS / inchangé | L'API canonique reste `/api/2026-10-14`; `/api/v1` reste retirée. Aucun nouveau cookie, stockage navigateur, redirect, auth ou tenant. Les invariants Host/Origin/Sec-Fetch/TLS de `fdb80b5` ne changent pas. |
| Bonjour/mDNS | PASS / hérite de la revue finale précédente | Annonce non fiable traitée comme indice ; type/TXT/UUID/adresses bornés ; choix renvoyé dans le parcours TLS/identité ordinaire. Aucun secret, chemin, commande ou confiance certificat transmis par mDNS. |
| Exception systemd | PASS ciblé | Les deux modèles de l'unité serveur ajoutent seulement `AF_NETLINK` aux trois familles existantes. `chillcenter-update.service` reste limité à `AF_UNIX AF_INET AF_INET6`. Aucun capability, user, port ou permission filesystem ajouté. |
| Opérateur Bonjour root | PASS | Service, chemin et octets de drop-in sont constants ; baseline exacte des trois familles ; racine rollback `0700`, journal/snapshots `0600`, création exclusive et fsync ; parent root non symlink et non inscriptible groupe/autres ; retrait seulement de la drop-in root `0644` de hash exact. |
| Opérateur binaire | PASS / inchangé depuis `aba6402` | Aucun delta des deux fichiers de l'opérateur binaire ; 21 tests sont inclus dans le rerun combiné 31/31. L'opération reste bornée au binaire serveur avec backup et invariants config/data/TLS/web. |
| Packs thèmes téléchargés | PASS | Données déclaratives seulement, chemins PNG fermés, hashes/tailles, décodage PNG, dimensions/pixels/APNG et somme totale bornés ; arbre sans symlink ni fichier/répertoire non déclaré ; renderer 1 ancien (six thèmes) et renderer 2 (huit thèmes) explicitement distincts. |
| Pont natif / WebView | PASS | Le pont ne fournit au WebView que manifeste validé et PNG base64 validés par Rust. Les URL Blob sont créées localement, admises seulement si présentes dans la liste privée courante et révoquées après remplacement, fallback, échec partiel ou échec total. Une URL Blob fournie par le manifeste est refusée. |
| CSP | PASS ciblé | Seul `img-src` gagne `blob:`. `script-src`, `style-src` et `connect-src` ne sont pas élargis ; `object-src`, `frame-src`, `base-uri` et `form-action` restent `none`. |
| Calendrier/effets | PASS | IDs, tokens, couleurs, priorités, chemins et ressources sont fermés. Pâques/Saint-Valentin sont les seuls calendriers dynamiques ; la décoration est propre à Pâques. Le contrôle exhaustif couvre 2026–2038. Les effets restent CSS locaux, non interactifs et bornés. |
| Échec du pack | PASS | `c603f29` révoque les ressources natives et efface palette/fond/lapin avant le fallback. Easter/Valentine n'inventent pas de SVG legacy absent ; ils reviennent au logo par défaut/off. |
| Manifestes/installateur | PASS source | `versions.json` fixe composants 0.7.1, API `2026-10-14`, format 1, renderer 2 ; les quatre séquences sont 2. Le générateur lie le manifeste thème à `renderer_min = renderer_max = 2`; bootstrap et desktop annoncent renderer 2. Le prédicat de tag est une allowlist + semver fixe, sans regex issue de l'entrée. |
| Logs et données sensibles | PASS ciblé | Le nouveau code ne journalise ni octets d'unités, hashes privés, secrets ou contenu de sauvegarde. Les erreurs exposées par les opérateurs sont des raisons fixes. Aucun PII ou credential n'est ajouté. |
| Dépendances/runtime | PASS pour les artefacts livrés, dette ouverte hors périmètre | Aucun lockfile ou manifeste de dépendances n'a changé depuis `fdb80b5`. `glib 0.18.5`, vulnérable, est verrouillé mais seulement dans le graphe GTK du desktop Linux non distribué ; il est absent des graphes Mac Apple Silicon et serveur ARM. Rust/Cargo 1.99.1, Node 24.13.0 et Python 3.9.6 ont servi aux contrôles. |

### Validation renderer 2 et fichiers exacts

Le manifeste embarqué contient 8 thèmes et 18 ressources. Les trois nouvelles
ressources correspondent exactement à leurs reçus :

| Ressource | Octets | SHA-256 |
| --- | ---: | --- |
| `assets/backgrounds/easter.png` | 2 235 712 | `56ad3ec95c77fecfe85118f4fa5817315dc22ec9031a6ac27bd70d30458355ab` |
| `assets/backgrounds/valentine.png` | 2 088 010 | `2ccb697cda98dff3c56178e74fa57675f1071d860ae0307334fa7afd76f873d5` |
| `assets/decorations/rabbit-silhouette.png` | 1 362 | `e2e3cd42b25927af9392c306cad656136a50bd0016428453637e843eadba2444` |

La recherche de chaînes dans ces PNG ne révèle aucun secret, IP ou certificat ;
`itstTokens` est une chaîne de métadonnée PNG sans rapport avec un jeton
d'authentification.

## AF_NETLINK : autorisation et rollback précis

L'autorisation humaine couvre seulement la drop-in `AF_NETLINK`, une sauvegarde
privée fraîche et la mise à jour du serveur. Elle exclut NAS, réglages applicatifs,
TLS, web déployé, helper/timer et déploiement de thèmes.

La drop-in opérée contient exactement :

```ini
[Service]
RestrictAddressFamilies=AF_NETLINK
```

L'ajout permet l'énumération d'interfaces par `getifaddrs()` mais n'accorde pas
`CAP_NET_ADMIN`. Avant application, l'ensemble effectif doit être exactement
`AF_UNIX AF_INET AF_INET6`; après application, exactement ce jeu plus
`AF_NETLINK`. Une dérive, un fichier préexistant, un parent symlink/non-root ou
inscriptible par groupe/autres, ou un journal inachevé entraîne un refus.

L'ordre de retour arrière est un invariant de sécurité :

1. achever d'abord `pi-server-binary-operator.py recover` jusqu'à preuve du
   binaire 0.7.0 et du journal `rollback_complete` ;
2. seulement ensuite lancer le rollback Bonjour ;
3. celui-ci retire uniquement sa drop-in si chemin, owner, mode et hash sont
   exacts, fait `daemon-reload`, vérifie le hash de la vue `systemctl cat`
   d'origine **avant** redémarrage, redémarre puis prouve service actif et jeu de
   familles initial ;
4. toute preuve manquante conserve journal/maintenance et interdit d'annoncer un
   rollback réussi.

Supprimer la drop-in sans redémarrer ne retire pas `AF_NETLINK` du processus déjà
lancé. Inversement, récupérer le seul binaire laisserait 0.7.0 avec la famille
supplémentaire : les deux opérateurs et cet ordre sont donc nécessaires.

## Secrets, IP, dépendances et GitHub Advanced Security

Contrôles heuristiques sur les fichiers Git de `c603f29…` :

- aucun `.env`, clé, certificat privé, base, DMG ou exécutable sensible suivi ;
- les motifs de clé privée/token à haute confiance apparaissent uniquement dans
  les gardes ou fixtures négatives de `scripts/test-bilingual-docs.mjs` et
  `scripts/test-public-site.mjs` ;
- les affectations génériques détectées sont des valeurs synthétiques de tests
  (`tmdb_worker.rs`, `ui.rs`, `player-resume-guard.test.js`) ;
- l'URL avec credentials `https://u:p@host` est une fixture que
  `normalize_address` doit refuser ;
- aucun littéral IPv4 n'est ajouté au code par le delta ; la seule ligne ajoutée
  trouvée est la documentation des wildcard binds `0.0.0.0`/`[::]` ;
- aucun lockfile/manifeste de dépendance n'est modifié.

Le run CodeQL communiqué `39578834412` sur le main distant
`121cef82368724c8b5b4d8b46ff8a20d1882805b` a terminé les cinq langages avec
succès. La revue précédente a qualifié indépendamment comme faux positifs les
alertes 2–7 (modes de fixtures root), 9–10 (Dedicated Workers) et 11–16 (chemins
UUID/fixes sous data dir protégé). L'humain a ensuite autorisé le coordinateur à
clore ces 14 alertes ; cette revue n'a ni interrogé ni modifié leur état GitHub.

Les alertes 1 et 8 ne doivent pas être dismiss comme faux positifs : la source
courante conserve respectivement le minimum TLS 1.2 explicite et le prédicat de
tag sans regex dynamique. Un nouveau scan sur le SHA final/publication doit
observer leur disparition. Le premier scan communiqué ne signalait aucun secret
ni alerte Dependabot ; il ne remplace pas un scan de l'artefact final.

## Preuves reproductibles

| Outil / commande | Portée | Résultat | Limite |
| --- | --- | --- | --- |
| `git rev-parse HEAD`, `git status --short` | worktree candidat | HEAD exact `c603f29…`; les ajouts concurrents observés sont des rapports/plans Markdown non suivis et hors source revue | État local ponctuel, pas état distant |
| `git diff f52c630..c603f29 -- ...` | correction finale | 4 fichiers code/test + doc BUG ; révocation et nettoyage exactement ciblés | Inspection statique |
| `node ui/connected/theme-pack.test.js` | SHA exact | PASS | DOM/Blob simulés |
| `node --test scripts/test-component-release.mjs` | SHA exact | PASS 7/7 | Pas de publication ni clé réelle |
| `python3 -m unittest -v scripts/test_pi_bonjour_family_operator.py scripts/test_pi_server_binary_operator.py` | SHA exact | PASS 31/31 | systemd/UID/Pi simulés |
| `cargo test -p chill-updates --locked --offline theme::tests::` | SHA exact | PASS 2/2 | Sous-ensemble thème |
| `cargo test -p chillcenter-desktop --locked --offline theme_bridge::tests::` | SHA exact | PASS 4/4, dont pack renderer 2 installé | Pas de WebKit physique |
| `cargo test -p chill-server --locked --offline dated_contract_is_canonical_and_retired_alias_is_json_404 -- --nocapture` | SHA exact | PASS 1/1 | Test route ciblé |
| `node scripts/validate-api-contract.mjs` | SHA exact | PASS, 80 chemins et OpenAPI `2026-10-14` | Validation statique |
| hash/taille manifeste via Node `crypto` | SHA exact | PASS pour les 3 nouvelles ressources ; renderer 2, 8 thèmes, 18 assets | Fichiers source, pas archive finale |
| scan `git ls-files`, `git grep`, diff IPv4, `strings` PNG | SHA exact | Aucun secret/IP domestique/fichier sensible détecté ; fixtures seulement | Heuristique, pas `gitleaks` |
| `git diff --check fdb80b5..c603f29` | delta entier | Une ligne vide finale dans `REL-0.7.1-resource-final.md` | Défaut documentaire non sécurité, ne touche pas le runtime |

En complément, le coordinateur fournit la suite Rust complète antérieure : 442
PASS, 2 NFR historiques ignorés, journal
`/private/tmp/chillcenter-v071-rust-tests-final.txt` de SHA-256
`ab2b7bd36e363f3e7b10e843e03f17ff0b83ab3cb007f040a9679b05d9c4cbfe`.
Les tests ciblés ci-dessus ont été rejoués après les fixes ultérieurs et ferment
leurs deltas. Le coordinateur rapporte aussi 337 tests Node PASS et le navigateur
réel PASS pour les 8 événements, puis PASS pour le nettoyage après échec total
sur `c603f29…`. La tentative locale indépendante de rejouer ce dernier test a
échoué avant exécution (`Cannot find module 'playwright'`) : le présent rapport
ne transforme donc pas ce résultat communiqué en reproduction indépendante.

## Gate suivant et limites du verdict

1. Construire tous les paquets dans un checkout propre et immuable du **seul**
   SHA `c603f29db2936ee40f7bf66484187b4e237d9bfd`; les hashes Linux/DMG historiques
   liés à `fdb80b5` sont périmés.
2. Vérifier hashes, signatures Minisign/Tauri, contenu des archives, source_commit,
   séquences 2 et compatibilité thème `renderer_min = renderer_max = 2`. Le DMG
   ad hoc non notarisé ne doit pas être présenté comme notarisation Apple.
3. Attendre la fin de toutes les tâches du nouveau CodeQL sur le porteur
   source-équivalent et confirmer qu'aucune alerte non qualifiée ne reste ouverte.
   Les anciennes alertes 1 et 8 sont déjà auto-closes par ce scan ; voir l'annexe
   ci-dessous pour les cinq nouvelles alertes Python. Toute autre alerte
   HIGH/CRITICAL nouvelle bloque.
4. La seule opération Pi autorisée reste : drop-in AF_NETLINK contrôlée, backup
   privé validé, upgrade du binaire serveur, puis vérification et rollback dans
   l'ordre documenté. Aucun web, thème, NAS ou réglage déployé n'est inclus.
5. Le succès réel exige encore une vérification post-déploiement indépendante :
   hashes actif/installé, TLS strict, API datée et `/api/v1` 404, invariants
   config/data/TLS/web/NAS, user/capabilities/familles, stabilité, annonce mDNS et
   browse Mac réels.

Une modification ultérieure du runtime, des opérateurs, manifestes, unités,
assets ou scripts de packaging rend ce verdict `STALE`. Un commit porteur limité
à ce rapport ou à d'autres documents de gate, sans changement de l'arbre source
revu, n'ouvre pas une chaîne de revue récursive.

## Annexe — nouveau scan CodeQL du porteur `b11a480`

Mise à jour indépendante `2026-10-21T10:22:52Z`. Le porteur documentaire exact
`b11a480ddbf10c881c1f5848ed86e54bded90ecb` ne modifie aucun des trois fichiers
analysés par rapport à `c603f29…` (`git diff --quiet` : PASS). Le coordinateur
rapporte que le run `39583548692` a auto-clos, et non dismiss, les anciennes
alertes 1 (TLS) et 8 (regex). Actions, C++, JavaScript et Python sont PASS ; Rust
était encore en finition au moment de cette annexe.

Les cinq nouvelles alertes Python de permissions sont des **faux positifs**.
Recommandation indépendante : les marquer individuellement `false positive` avec
les justifications ci-dessous. Cette recommandation ne s'étend à aucun nouvel
emplacement par analogie et le reviewer n'a pas modifié GitHub.

| Alerte / sink | Qualification indépendante | Motif exact |
| --- | --- | --- |
| 17 — `pi-bonjour-family-operator.py:93`, mode passé à `os.open`/`fchmod` | **FALSE_POSITIVE** | Le paramètre n'est pas contrôlé par une entrée : ses quatre appels sont des littéraux. Les trois snapshots sous la racine root `0700` sont `0600`; la seule valeur `0644` est la drop-in systemd fixe, sans secret, devant être lisible par systemd. Elle est créée `O_EXCL`, dans un parent root non symlink et sans bits d'écriture groupe/autres, puis son owner/mode exact est revérifié. |
| 18 — `pi-server-binary-operator.py:122`, parent maintenance `0755` | **FALSE_POSITIVE** | Le chemin fixe `/run/chillcenter-update` ne contient aucun secret et doit être traversable par le service non privilégié pour observer la maintenance. Seul root peut le créer sous `/run`; s'il existe, owner root, type répertoire non symlink et mode exact `0755` sont exigés. Le drapeau et son temporaire restent `0600`, `O_EXCL`, liés atomiquement puis validés par inode/device/nonce. Connaître le nom temporaire ne donne aucun droit d'écriture et le nonce seul n'est pas une autorisation. |
| 19 — `pi-server-binary-operator.py:273`, stage ELF `0755` | **FALSE_POSITIVE** | Il s'agit du binaire serveur public/exécutable, pas d'un secret. Avant chmod, le candidat est un fichier régulier ELF64 AArch64 et son SHA-256 est épinglé ; la copie devient root:root. Le stage réside dans la racine rollback root `0700`, donc le mode de fichier ne l'expose pas pendant le staging. Après rename, le compte service doit pouvoir l'exécuter ; aucun bit d'écriture groupe/autres n'est accordé. Le hash est revérifié avant arrêt et swap. |
| 20 — `test_pi_server_binary_operator.py:51`, journal `0777` | **FALSE_POSITIVE** | Fixture négative uniquement : le test rend volontairement un faux journal world-writable et prouve que `journal_load` le refuse. Le code de production crée les journaux `0600` dans une racine root `0700`. |
| 21 — `test_pi_server_binary_operator.py:109`, parent maintenance `0777` | **FALSE_POSITIVE** | Fixture négative uniquement : le test crée volontairement un parent inscriptible et exige `maintenance_parent_invalid` avant création du drapeau. Il démontre le contrôle de production au lieu d'accorder cette permission. |

Preuve ciblée reproduite sur `b11a480`/source `c603f29` : cinq tests sélectionnés
(snapshot/drop-in Bonjour, création maintenance sous umask restrictif, swap
binaire, refus journal `0777`, refus parent `0777`) **PASS 5/5**. Les classifications
17–21 n'ajoutent aucun finding source ouvert et ne changent donc pas le verdict
**APPROVED_SCOPED** de `c603f29…`. La publication attend toutefois le résultat final de
la tâche Rust du run `39583548692` et la qualification de toute éventuelle alerte
supplémentaire.

## Annexe — Dependabot `glib` / GHSA-wrw7-89jp-8q8g

Qualification indépendante `2026-10-21`. L'alerte Dependabot 1 est une
**vulnérabilité réelle**, pas un faux positif et ne doit pas être dismiss. Le
lockfile contient `glib 0.18.5`, dans la plage affectée `>=0.15.0,<0.20.0`; la
première version corrigée est `0.20.0`. L'avis décrit un comportement indéfini
dans les implémentations `Iterator`/`DoubleEndedIterator` de `VariantStrIter`,
pouvant produire un pointeur NULL et un crash dans une build optimisée. Source
primaire : [GitHub Advisory Database — GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g).

### Applicabilité aux artefacts 0.7.1

| Artefact livré | Preuve du graphe verrouillé | Applicabilité |
| --- | --- | --- |
| Mac Apple Silicon | `cargo tree --locked --offline --target aarch64-apple-darwin -i glib` retourne « nothing to print » ; le graphe complet du package desktop ne contient ni `glib`, ni GTK, ni WebKitGTK. Tauri/Wry sélectionne ses dépendances AppKit/WebKit Objective-C sur macOS. | **Non compilé, non exploitable dans le DMG/updater Mac.** |
| Serveur ARM Linux | `cargo tree --locked --offline --target aarch64-unknown-linux-gnu -p chill-server -i glib` retourne que le package ne correspond à aucun élément du graphe ; le graphe complet serveur ne contient ni `glib`, GTK ou WebKitGTK. | **Non compilé, non exploitable dans le binaire serveur ARM.** |
| Web et thèmes | Archives de données HTML/JS/CSS/PNG/manifeste, sans binaire Rust desktop. | **Non applicable.** |
| Desktop Linux hypothétique | Les manifestes verrouillés de `tauri 2.12.3`, `tauri-runtime-wry 2.12.1` et `wry 0.57.0` activent uniquement sous `target_os=linux` leur pile GTK/WebKitGTK, contrainte sur GTK `0.18`, laquelle résout `glib 0.18.5`. | **Affecté si un desktop Linux était construit/distribué. Aucun desktop Linux 0.7.1 n'est livré.** |

Une recherche des sources du projet ne trouve aucun appel direct à
`VariantStrIter`/`Variant::array_iter_str`; ce constat de reachability ne sert pas
à minorer une build Linux, car l'unsoundness demeure dans une dépendance compilée.
La raison de non-blocage est plus forte et vérifiable : le crate n'est pas présent
dans les graphes des deux exécutables réellement livrés.

### Disposition

- **Sévérité dépôt** : `MEDIUM`, `OPEN`, hors périmètre de livraison 0.7.1.
- **Gate 0.7.1 Mac/server/web/themes** : non bloquant ; aucun correctif source
  causal n'est requis avant publication des artefacts exacts déjà revus.
- **État Dependabot** : conserver l'alerte visible ; ne pas la dismiss comme faux
  positif ou risque toléré silencieux.
- **Remédiation backlog** : avant toute annonce/build d'un desktop Linux, migrer
  la pile Tauri/GTK vers une chaîne qui résout `glib >=0.20.0`, puis vérifier le
  graphe cible, reconstruire et refaire les revues sécurité/tests. Une simple
  modification isolée de `Cargo.lock` vers 0.20 n'est pas compatible avec les
  contraintes GTK `0.18` actuelles et ne constitue pas un correctif sûr.

Cette dette ouverte ne change pas le verdict **APPROVED_SCOPED** du candidat
`c603f29…` pour les seuls artefacts Mac Apple Silicon, serveur ARM, web et thèmes.
Elle rendrait en revanche tout futur desktop Linux **CHANGES_REQUESTED** tant que
son graphe contient la version affectée.

## Revue ciblée — garde-fou dépendances CI/documentation

- **Base observée** : `b11a480ddbf10c881c1f5848ed86e54bded90ecb`
- **Candidat** : huit fichiers non commités, liés par les SHA-256 consignés
  ci-dessous ; aucun runtime, manifeste Cargo ou lockfile modifié
- **Revue** : `/root/v071_final_security`, `2026-10-21T10:38:10Z`
- **Verdict** : **APPROVED** pour intégrer ce seul lot CI/outillage/docs

Le garde complète Dependabot sans supprimer, dismiss ou requalifier l'alerte
réelle `glib`. Il analyse uniquement les graphes effectivement livrés, avec des
packages et targets littéraux, `cargo tree --locked`, sans shell, et échoue si la
résolution Cargo échoue, si une version affectée apparaît, ou si une version
`glib` rencontrée ne respecte pas la grammaire attendue. `--prefix none` et
`--format {p}` rendent le parseur strict `startsWith("glib v")` correct pour la
commande exacte revue ; les dépendances normales et de build sont incluses, les
dépendances de test exclues.

Le workflow se limite à `contents: read`, désactive la persistance des credentials,
n'utilise ni `pull_request_target`, secret, permission write, publication ou
déploiement. Les deux actions officielles sont épinglées par SHA-1 complet et les
commits existent dans leurs dépôts officiels :
`actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803` et
`actions/setup-node@249970729cb0ef3589644e2896645e5dc5ba9c38`. Dependabot
surveille Cargo et GitHub Actions chaque semaine, sans `ignore`, groupement
masquant ou auto-merge. La concurrence annule seulement un run plus ancien de la
même ref ; push main, PR, horaire et déclenchement manuel restent couverts.

La documentation FR/EN est transparente : avis `MEDIUM` réel, alerte conservée
ouverte, pas de faux correctif `glib 0.20` forcé, exclusion explicite du desktop
Linux et réévaluation obligatoire avant de le livrer. Le lien Tauri `dev` confirme
actuellement la contrainte GTK `0.18`, mais reste une référence mouvante ; la
preuve normative demeure le lockfile et les manifestes registry verrouillés.

### Preuves

| Contrôle | Résultat |
| --- | --- |
| `node --test scripts/security-shipping-dependencies.test.mjs` | PASS 4/4 : versions affectées, version inconnue, échec de commande, graphe affecté, permissions/pins/absence d'ignore |
| `node scripts/security-shipping-dependencies.mjs` | PASS : Mac `aarch64-apple-darwin/chillcenter-desktop` et serveur `aarch64-unknown-linux-gnu/chill-server` |
| Parse YAML par Ruby Psych | PASS pour les deux fichiers |
| `node --test scripts/test-bilingual-docs.mjs` | PASS 23/23 ; liens SECURITY FR/EN et garde privacy inclus |
| Vérification officielle des SHA d'actions | PASS, les deux commits épinglés existent dans `actions/checkout` et `actions/setup-node` |

Empreintes du candidat revu :

| Fichier | SHA-256 |
| --- | --- |
| `.github/dependabot.yml` | `6a082e0c22e36e7ae754cb611bc1b8cbb96924355b87480dae1cbb7e57a8bcb2` |
| `.github/workflows/dependency-security.yml` | `ca1dcc3349c9d98c02c6b03ebcfe2dd3494511d9cc68c64f78f568132e54f1a6` |
| `scripts/security-shipping-dependencies.mjs` | `753b23181012e9c0cbac4b3ab54611574b0171718292e571b2afe73e1884947a` |
| `scripts/security-shipping-dependencies.test.mjs` | `ff865cb9db785bf5ec4073a87086dfdb20d13f457688a2bc6753e3646a2195af` |
| `docs/security/DEPENDENCIES.md` | `8b60150fe2215d09cc3a9325f085bcbac0788e43eb37f39417c064ae4f67dd9a` |
| `docs/security/DEPENDENCIES.en.md` | `ba638508de813f14e7a3d29a3ff39e8c6cf5439db5bac91e1ed37675aafd05ab` |
| `SECURITY.md` | `fe5d4678451090ac1fcca3fcdd8e3f5f1143e1517c20d1997c02c43d0b2e2336` |
| `SECURITY.en.md` | `cf69bae466995cdfab7dbd86c6f79af41549666218d34e5443494907fd7f120d` |

### Recommandations non bloquantes

- `INFO-DEP-CI-001` : lors de la prochaine modification du garde, renforcer le
  test de commande pour figer aussi les deux targets et la paire
  `--prefix none`/`--format {p}`. Une sortie préfixée synthétique n'est pas
  reconnue par `assessGraph`; le candidat courant reste sûr parce que ces options
  sont présentes dans la commande exacte et immuables dans ce lot.
- `INFO-DEP-DOC-002` : remplacer à terme le lien documentaire Tauri `dev` par un
  tag/commit correspondant à `tauri 2.12.3`, afin que la preuve secondaire ne
  dérive pas. Le lockfile et le manifeste local de cette version rendent le
  constat actuel reproductible malgré ce lien mouvant.

Ces deux améliorations de maintenance n'ouvrent pas de chemin d'abus dans le
candidat exact et ne masquent aucune alerte. Toute modification de l'un des huit
fichiers ou de leurs empreintes requiert une nouvelle revue ciblée.

## Revue delta finale — candidat CI `61da27f`

- **Parent / verdict** : `b11a480ddbf10c881c1f5848ed86e54bded90ecb` /
  revue ciblée approuvée avec deux recommandations `INFO`
- **Candidat exact** : `61da27f8cd1cbadbfdfbe8b3f7540cf6fae9ccbc`
- **Revue** : `/root/v071_final_security`, `2026-10-21T10:40:26Z`
- **Verdict** : **APPROVED** pour push de ce seul delta CI/outillage/docs

Le delta contient exactement neuf fichiers CI/outillage/docs et aucun runtime,
`Cargo.toml` ou `Cargo.lock`. Il clôt les deux recommandations non bloquantes de
la revue de worktree précédente :

- `INFO-DEP-CI-001` **CLOSED** : une sortie Cargo vide, non-string ou dépourvue
  de la racine package attendue refuse désormais le gate. Les tests figent les
  deux targets exactes, `normal,build`, `--prefix none` et `--format {p}`, en plus
  de `--locked` et de l'absence de shell. Cette fermeture protège le contrat entre
  format de sortie et parseur, au lieu de seulement tester le résultat heureux.
- `INFO-DEP-DOC-002` **CLOSED** : les documents FR/EN remplacent la branche
  Tauri mouvante par le SHA amont immuable
  `0007a43ae74551655357aced98a000965b759ff3`. Le lockfile et les manifestes
  registry locaux restent la preuve normative.

`docs/security/GITHUB-SECURITY.md` distingue correctement : constats source
auto-clos par le nouveau scan, false positives qualifiés et état réellement
conservé des alertes, advisory `glib` réel toujours ouvert, premier run hébergé
du nouveau workflow encore à observer. Il n'attribue au garde ni correction de
`glib`, ni couverture exhaustive, ni exécution GitHub non encore observée. Cette
revue n'a pas interrogé ou modifié GitHub ; l'état d'alertes est l'évidence
opérationnelle fournie par le coordinateur.

### Preuves exactes

| Contrôle sur `61da27f` | Résultat |
| --- | --- |
| `git diff --name-status b11a480..61da27f` | 9 fichiers attendus seulement |
| absence de delta `Cargo.lock`, `**/Cargo.toml`, runtime/UI/web/deploy | PASS |
| `git diff --check b11a480..61da27f` | PASS |
| `node --test scripts/security-shipping-dependencies.test.mjs scripts/test-bilingual-docs.mjs` | PASS 27/27 |
| `node scripts/security-shipping-dependencies.mjs` | PASS, graphes Mac Apple Silicon et serveur ARM |
| garde / tests / workflow / suivi GitHub | SHA-256 respectifs `078958e4…8e36`, `6773e021…2108`, `ca1dcc33…f1a6`, `d96f4904…a0f6` |

Aucun constat sécurité ouvert n'est introduit par `61da27f`. L'advisory dépôt
`glib` reste volontairement `MEDIUM OPEN`, non compilé dans les artefacts 0.7.1,
et tout futur desktop Linux reste bloqué comme décrit plus haut. Un changement
postérieur de ces neuf fichiers rend cette approbation delta obsolète.
