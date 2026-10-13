# US-018 — Configurer ses langues de lecture

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-004 — Lecteur Mac.
- Priorité : Must Have, périmètre validé.
- Complexité : 2 points ; Deux préférences distinctes, défaut et synchronisation.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-003.
- Blocks : US-019, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **choisir mes langues préférées pour audio et sous-titres**, afin de **les retrouver lors de mes lectures sur tous mes Macs**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-018-AC-01 — Défaut

Trace : F-10, F-11, F-06.

```gherkin
Given un profil nouvellement créé
When j'ouvre ses préférences
Then audio anglais et sous-titres anglais sont sélectionnés par défaut
```

### US-018-AC-02 — Configurer

Trace : F-10, F-11, F-06.

```gherkin
Given mon profil possède des préférences
When je choisis une langue audio et une langue de sous-titres puis confirme
Then les deux préférences sont sauvegardées indépendamment et retrouvées sur un autre appareil
```

### US-018-AC-03 — Isolation

Trace : F-10, F-11, F-06.

```gherkin
Given un autre profil utilise ses langues
When je modifie les miennes
Then ses préférences restent inchangées
```

### US-018-AC-04 — Échec

Trace : F-10, F-11, F-06.

```gherkin
Given la sauvegarde des préférences échoue
When je confirme une modification
Then un message permet de réessayer sans afficher un changement non enregistré comme acquis
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-10, F-11, F-06 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

