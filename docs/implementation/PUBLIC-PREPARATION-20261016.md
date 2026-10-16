# Préparation GitHub et audit de confidentialité — 16 octobre 2026

## Sources et périmètre

Branche isolée `codex/public-pages-cleanup`, source finale **fd2b7a961f7fc9212c3208377cb9f4e61d306e27**, base `bab357e`. Sources, historiques privés et versions Mac figées conservés. Aucun push, changement de visibilité, déploiement Pi, changement NAS ou modification de l’autre worktree web.

Livrable principal : README, licence MIT identique à la licence existante, contribution/sécurité, formulaire de bug, code récent lisible/commenté. GitHub Pages est facultatif, avec publication manuelle désactivée par défaut et artefact limité à huit fichiers publics + manifeste. Aucun nouveau site applicatif ni service distant.

Les exemples de configuration sont loopback-only avec port `8443`. Adresses du foyer retirées de la version proposée, cibles opérateur explicites et validées. Les documents concernés portent un avertissement d’anonymisation et d’invalidité des anciennes gates pour les nouveaux scripts. Les manifestes et preuves historiques restent liés à leurs anciennes sources, pas à cette préparation.

Le lockfile du harnais Pi a été synchronisé avec la dépendance SHA-2 déjà requise par le catalogue du workspace : aucun numéro de paquet existant changé et lockfile racine inchangé. Vérification hors ligne avec `--locked` concluante après synchronisation.

## Audit, sans divulgation de valeurs

Gitleaks officiel **8.30.1**, archive macOS ARM64 vérifiée SHA-256 `b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5`, binaire `ba52fb1bfabbcde42f032afad3d6e0b19dff8ed105229a16e7caa338bbc0e84f`. Scan local, aucun code transmis ; rapports intégralement expurgés et snapshots d’objets dans un répertoire temporaire privé, hors Git.

- État initial candidat `2cb6f73` : **236 commits accessibles**, **1 708 blobs**, 64 745 806 octets ; dépôt canonique `37a5b35` : **233 commits**, **1 701 blobs**, 65 311 831 octets.
- Rescan `c4b5679` : **237 commits**, **1 777 blobs**, 65 405 301 octets. `git --all --full-history` et scan de tous les blobs avec profondeur d’archives 3. Le scan de diff Gitleaks voit 235 commits ; l’inventaire complet couvre aussi les commits sans diff utile. Le dernier delta `fd2b7a9` remplace seulement une adresse Mac restante par un nom d’exemple, relu indépendamment.
- **Aucun certificat/clé privée identifié** : aucun nom de fichier sensible, bloc PEM complet, objet DER décodable comme certificat/clé ou clé publique SSH dans les objets accessibles inventoriés. L’unique en-tête PEM du nouveau test n’a ni corps ni clé.
- Gitleaks retourne **6 alertes** sur l’historique candidat, **4** sur l’historique canonique et **6** sur chacun des snapshots complets de blobs. Code retour brut **1**, pas artificiellement déclaré zéro : chaque alerte est classée indépendamment **FALSE_POSITIVE / SYNTHETIC_TEST_FIXTURE**. Elles sont toutes dans les tests `cfg(test)` de `tmdb_worker.rs`, fixtures hexadécimales déterministes de redaction/export/file d’attente. Pas d’exclusion générale ni vérification auprès de TMDB.
- Les deux blobs concernés sont `57c7a7aed177508c22d622e0f701476d670c92e0` et `f57a5e570b4a97ebdabf53c7d20435edb3209ba8`, identiques dans les snapshots. Triages commités : `9aec051…`, `23ed466…` (copie du même blob sur une ancienne branche locale), `50bbdf8…`.

Verdict : **aucun secret réel détecté dans ce périmètre**, pas une garantie absolue. Les clés serveur et identifiants TMDB sont générés/configurés à l’exécution dans les répertoires privés, pas distribués dans Git.

## Preuves ciblées

Node `24.13.0`, Rust `1.99.0`, Mac Apple Silicon. Commandes depuis la racine :

```sh
node --test scripts/test-public-site.mjs ui/connected/filters.test.js ui/connected/filters.dom.test.js ui/connected/catalog-loading.dom.test.js ui/connected/sync.dom.test.js ui/connected/tests/catalog-query.test.js
node scripts/test-operator-address-config.cjs
cargo test --locked --offline -p chill-server config::tests --lib
cargo check --manifest-path scripts/pi-probe/Cargo.toml --locked --offline
cargo fmt --all -- --check
git diff --check
```

Résultats : **26/26 Node**, **9/9 Rust configuration**, faux SSH jamais appelé, syntaxe shell et quatre self-tests opérateur/NAS fixture PASS, compilation du harnais PASS. Cache Cargo existant utilisé, aucun accès appareil ou test vidéo. Deux régressions navigateur de l’interface connectée (filtres/ambiances) PASS. Site facultatif : trois pages × deux tons × quatre largeurs, liens/clavier/images/absence de chargements distants PASS ; rendu inspecté, pas une approbation visuelle utilisateur ni un déploiement Pages.

## Revues et défauts rencontrés

- `9274d5c` : revue code CHANGES_REQUESTED pour 404 relative ; correction `df95432`, revue code APPROVED.
- `df95432` : revue sécurité CHANGES_REQUESTED, LOW `SEC-PUBLIC-001`, lien physique de sortie ; correction `2cb6f73`, revues code et sécurité indépendantes APPROVED, test prouvant le fichier utilisateur inchangé.
- Audit des alertes Gitleaks par reviewer sécurité distinct : APPROVED_SCOPED, fixtures uniquement.
- Les reviewers configurés ont ensuite rencontré une limite de capacité du modèle ; aucune approbation inventée. Revue indépendante de remplacement du lot adresse `c4b5679` : CHANGES_REQUESTED_SCOPED, adresse d’un Mac de test restée dans un compte rendu ; correction `fd2b7a9`, verdict **APPROVED_SCOPED**. Code opérateur, validations, transmission NAS et contrôles TLS examinés ; tests locaux en lecture seule, aucun SSH réel.

## Ce qui reste avant publication

**Le dépôt complet n’est pas approuvé public.** L’historique conserve les anciennes adresses et des informations d’exploitation/chemins personnels. L’utilisateur doit choisir un état public assaini sans historique privé (en gardant le dépôt de travail privé) ou une réécriture coordonnée explicitement autorisée. Aucun effacement d’historique ou force-push n’a été effectué. Les refs distantes non récupérées, reflogs/objets inaccessibles, fichiers non suivis et audit intégral des licences/dépendances ne sont pas couverts.

Les versions figées `v0.5`/`v0.5.1` et leurs bundles ne sont pas remplacés. Les changements doivent être repris de manière ciblée dans le chantier approprié, pas fusionnés aveuglément dans une autre branche active. Voir [stratégie du dépôt](../publication/GITHUB-REPOSITORY.md).
