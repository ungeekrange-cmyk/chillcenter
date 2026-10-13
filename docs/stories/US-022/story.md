# US-022 — Marquer vu selon le visionnage final

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-005 — Reprise, vu et exclusivité.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Calcul exact, vitesses, cumul et action manuelle.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-021.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **retrouver un statut vu fidèle à mon visionnage**, afin de **distinguer ce que j'ai regardé de ce que j'ai simplement ouvert**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-022-AC-01 — Seuil

Trace : F-08, F-06, F-14.

```gherkin
Given une vidéo de mille secondes, non vue pour mon profil
When je regarde neuf secondes réelles puis une seconde supplémentaire dans les positions neuf cents à mille secondes
Then elle reste non vue après neuf secondes et devient vue à dix secondes, sans changer l'état des autres profils
```

### US-022-AC-02 — Saut et interruptions

Trace : F-08, F-06, F-14.

```gherkin
Given une vidéo non vue
When je saute à la fin, reste en pause ou attends un chargement
Then ces temps et déplacements seuls ne font pas passer la vidéo à vue
```

### US-022-AC-03 — Vitesse et cumul

Trace : F-08, F-06, F-14.

```gherkin
Given une vidéo dont le dernier dixième est suffisamment long
When je regarde cinq secondes réelles à ×2 dans ce dernier dixième puis cinq secondes réelles lors d'une session ultérieure
Then seulement dix secondes réelles cumulées sont comptées ; la position parcourue n'est pas assimilée au temps regardé
```

### US-022-AC-04 — Manuel ou durée inconnue

Trace : F-08, F-06, F-14.

```gherkin
Given une vidéo vue ou dont la durée réelle n'est pas connue
When je marque manuellement non vu ou consulte son état
Then non vu réinitialise le compteur final sans effacer la reprise ; une durée inconnue ne déclenche pas de seuil inventé
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-08, F-06, F-14 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

