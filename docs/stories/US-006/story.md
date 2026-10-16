# US-006 — Indexer les vidéos du NAS

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-002 — Catalogue commun.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Scan incomplet, indisponibilité et conservation des corrections.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-001, ENABLER-004.
- Blocks : US-007, US-012, US-026, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **configurer les sources vidéo du serveur et actualiser le catalogue**, afin de **rendre les vidéos du NAS disponibles à la famille**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-006-AC-01 — Scan complet

Trace : F-03, F-14, F-16, F-18.

```gherkin
Given une source accessible au serveur et des fichiers vidéo
When je lance l'actualisation depuis Administration
Then les nouvelles vidéos sont ajoutées et classées ; leurs fichiers ne sont ni déplacés ni modifiés
```

### US-006-AC-02 — Préserver

Trace : F-03, F-14, F-16, F-18.

```gherkin
Given le catalogue contient corrections, affiches, notes, reprises et playlists
When un nouveau scan complet réussit
Then les éléments déjà identifiés conservent ces informations ; seuls disponibilité et nouveaux éléments sont actualisés
```

### US-006-AC-03 — NAS absent ou scan partiel

Trace : F-03, F-14, F-16, F-18.

```gherkin
Given le NAS est inaccessible ou la lecture d'un dossier échoue
When j'actualise
Then un message signale l'échec et le catalogue précédent est conservé sans purge
```

### US-006-AC-04 — Suivre le scan

Trace : F-03, F-14, F-16, F-18.

```gherkin
Given une actualisation est déjà en cours
When je consulte l'état ou demande une seconde actualisation
Then l'avancement est visible et une seconde actualisation concurrente n'est pas lancée
```

## 5. Contraintes fonctionnelles de qualité

Amendement16octobre2026, **US-006-AC-05** : Configuration › Bibliothèque accepte plusieurs emplacements (au plus16), sans modifier les montages/fichiers. Un scan rassemble toutes les sources puis publie une fois ; une source indisponible conserve l'ancien catalogue entier. Des fichiers homonymes de deux sources restent distincts ; les requêtes historiques conservent les sources supplémentaires omises. Voir ADR0021 et le lot Configuration.

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-03, F-14, F-16, F-18 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

