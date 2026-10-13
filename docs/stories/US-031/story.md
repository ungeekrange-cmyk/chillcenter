# US-031 — Choisir clair, sombre ou automatique

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-009 — Apparence.
- Priorité : Must Have, périmètre validé.
- Complexité : 2 points ; Préférence locale et basculement du système.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-003.
- Blocks : US-032, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **choisir le mode d'affichage de mon Mac**, afin de **consulter confortablement l'application**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-031-AC-01 — Choix

Trace : F-20.

```gherkin
Given l'application ouverte
When je sélectionne Clair, Sombre ou Automatique
Then le mode choisi est appliqué et conservé à la réouverture
```

### US-031-AC-02 — Automatique

Trace : F-20.

```gherkin
Given le mode Automatique sélectionné
When le système passe du clair au sombre ou inversement
Then l'application suit ce changement
```

### US-031-AC-03 — Appareils indépendants

Trace : F-20.

```gherkin
Given le même profil est utilisé sur deux Macs
When je change le mode sur l'un
Then l'autre conserve son propre réglage et le catalogue personnel n'est pas modifié
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-20 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

