# US-009 — Corriger les informations du catalogue

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-002 — Catalogue commun.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Éditions communes/épisodes, conflits et non-fusion.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-007, US-008.
- Blocks : US-010, US-030, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **corriger une fiche ou les informations communes d'une série**, afin de **rendre les descriptions de la bibliothèque fiables**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-009-AC-01 — Vidéo

Trace : F-14.

```gherkin
Given une fiche de vidéo
When je corrige ses informations et confirme
Then ses champs valides sont sauvegardés et consultables depuis un autre appareil sans renommer le fichier
```

### US-009-AC-02 — Série distincte

Trace : F-14.

```gherkin
Given une série avec plusieurs épisodes
When je modifie les métadonnées communes de la série
Then ses informations communes changent sans écraser titres, résumés, notes ou états personnels des épisodes
```

### US-009-AC-03 — Association épisode

Trace : F-14.

```gherkin
Given un épisode mal classé
When je corrige sa série, saison ou numéro
Then cet épisode rejoint le bon regroupement et conserve son identité et les données personnelles des profils
```

### US-009-AC-04 — Conflit et fusion

Trace : F-14.

```gherkin
Given une fiche modifiée depuis mon ouverture ou un renommage de série vers un nom déjà utilisé
When je valide mon édition
Then un conflit est signalé sans écraser une modification concurrente ni fusionner deux séries involontairement
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-14 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

