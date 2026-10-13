# US-008 — Explorer une série par saisons et épisodes

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-002 — Catalogue commun.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Regroupement, ordre et compteurs personnels.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-007.
- Blocks : US-009, US-013, US-020, US-024, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **ouvrir une série puis ses saisons et épisodes**, afin de **retrouver un épisode sans perdre le contexte de la série**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-008-AC-01 — Navigation

Trace : F-14, F-12.

```gherkin
Given une série comportant plusieurs saisons
When j'ouvre sa carte puis une saison
Then les saisons puis épisodes sont ordonnés par numéro croissant et les saisons sont initialement repliées
```

### US-008-AC-02 — Informations communes

Trace : F-14, F-12.

```gherkin
Given une série avec synopsis et affiche
When je consulte sa présentation
Then ses informations communes sont distinctes des titres et résumés des épisodes
```

### US-008-AC-03 — Compteurs

Trace : F-14, F-12.

```gherkin
Given des épisodes vus et non vus pour mon profil et un filtre actif
When je consulte les compteurs de série ou saison
Then les compteurs portent sur tous leurs épisodes présents, selon mon profil, sans être réduits par le filtre
```

### US-008-AC-04 — Numéro inconnu

Trace : F-14, F-12.

```gherkin
Given une série contient un épisode sans numéro identifié
When je la parcours
Then cet épisode reste accessible, classé après les numéros connus et sans inventer un numéro
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-14, F-12 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

