# Composants indépendants — candidat du 19 octobre 2026

**Français** | [English](COMPONENT-UPDATES-20261019.en.md)

Source figée : `22a38ff50879837c32fc8d5ac7c7200f35c8eaa1`, branche
`codex/component-updates-design`, base publique `ac85794`.
**RELEASE_BLOCKED : aucune fusion main, release 0.7 ou installation Pi.**

La conception ADR0029 R2 a reçu le « oui je valide » humain ; le
[gate](../gates/G-ADR-0029-R2.md) le consigne. Les revues de code et tests ne
peuvent pas remplacer la revue de sécurité ni les essais système manquants.

## Implémenté sur la branche, pas livré

- Version source initiale0.7.0 pour Mac, serveur, web, thèmes. Depuis le lot0.7.1,
  seule l’API datée2026-10-14 est servie ; l’ancien alias est supprimé (ADR0031).
- Packs déclaratifs dans `themes/default`, dates/priorités documentées en FR/EN,
  ressources vérifiées et repli embarqué. Aucun JS/CSS/HTML dans les thèmes.
- Carte Configuration indépendante du verrou du catalogue, versions réellement
  installées, ancien serveur supporté avec seuls contrôles indisponibles masqués.
- Vérifications GitHub du dépôt fixe, manifeste/signatures, compatibilité,
  séquences anti-rejeu ; installation Mac derrière confirmation native.
- Politiques manuelle/vérification/automatique, heure locale, défaut serveur 02:00.
- Intentions LAN bornées, helper distinct, installation par slots et simulations
  de retour arrière. **Son chemin privilégié Linux n'est pas validé pour livraison.**
- Scripts d'installation inspectables et préparation/signature locale des releases.
  La clé privée reste hors Git ; seule la clé publique est embarquée.
- Documentation API additive FR/EN et OpenAPI ; guide utilisateur FR/EN.

## Preuves locales

Commandes sur la source figée, Mac Apple Silicon / Rust 1.99.0 :

```sh
cargo test --locked --offline --workspace --all-targets -- --test-threads=1
cargo clippy --locked --offline -p chill-updates -p chill-server -p chillcenter-desktop --all-targets -- -D warnings
cargo fmt --check --all
node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js scripts/test-component-release.mjs scripts/test-bilingual-docs.mjs scripts/test-public-site.mjs
```

Résultats : suite Rust sérialisée PASS (deux tests ignorés existants), Clippy et
format PASS, **357/357 tests JavaScript/documentation PASS**. Moteur : 12 unitaires
et 1 intégration syntaxe/bootstrap-help ; desktop : 31 unitaires. La syntaxe
shell n'est pas une preuve de fonctionnement root/systemd.

QA Chromium isolée `scripts/test-component-settings.cjs` PASS : versions,
réglages CAS, formulaire conservé pendant polling, catalogue utilisable pendant
IPC lent et repli ancien serveur. Bridge synthétique, pas une preuve native Tauri
de clic sur le dialogue ni d'installation. QA des quatre ambiances déjà PASS.
Build web : **6dd3d04c9203**, 98 fichiers, non déployé.

Bundle Mac de **revue locale uniquement** construit : 0.7.0, build 9,
`Chill-Center-v0.7.0-review.app`. Signature ad hoc et vérification stricte PASS,
48 bibliothèques arm64 relocalisées, chargement du libmpv embarqué PASS. SHA-256
de l’exécutable signé : `aacf3689d8b381e5f00eb94a4edcb5f27b8c2bb2e9b91f618fe6765508ef36cc`.
Il ne remplace pas le bundle livré et n'est pas publié ni approuvé pour livraison.

Deux échecs antérieurs du test existant de timing des sous-titres sont conservés
dans [BUG0114](../debug/BUG-0114-subtitle-timing-test-instability.md). Sa réussite
isolée et celle de la suite sérialisée ne constituent pas une correction.

Scan des lignes ajoutées entre `ac85794` et la source figée : aucun en-tête de
clé privée/certificat, jeton de forme GitHub/API ou adresse privée du foyer détecté.
Ce scan ciblé ne certifie pas l'historique entier ni des formats inconnus.

## Blocage de publication et reprise

[BUG0113](../debug/BUG-0113-component-updater-release-gate.md) et la
[revue de sécurité](../reviews/component-updates-security-22a38ff.md) recensent
huit HIGH : autorisation résiduelle, cache après coupure, environnement bootstrap,
sauvegarde non bornée, préparation pendant arrêt, restrictions systemd incompatibles
avec récupération, permissions des slots et reprise après arrêt du helper.
La [revue de code](../reviews/component-updates-code-22a38ff.md) demande aussi
quatre corrections P1 de reprise/différé/état et deux P2. La
[revue de tests](../reviews/component-updates-tests-22a38ff.md) confirme les
contrôles ciblés mais demande les preuves manquantes. Les trois verdicts sont
**CHANGES_REQUESTED** sur la même source.
Leur correction suivie d'une revue indépendante et de tests d'interruption Linux
est obligatoire avant fusion/publication. Le patch de toolchain doit également
être revérifié avant les compilations de livraison.

Non exécutés : installation root/systemd, SIGKILL à chaque frontière réelle,
disque plein, Pi/NAS, mise à jour d'un vrai bundle Mac, publication puis
retéléchargement des artefacts signés. Aucun secret CI, aucune nouvelle exposition
réseau, aucun serveur de production ni NAS modifié.

Les versions livrées précédentes restent utilisables et inchangées. Aucun retour
arrière opérationnel n'est nécessaire : rien n'a été déployé. Ne pas activer le
helper de ce candidat pour contourner le gate ; poursuivre les corrections sur
une branche isolée, sans réécrire les anciens tags/rapports.
