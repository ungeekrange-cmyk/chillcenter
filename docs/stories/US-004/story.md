# US-004 — Supprimer un profil sans toucher à la bibliothèque

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-001 — Accès et profils.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Suppression ciblée, lecture active et annulation.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-003, ENABLER-005.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **supprimer un profil après confirmation**, afin de **retirer ses données personnelles sans affecter le reste du foyer**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-004-AC-01 — Confirmer

Trace : F-05, F-06, F-18.

```gherkin
Given un profil contenant notes, reprises et playlists
When je confirme sa suppression après le récapitulatif
Then ses données personnelles sont retirées ; vidéos, métadonnées, sagas et autres profils restent intacts
```

### US-004-AC-02 — Annuler

Trace : F-05, F-06, F-18.

```gherkin
Given une confirmation de suppression ouverte
When j'annule
Then le profil, ses données et sa lecture éventuelle ne changent pas
```

### US-004-AC-03 — Lecture et appareils ouverts

Trace : F-05, F-06, F-18.

```gherkin
Given le profil supprimé est actif sur un Mac
When la suppression est confirmée
Then sa lecture s'arrête sans données personnelles résiduelles ; les appareils concernés proposent les profils restants, ou l'état vide avec Créer un profil et accès à Administration si le dernier profil a été supprimé
```

### US-004-AC-04 — Échec

Trace : F-05, F-06, F-18.

```gherkin
Given une suppression ne peut être enregistrée
When je la confirme
Then l'erreur est affichée sans disparition partielle des données
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-05, F-06, F-18 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

