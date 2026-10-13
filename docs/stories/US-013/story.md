# US-013 — Noter les vidéos avec son profil

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-003 — Notes et playlists personnelles.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Notes agrégées et propagation limitée au profil.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-007, US-008.
- Blocks : US-026, US-027, US-030, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **attribuer mes notes aux vidéos, saisons ou séries**, afin de **retrouver mes appréciations indépendamment du foyer**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-013-AC-01 — Noter ou retirer

Trace : F-06, F-14.

```gherkin
Given une fiche ouverte sous mon profil
When je donne une à cinq étoiles ou retire la note
Then ma note est sauvegardée et synchronisée, sans modifier celle d'un autre profil
```

### US-013-AC-02 — Série ou saison

Trace : F-06, F-14.

```gherkin
Given des épisodes présents d'une série ou saison
When je donne une note à ce regroupement
Then la note remplace uniquement celles de ses épisodes présents pour mon profil
```

### US-013-AC-03 — Divergence

Trace : F-06, F-14.

```gherkin
Given des épisodes ont des notes différentes
When je consulte la série ou saison
Then les différences sont signalées ; noter un épisode ne remplace pas les notes de tous les autres
```

### US-013-AC-04 — Épisode nouveau

Trace : F-06, F-14.

```gherkin
Given j'ai noté une saison avant un nouveau scan
When un épisode supplémentaire est découvert
Then il ne reçoit pas rétroactivement ma note de groupe
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-06, F-14 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

