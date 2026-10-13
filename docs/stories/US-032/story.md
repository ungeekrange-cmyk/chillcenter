# US-032 — Activer les ambiances de saison et d'événement

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-009 — Apparence.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Calendrier, priorités, fuseau et variantes.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-031.
- Blocks : US-033, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **profiter d'une ambiance automatique adaptée à la date**, afin de **retrouver une vidéothèque qui suit les saisons**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-032-AC-01 — Saisons

Trace : F-20, F-21.

```gherkin
Given les ambiances automatiques sont actives
When la date locale atteint le premier mars, juin, septembre ou décembre
Then l'ambiance devient respectivement printemps, été, automne ou hiver avec la variante claire ou sombre correspondant au mode
```

### US-032-AC-02 — Halloween

Trace : F-20, F-21.

```gherkin
Given les ambiances automatiques sont actives
When la date locale passe du trente septembre au premier octobre puis au premier novembre
Then Halloween est actif du premier au trente-et-un octobre inclus ; l'automne revient le premier novembre
```

### US-032-AC-03 — Noël

Trace : F-20, F-21.

```gherkin
Given les ambiances automatiques sont actives
When la date locale atteint le vingt-cinq novembre puis le vingt-six décembre
Then Noël prime sur la saison jusqu'au vingt-cinq décembre inclus puis l'hiver revient
```

### US-032-AC-04 — Réveil

Trace : F-20, F-21.

```gherkin
Given le Mac a dormi ou l'application était fermée pendant un changement de date ou de mode
When je la rouvre ou la réveille
Then l'ambiance et sa variante sont réévaluées sans animations permanentes ni attente d'un contrôle continu
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-20, F-21 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

