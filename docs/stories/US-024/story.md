# US-024 — Proposer la bonne vidéo suivante

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-006 — Enchaînement.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Priorités de contexte et changement de saison.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-008, US-011, US-015, US-016.
- Blocks : US-025, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **voir la prochaine vidéo selon le contexte de lancement**, afin de **continuer ma série, saga ou playlist dans le bon ordre**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-024-AC-01 — Playlist prioritaire

Trace : F-12.

```gherkin
Given un épisode lancé depuis une playlist où la vidéo suivante est un film
When l'épisode se termine
Then le film suivant de cette playlist est proposé, pas l'épisode suivant de la série
```

### US-024-AC-02 — Série et saison

Trace : F-12.

```gherkin
Given le dernier épisode d'une saison a une saison suivante présente
When il se termine après lancement depuis la série ou sa fiche
Then le premier épisode suivant est proposé avec la mention Nouvelle saison
```

### US-024-AC-03 — Saga ou film isolé

Trace : F-12.

```gherkin
Given un film lancé depuis une saga ou depuis sa fiche isolée
When il se termine
Then depuis la saga, le suivant de son ordre est proposé ; depuis la fiche isolée, aucun choix arbitraire de saga ou playlist n'est effectué
```

### US-024-AC-04 — Suite inconnue ou changée

Trace : F-12.

```gherkin
Given les numéros de série sont inconnus, la collection a changé ou aucun suivant n'existe
When la vidéo se termine
Then aucun ordre n'est inventé ; une collection modifiée est réévaluée et l'absence de suivant n'entraîne pas de lancement
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-12 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

