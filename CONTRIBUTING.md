# Contribuer

**Français** | [English](CONTRIBUTING.en.md)

Lire `AGENTS.md` et le contexte du projet. Travailler sur une branche isolée ; préserver les configurations, changements et versions livrées des autres contributeurs.

Chaque correction précise le comportement attendu, la reproduction et les tests exécutés. Commenter les invariants et les raisons non évidentes, plutôt que traduire chaque ligne. Éviter les réécritures générales et les dépendances sans nécessité.

## Vérifications

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked
node --test ui/connected/filters.test.js ui/connected/filters.dom.test.js ui/connected/catalog-loading.dom.test.js ui/connected/sync.dom.test.js ui/connected/tests/catalog-query.test.js
node --test scripts/test-public-site.mjs
node --test scripts/test-bilingual-docs.mjs
node scripts/build-public-site.mjs
git diff --check
```

Commencer par les tests couvrant le changement, consigner séparément les échecs préexistants. Des données synthétiques ne prouvent ni une lecture réelle ni la capacité du Pi/NAS.

Les contrats API doivent rester compatibles avec les clients Mac et web existants. Ne pas publier une page, une release ou le dépôt sans autorisation explicite.

Aucune clé, base utilisateur, configuration réelle, clé de certificat privée ou capture personnelle dans un commit ou ticket. Fournir des exemples anonymisés. Les documents internes et l’historique Git demandent un audit avant de rendre le dépôt public ; Pages utilise sa propre sélection de fichiers.
