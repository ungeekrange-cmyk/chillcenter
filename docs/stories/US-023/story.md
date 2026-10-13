# US-023 — Transférer la lecture entre appareils

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-005 — Reprise, vu et exclusivité.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Confirmation, inaccessibilité et transferts concurrents.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-021, ENABLER-005.
- Blocks : US-025, ENABLER-006, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **confirmer le transfert de ma lecture sur un autre Mac**, afin de **continuer avec une seule lecture active pour mon profil**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-023-AC-01 — Confirmation

Trace : F-07, F-06, F-04.

```gherkin
Given une vidéo est en lecture sur A avec mon profil
When je lance une vidéo sur B puis confirme le transfert présenté
Then A est mis en pause, sa position finale est conservée et B démarre seulement après cet arrêt confirmé
```

### US-023-AC-02 — Refuser

Trace : F-07, F-06, F-04.

```gherkin
Given A lit une vidéo avec mon profil
When je refuse le transfert sur B
Then A continue et B ne démarre pas
```

### US-023-AC-03 — A inaccessible

Trace : F-07, F-06, F-04.

```gherkin
Given A ne répond plus à la demande de pause
When je confirme depuis B
Then B attend la fin du droit de lecture de A, au plus le temps restant du délai nominal de trente secondes ; A se met en pause à cette échéance même avec vidéo en mémoire
```

### US-023-AC-04 — Concurrence et profils distincts

Trace : F-07, F-06, F-04.

```gherkin
Given deux appareils demandent en même temps le même profil tandis qu'un autre profil lit
When les transferts sont confirmés
Then un seul nouvel appareil obtient la lecture du premier profil et l'autre profil continue indépendamment
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-07, F-06, F-04 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

