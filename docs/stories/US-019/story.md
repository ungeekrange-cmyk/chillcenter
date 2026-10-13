# US-019 — Choisir audio et sous-titres dans le lecteur

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-004 — Lecteur Mac.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Pistes intégrées/externes, langue manquante et association.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-016, US-018.
- Blocks : ENABLER-006, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **sélectionner les pistes audio et sous-titres de la vidéo**, afin de **la regarder dans la langue souhaitée**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-019-AC-01 — Préférences disponibles

Trace : F-09, F-10, F-11, F-22.

```gherkin
Given la vidéo dispose des langues préférées du profil
When je lance la lecture
Then les pistes préférées sont sélectionnées et les pistes disponibles sont consultables
```

### US-019-AC-02 — Choisir

Trace : F-09, F-10, F-11, F-22.

```gherkin
Given une vidéo avec plusieurs pistes audio et sous-titres intégrés ou externes
When je sélectionne une autre piste ou désactive les sous-titres
Then le choix s'applique à la lecture sans nécessiter d'accès direct au stockage
```

### US-019-AC-03 — Anglais ou préférence absente

Trace : F-09, F-10, F-11, F-22.

```gherkin
Given la langue choisie pour audio ou sous-titres n'existe pas dans le fichier
When je lance la vidéo
Then l'audio original identifiable sinon la première piste est utilisé ; sous-titres absents de la langue désactivés ; indication discrète, préférences conservées
```

### US-019-AC-04 — Fichier externe ambigu ou invalide

Trace : F-09, F-10, F-11, F-22.

```gherkin
Given plusieurs sous-titres externes candidats ou un fichier illisible
When je choisis les sous-titres
Then un choix explicite ou une erreur est proposé sans charger silencieusement un fichier d'une autre vidéo
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-09, F-10, F-11, F-22 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

