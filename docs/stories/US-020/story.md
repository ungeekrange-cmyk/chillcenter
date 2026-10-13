# US-020 — Choisir et mémoriser la vitesse d'une série

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-004 — Lecteur Mac.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Valeurs fermées, série/profil et retour au défaut.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-016, US-008.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **adapter la vitesse et retrouver mon dernier choix pour une série**, afin de **regarder à mon rythme sans changer toutes les vidéos**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-020-AC-01 — Valeurs

Trace : F-09, F-10, F-06.

```gherkin
Given une vidéo en lecture
When j'ouvre le choix de vitesse
Then les valeurs ×0,5, ×0,75, ×1, ×1,25, ×1,5, ×1,75 et ×2 sont proposées
```

### US-020-AC-02 — Série

Trace : F-09, F-10, F-06.

```gherkin
Given je choisis ×1,5 sur un épisode
When je lis un autre épisode de cette série sur un autre Mac avec mon profil
Then ×1,5 est retrouvé ; revenir à ×1 est aussi mémorisé
```

### US-020-AC-03 — Autre série ou profil

Trace : F-09, F-10, F-06.

```gherkin
Given un autre profil ou une série sans choix enregistré
When je lance un épisode
Then la vitesse initiale est ×1, indépendante de mes autres séries
```

### US-020-AC-04 — Hors série

Trace : F-09, F-10, F-06.

```gherkin
Given j'ai accéléré un film pendant sa lecture
When je lance une nouvelle vidéo hors série
Then elle commence à ×1 ; ma modification précédente n'est pas un défaut global
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-09, F-10, F-06 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

