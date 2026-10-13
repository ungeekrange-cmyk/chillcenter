# Traçabilité, décisions et validations

Suivi vivant au 2026-10-13 ; auteur `/root`. Entrées acceptées : `CC-FONC-002` et `CC-DOC-001`. Stories en revue : `CC-STORIES-001`. Les IDs F et AC sont stables ; [le backlog](BACKLOG.md) et [sa matrice](stories/TRACEABILITY.md) relient les exigences aux US/AC et travaux techniques. Aucun commit de code créé.

## Jalons utilisateur

| Gate | Révision | Statut | Preuve |
| --- | --- | --- | --- |
| G-01 fonctionnalités | CC-FONC-002 | APPROVED_BY_USER | « vise directement un serveur sur linux, on va zapper la version mac, le reste ok », après synthèse CC-FONC-001. |
| G-02 documentation/architecture | CC-DOC-001 | APPROVED_BY_USER | « oui étape suivante » ; [gate canonique](gates/G-02-CC-DOC-001.md). |
| G-03 user stories | CC-STORIES-001 | WAITING_USER | Rédaction et revue après G-02 ; implémentation seulement après validation de G-03. |
| G-04 implémentation | NOT_CREATED | NOT_STARTED | Commencer seulement après validation des stories ; remise/test final puis validation de l'implémentation. |

La confirmation du nom `ChillCenter` fixe la destination. La revue indépendante et la validation utilisateur G-02 sont désormais acquises pour leurs revisions exactes ; budgets, baux, compteur final et moteur y sont inclus. Cela n'est pas une preuve de code, benchmark ou livraison.

## Exigences → décisions → preuves prévues

| Exigences | Architecture | ADR | Vérification |
| --- | --- | --- | --- |
| F-01–04 | §§1–4, 10 | 0001, 0004 | V-01–03 |
| F-05–06 | §§5, 8 | 0002, 0003 | V-04, V-08, V-10 |
| F-07–08 | §5, contrats | 0003 | V-04–05 |
| F-09–11, F-22 | §§4, 6 | 0004 | V-03, V-05 |
| F-12–13 | §6 | 0003 | V-06 |
| F-14–15 | §§2, 8–9 | 0002, 0007 | V-07, V-10 |
| F-16–17 | §8 | 0002, 0006 | V-07–08, V-12 |
| F-18–19 | §§3, 7–8 | 0001, 0005, 0006 | V-09–10 |
| F-20–21 | §9 | Règles validées, sans ADR distinct | V-11 |

## Évidence exécutée à cette étape

- Lecture README/manifeste/model/database/scanner/assistant et diagnostic NAS historique.
- `git bundle verify Cinefolio-source.bundle` : réussi ; hash/base dans SOURCES L-06.
- `gh repo view ungeekrange-cmyk/chillcenter --json nameWithOwner,isEmpty,defaultBranchRef,url` : réussi après permission réseau ; dépôt vide.
- Documentation primaire datée : SOURCES S-01 à S-11.
- Revue indépendante et vérification des liens : voir rapports et journal après exécution.

Tests, builds, mesures Pi, packaging Mac et déploiement : NOT_RUN. Aucun bug Chill Center inventé à partir d'un risque hypothétique. Tout changement matériel après G-02 rend les parties affectées du dossier/stories à revalider ; une correction documentaire non sémantique peut rester sur la même décision avec hash de révision actualisé.
