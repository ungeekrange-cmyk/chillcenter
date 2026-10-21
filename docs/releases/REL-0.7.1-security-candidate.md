# Revue de sécurité du candidat Chill Center 0.7.1

- **Mode** : `FULL_ASSURANCE`, revue indépendante en lecture seule
- **Branche** : `codex/mac-server-discovery`
- **Source candidate exacte** : `c603f29db2936ee40f7bf66484187b4e237d9bfd`
- **Baseline runtime déjà approuvée** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Baseline opérateur binaire déjà approuvée** : `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82`
- **Reviewer indépendant** : `/root/v071_final_security`
- **Revue achevée** : `2026-10-21T10:17:12Z`
- **Verdict source** : **APPROVED**

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
`MEDIUM` ou `LOW` ne reste ouvert sur le candidat exact.

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
| Dépendances/runtime | PASS avec limite | Aucun lockfile ou manifeste de dépendances n'a changé depuis `fdb80b5`. Rust/Cargo 1.99.1, Node 24.13.0 et Python 3.9.6 ont servi aux contrôles. Aucun audit RustSec local exhaustif n'est revendiqué. |

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
| `git rev-parse HEAD`, `git status --short` | worktree candidat | HEAD exact `c603f29…`; les autres rapports reviewers `REL-0.7.1-code-candidate.md` et `REL-0.7.1-test-candidate.md` sont untracked et hors source revue | État local, pas état distant |
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
3. Rejouer les scans secret/dépendances/CodeQL sur le candidat ou porteur
   source-equivalent exact et confirmer la disparition des alertes 1 et 8 avant
   publication publique. Toute alerte HIGH/CRITICAL nouvelle bloque.
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
