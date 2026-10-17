# Documentation Chill Center

**Français** | [English](README.en.md)

Documentation de référence bilingue pour le client Mac v0.6 et l'API
2026-10-14. Les fichiers de chaque paire portent les mêmes garanties,
identifiants, limites et réserves ; les exemples ne contiennent pas de
configuration du foyer. Le contrat OpenAPI reste unique et indépendant de la
langue. Un changement documentaire n'implique ni nouveau bundle ni déploiement.

| Sujet | Français | English |
| --- | --- | --- |
| Présentation et compilation | [README](../README.md) | [README](../README.en.md) |
| Architecture actuelle, synthèse | [Architecture](architecture/OVERVIEW.md) | [Architecture](architecture/OVERVIEW.en.md) |
| Intégration API tierce | [Guide API](api/INTEGRATION-FR.md) | [API guide](api/INTEGRATION-EN.md) |
| Livraison v0.6 et limites connues | [Release](releases/REL-V0.6.md) | [Release](releases/REL-V0.6.en.md) |
| Candidat suivant : bugs 82–84 et parité web (sans IA) | [Rapport](implementation/WEB-PARITY-20261017.md) | [Report](implementation/WEB-PARITY-20261017.en.md) |
| Déploiement des seuls fichiers web sur Pi | [Livraison](releases/REL-WEB-PARITY-001-web-deployment.md) | [Delivery](releases/REL-WEB-PARITY-001-web-deployment.en.md) |
| Client web smartphone, livraison | [Livraison mobile](releases/REL-WEB-MOBILE-001.md) | [Mobile delivery](releases/REL-WEB-MOBILE-001.en.md) |
| Adaptateurs smartphone, décision | [ADR0026](architecture/decisions/ADR-0026-mobile-web-presentation.md) | [ADR0026](architecture/decisions/ADR-0026-mobile-web-presentation.en.md) |
| Contribution | [Contribuer](../CONTRIBUTING.md) | [Contributing](../CONTRIBUTING.en.md) |
| Sécurité | [Sécurité](../SECURITY.md) | [Security](../SECURITY.en.md) |
| Préparation du dépôt public | [GitHub](publication/GITHUB-REPOSITORY.md) | [GitHub](publication/GITHUB-REPOSITORY.en.md) |
| GitHub Pages facultatif | [Pages](publication/GITHUB-PAGES.md) | [Pages](publication/GITHUB-PAGES.en.md) |

## Traçabilité et maintenance

Le [contexte chronologique](PROJECT_CONTEXT.md), les
[décisions acceptées](architecture/decisions/INDEX.md), les
[bugs](debug/INDEX.md), les revues et les preuves datées restent dans leur
langue d'origine. Ils ne sont pas traduits ou réécrits rétroactivement : leurs
statuts et validations restent attachés aux sources exactes citées. Les notes
v0.6 synthétisent l'état livré sans prétendre clôturer toutes les limites.

Pour toute évolution d'un document de référence, mettre à jour les deux
langues dans le même changement. Ne traduire ni les identifiants de contrat,
ni les noms de champs, commandes, hashes ou valeurs d'exemple. Vérifier les
liens, les limites et les réserves dans les deux versions :

```sh
node --test scripts/test-bilingual-docs.mjs
git diff --check
```

Ce complément documentaire vient après le tag immuable `v0.6` ; il ne déplace
pas le tag et conserve le bundle signé de source `bac22ca`.
