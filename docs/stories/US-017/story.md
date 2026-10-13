# US-017 — Naviguer dans la vidéo

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-004 — Lecteur Mac.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Bornes temporelles et navigation pendant buffering.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-016.
- Blocks : ENABLER-006, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **avancer, revenir et choisir une position**, afin de **retrouver le passage voulu**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-017-AC-01 — Sauts

Trace : F-09.

```gherkin
Given une vidéo de durée connue à une position intérieure
When j'appuie sur avance ou retour
Then la position cible avance de trente secondes ou recule de dix secondes
```

### US-017-AC-02 — Bornes

Trace : F-09.

```gherkin
Given je suis à moins de dix secondes du début ou trente secondes de la fin
When je demande un saut
Then la cible reste entre zéro et la durée réelle, sans position négative ou dépassement
```

### US-017-AC-03 — Barre

Trace : F-09.

```gherkin
Given une vidéo ouverte
When je déplace la barre de progression
Then la nouvelle position est affichée et la lecture reprend à cette position dès les données disponibles
```

### US-017-AC-04 — Chargement

Trace : F-09.

```gherkin
Given un saut nécessite de nouvelles données ou échoue
When je navigue
Then le chargement ou l'erreur est visible et je peux réessayer ; le saut seul ne compte pas comme visionnage
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-09 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

