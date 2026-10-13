# US-011 — Organiser les films en sagas communes

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-002 — Catalogue commun.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Membres multiples, image, nom et ordre hérité.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-007, US-010.
- Blocks : US-024, US-030, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **créer et modifier des sagas communes**, afin de **regrouper les films liés pour tous les profils**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-011-AC-01 — Créer et ajouter

Trace : F-14, F-12.

```gherkin
Given des films dans le catalogue
When je crée une saga puis sélectionne plusieurs films
Then la saga commune apparaît, y compris vide ; un film peut appartenir à plusieurs sagas
```

### US-011-AC-02 — Ordre

Trace : F-14, F-12.

```gherkin
Given une saga contenant des films avec et sans année
When je l'ouvre
Then l'ordre par défaut est année puis titre, années inconnues à la fin ; changer le tri n'altère pas une playlist
```

### US-011-AC-03 — Renommer et illustrer

Trace : F-14, F-12.

```gherkin
Given une saga existante
When je change son nom ou choisis une affiche de film, une image personnelle ou un visuel TMDB
Then ses membres restent associés et son image choisie reste fixée même si un film est retiré ; un nom déjà utilisé est refusé
```

### US-011-AC-04 — Retirer ou supprimer

Trace : F-14, F-12.

```gherkin
Given une saga avec des membres
When je retire un film ou confirme la suppression de la saga
Then seules les associations ou la saga sont supprimées, jamais vidéos, notes ou fichiers NAS ; annuler ne change rien
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-14, F-12 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

