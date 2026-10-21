# Documentation Chill Center

**Français** | [English](README.en.md)

Documentation de référence bilingue pour les composants 0.7.0, la v0.6 et l'API
2026-10-14. Les fichiers de chaque paire portent les mêmes garanties,
identifiants, limites et réserves ; les exemples ne contiennent pas de
configuration du foyer. Le contrat OpenAPI reste unique et indépendant de la
langue. Un changement documentaire n'implique ni nouveau bundle ni déploiement.

| Sujet | Français | English |
| --- | --- | --- |
| Présentation et compilation | [README](../README.md) | [README](../README.en.md) |
| Architecture actuelle, synthèse | [Architecture](architecture/OVERVIEW.md) | [Architecture](architecture/OVERVIEW.en.md) |
| Intégration API tierce | [Guide API](api/INTEGRATION-FR.md) | [API guide](api/INTEGRATION-EN.md) |
| Versions et mises à jour indépendantes | [Guide](updates/GUIDE-FR.md) | [Guide](updates/GUIDE-EN.md) |
| Maintenance hebdomadaire des paquets Linux (distincte de Chill Center) | [Procédure FR/EN](operations/OS-PACKAGE-MAINTENANCE.md) | [FR/EN procedure](operations/OS-PACKAGE-MAINTENANCE.md) |
| Activation des mises à jour serveur/web sur installation existante | [Procédure FR/EN](operations/ENABLE-LEGACY-AUTO-UPDATES.md) | [FR/EN procedure](operations/ENABLE-LEGACY-AUTO-UPDATES.md) |
| Pi : mises à jour serveur/web à02:00, preuves du21octobre2026 | [Livraison FR/EN](releases/REL-PI-AUTO-UPDATES-20261021.md) | [FR/EN delivery](releases/REL-PI-AUTO-UPDATES-20261021.md) |
| v0.7 : notes de version et limites | [Release](releases/REL-V0.7.md) | [Release](releases/REL-V0.7.en.md) |
| v0.7 : vérification indépendante des paquets publics | [Rapport FR/EN](releases/REL-V0.7-verification.md) | [FR/EN report](releases/REL-V0.7-verification.md) |
| v0.7 : corrections et preuves finales | [Rapport](implementation/COMPONENT-UPDATES-FIXES-20261019.md) | [Report](implementation/COMPONENT-UPDATES-FIXES-20261019.en.md) |
| Composants 0.7 : preuves et blocage de publication | [Rapport](implementation/COMPONENT-UPDATES-20261019.md) | [Report](implementation/COMPONENT-UPDATES-20261019.en.md) |
| Packs de thèmes et calendrier | [Thèmes](../themes/README.md) | [Themes](../themes/README.en.md) |
| Livraison v0.6 et limites connues | [Release](releases/REL-V0.6.md) | [Release](releases/REL-V0.6.en.md) |
| Candidat suivant : bugs 82–84 et parité web (sans IA) | [Rapport](implementation/WEB-PARITY-20261017.md) | [Report](implementation/WEB-PARITY-20261017.en.md) |
| Déploiement des seuls fichiers web sur Pi | [Livraison](releases/REL-WEB-PARITY-001-web-deployment.md) | [Delivery](releases/REL-WEB-PARITY-001-web-deployment.en.md) |
| Client web smartphone, livraison | [Livraison mobile](releases/REL-WEB-MOBILE-001.md) | [Mobile delivery](releases/REL-WEB-MOBILE-001.en.md) |
| Correctifs mobile R2 et lecture iPhone | [Livraison R2](releases/REL-WEB-MOBILE-002.md) | [R2 delivery](releases/REL-WEB-MOBILE-002.en.md) |
| AVI MP3 et commandes mobiles R3 | [Livraison R3](releases/REL-WEB-MOBILE-003.md) | [R3 delivery](releases/REL-WEB-MOBILE-003.en.md) |
| Bandeau plein écran et marges caméra R4 | [Livraison R4](releases/REL-WEB-MOBILE-004.md) | [R4 delivery](releases/REL-WEB-MOBILE-004.en.md) |
| MP3 AVI côté navigateur et plein écran mobile | [ADR0028](architecture/decisions/ADR-0028-web-mp3-fallback.md) | [ADR0028](architecture/decisions/ADR-0028-web-mp3-fallback.en.md) |
| Contenu mobile et lecteur iPhone, décision | [ADR0027](architecture/decisions/ADR-0027-mobile-content-and-iphone-playback.md) | [ADR0027](architecture/decisions/ADR-0027-mobile-content-and-iphone-playback.en.md) |
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
