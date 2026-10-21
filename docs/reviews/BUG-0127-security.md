# Revue de sécurité indépendante — BUG0127

- **Branche / source revue :** `codex/update-check-controls` /
  `a38d4b16616ebd6735fa4aab0b518e28d767079e`
- **Base exacte :** `6ba3970b314fb96325f906a5ccec2cde188c6ada`
- **Auteur/intégrateur :** `/root`
- **Reviewer indépendant :** `/root/v071_final_security`
- **Date :** `2026-10-21T11:30:35Z`
- **Mode :** `DEV_FAST`, correction bornée hors production dans les frontières
  déjà approuvées par F-18 et ADR-0029
- **Verdict :** **APPROVED**

Ce verdict approuve uniquement le delta source exact ci-dessus avant intégration.
Il n'autorise ni fusion automatique, publication 0.7.2, déploiement Pi/NAS,
remplacement du web installé, ni validation d'une interaction GUI native. Le
reviewer n'a modifié ni source, ni tests, ni environnement distant ; son seul
écrit est ce rapport.

## Menaces et frontières vérifiées

Les actifs concernés sont la clé publique de release embarquée, l'identité et
la compatibilité des releases affichées, le registre anti-rejeu et les intentions
du helper root, la disponibilité du serveur, et l'autorité d'installation. Les
acteurs utiles sont un appelant du LAN familial sans authentification, le service
`chillcenter` non privilégié, l'administrateur/root local, GitHub et un manifeste
ou transport hostile.

- **Entrée fermée et SSRF : PASS.** Le POST n'accepte que l'énumération
  `server|web|themes`; `deny_unknown_fields` refuse les champs supplémentaires.
  Le bridge web reconstruit le corps `{component}` et ignore notamment une URL
  fournie par l'appelant. Le client de transport ne consulte que le dépôt GitHub
  fixé, impose HTTPS, borne pages/redirections/tailles/délai et limite les
  redirections aux hôtes GitHub/CDN déjà approuvés. Les protections Host/Origin
  sans CORS restent celles de l'API datée unique.
- **Authenticité et compatibilité : PASS.** Les seuls octets distants interprétés
  sont le manifeste (64 Kio maximum) et sa signature (8 Kio maximum). La signature
  Minisign est vérifiée avant le JSON strict ; clé, composant, tag, version,
  expiration, cible, API, serveur, schéma, renderer et format sont ensuite liés.
  Le tag/version signé est encore comparé au candidat GitHub. Une signature
  invalide, une incompatibilité ou un transport défaillant ne devient jamais un
  faux `NO_UPDATE`.
- **Frontière root et rejeu : PASS.** Le chemin legacy écrit seulement dans
  `data_dir/updates/checks`, contrôlé par le compte de service et distinct des
  intentions, du cache et du registre anti-rejeu du helper. Il n'appelle aucun
  téléchargement de payload, extraction, activation ou installation. La valeur
  `highest_sequence: 0` est donc limitée à un indice d'affichage ; le helper
  privilégié reverra indépendamment séquence, signature et compatibilité. Sans
  helper, `/install` conserve son refus `UPDATE_HELPER_NOT_INSTALLED` (503).
- **Concurrence, disponibilité et persistance : PASS.** Le bitset atomique borne
  le processus à un worker par composant, soit trois au total. Le verrou d'état
  sérialise les reçus, le quota persistant regroupe cinq minutes par composant,
  l'historique est borné à 128 opérations, HTTP à 15 s, et les fichiers de résultat
  à 64 Kio. Aucun verrou d'état ou de catalogue n'est détenu pendant le réseau.
  Publication atomique et ordre « métadonnée durable puis succès » masquent tout
  résultat ancien après absence de mise à jour, erreur, incompatibilité ou crash.
  L'activité participe aussi à la preuve d'inactivité du helper.
- **UI fail-closed : PASS.** `check_available` est additif ; politique et
  installation restent liées à `helper_available`. Le bouton d'installation
  n'est exposé que pour un SemVer stable strictement supérieur, et disparaît
  pendant un contrôle ou après son échec. Sans helper, même un résultat signé ne
  rend pas le bouton exécutable. La valeur de version est écrite en `textContent`,
  pas interprétée comme HTML.
- **Secrets, données et journaux : PASS.** Le delta ne contient ni clé privée,
  jeton, IP du foyer, PII, contenu NAS ou nouvelle journalisation sensible. La
  clé ajoutée au binaire est la clé **publique** déjà suivie
  (`SHA-256 6a16e1896992dcb6e55486b8b325bcbc45a07a4ca422c0f81c664a372a05a8cc`).
  Aucun champ d'authentification, cookie, session, tenant, IAM ou cloud ne change.
- **Dépendances et compatibilité : PASS.** `semver = "1"` devient une dépendance
  directe du serveur mais réutilise exactement `semver 1.0.28` déjà verrouillé et
  déjà employé par `chill-updates`; aucun nouveau paquet tiers n'entre dans
  `Cargo.lock`. Mac/serveur/web avancent ensemble à 0.7.2/séquence 3, tandis que
  thèmes reste inchangé à 0.7.1/séquence 2.

L'administration sans authentification sur le LAN et le déni de service borné
par demandes répétées restent le choix produit explicite F-18/ADR-0029. Le delta
n'ajoute ni URL contrôlable, ni installation, ni privilège à ce risque résiduel.

## Preuves reproductibles

| Outil / version | Commande ou preuve exacte | Résultat | Portée / limite |
| :--- | :--- | :--- | :--- |
| Git 2.54 | `git rev-parse HEAD`; `git rev-parse HEAD^`; `git diff --check 6ba3970..a38d4b1` | PASS, SHAs exacts ci-dessus, delta 21 fichiers | Arbre propre avant l'ajout de ce rapport ; aucun historique distant inspecté. |
| Node 24.13.0 | `node --test ui/connected/updates.test.js web/test/bridge.test.js` | **24 PASS**, 0 échec | Comparateur SemVer, corps fermé, même origine et fallback ; pas un navigateur réel. |
| Rust/Cargo 1.99.1 | Trois `cargo test -p chill-server <nom-du-test> --locked` pour `passive_checks...`, `failure_or_no_update...`, `status_remains...` | **3 PASS**, 0 échec | État séparé, indice ancien masqué, statut lisible pendant le fetch. |
| Rust/Cargo 1.99.1 | `cargo test -p chill-server component_checks_use_cached_receipts_without_install_helper --locked` | **1 PASS**, 0 échec | Reçu coalescé sans helper et installation refusée ; transport synthétique. |
| Rust/Cargo 1.99.1 | `cargo fmt --all -- --check`; `cargo test -p chill-updates --locked` | Format PASS ; **26 PASS**, 0 échec, tests Linux root filtrés sur macOS | Vérificateur de signature, URL fixe, JSON strict, état hostile et limites héritées ; pas de root Linux réel dans cette commande. |
| Cargo 1.99.1 | `cargo tree -p chill-server -i semver --locked` | PASS ; unique `semver 1.0.28`, direct et via `chill-updates` | Prouve l'absence de nouvelle version transitive, pas l'absence de tout avis historique. |
| Scan de motifs Git | Motifs clé privée, jetons GitHub/AWS/Bearer, secrets affectés et IP privées sur les fichiers du delta ; recherche des extensions de clés | Aucun secret réel ni fichier de clé privée ; deux mentions documentaires génériques de `Bearer` seulement | Scan lexical du delta, non scan d'entropie ni historique complet. |
| Preuve fournie par le coordinateur, source exacte | `cargo test -p chill-server --lib --locked` (session 7248) ; `cargo clippy -p chill-server --all-targets --locked -- -D warnings` (session 42696) | **147 PASS**, 0 échec, 1 NFR RSS historique ignoré ; Clippy PASS | Exécution transmise, non relancée intégralement par ce reviewer. Le sandbox par défaut a séparément produit l'EPERM socket connu. |
| Preuve fournie par le coordinateur, source exacte | Suite Node web/update/release/security et DOM partagé Mac+web | **198 PASS** ; DOM PASS | DOM réel isolé, pas de clic/consentement AppKit natif. |
| QA HTTPS boucle locale, binaire `a38d4b1`, fournie par le coordinateur | Installation fictive web 0.7.0/renderer 1, helper absent, GitHub public | PASS : serveur `NO_UPDATE`; web signé 0.7.1/source `c603f29`/séquence 2 `AVAILABLE`; thèmes `CHECK_INCOMPATIBLE`; second check web `cached:true` avec le même UUID ; install 503 `UPDATE_HELPER_NOT_INSTALLED`; GET serveur/statut réactifs | Aucun Pi/NAS ; aucune release 0.7.2 publique ; seul manifeste/signature, aucun payload. Le premier lancement a correctement refusé un chevauchement de `data_dir`, puis le rerun isolé a réussi et son processus a été arrêté proprement. |

## Constats

Aucun constat Critical, High, Medium ou Low n'est ouvert sur ce delta. Aucun
secret, élargissement de privilège, autorité d'installation ou contournement de
signature/rejeu n'a été trouvé.

## Limites et prochaine porte

La QA publique ne peut pas détecter 0.7.2 avant sa publication et ne remplace pas
la vérification des artefacts immuables futurs. Aucun Pi, NAS, web domestique,
GitHub, configuration du helper ou donnée utilisateur n'a été modifié. La
construction/signature Mac et le DMG ont une preuve de packaging distincte
(bundle 0.7.2/build 11, 48 bibliothèques arm64, signature ad hoc stricte et DMG
`1d80517bdf1f5e4e079c14bc7ad6398b83de6347fc582fccf204140416b364f5` vérifiés),
mais l'interaction native réelle, l'intégration, la publication, l'installation
et la vérification post-déploiement restent des portes séparées. Toute
modification source après `a38d4b1` rend ce verdict caduc.

Rollback du correctif : restaurer les binaires clients/serveur précédents ; les
anciens binaires ignorent le répertoire optionnel `updates/checks`. Aucune donnée
NAS, préférence, identité TLS ni configuration root n'a à être restaurée.
