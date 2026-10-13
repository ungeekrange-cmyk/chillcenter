# US-002 — Créer et renommer les profils du foyer

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-001 — Accès et profils.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Identité persistante et validation du nom.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-001, ENABLER-004.
- Blocks : US-003, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **membre du foyer**, je veux **créer et renommer un profil**, afin de **disposer d'une identité personnelle dans la bibliothèque**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-002-AC-01 — Créer

Trace : F-05, F-18.

```gherkin
Given la rubrique Administration et un nom de profil valide
When je confirme la création
Then un profil sélectionnable est créé sans mot de passe, avec ses données personnelles initialement vides
```

### US-002-AC-02 — Renommer

Trace : F-05, F-18.

```gherkin
Given un profil possédant notes, listes et historique
When je change son nom et confirme
Then le nom est actualisé sur les appareils connectés et toutes ses données restent associées au même profil
```

### US-002-AC-03 — Nom invalide

Trace : F-05, F-18.

```gherkin
Given un formulaire avec nom vide ou uniquement composé d'espaces
When je valide
Then la création ou le renommage est refusé avec une explication et aucun profil valide n'est modifié
```

### US-002-AC-04 — Premier profil sans profil actif

Trace : F-05, F-18.

```gherkin
Given un serveur connecté qui ne contient aucun profil et un Mac sans profil actif
When j'arrive sur l'écran de choix des profils
Then l'état vide propose Créer un profil et l'accès à Administration avant toute sélection ; je peux saisir un nom valide, confirmer puis choisir le profil créé sans mot de passe ni profil préexistant
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-05, F-18 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

