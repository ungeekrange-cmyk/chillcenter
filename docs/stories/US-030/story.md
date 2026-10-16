# US-030 — Organiser le catalogue avec l'IA

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-008 — Organisation IA.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Fournisseur externe, aperçu, portée personnelle/commune et conflits.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-009, US-011, US-013, US-014, ENABLER-003.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur ou gestionnaire du foyer**, je veux **demander des recherches et propositions d'organisation à l'IA puis les valider**, afin de **organiser ma vidéothèque en gardant le contrôle**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-030-AC-01 — Bouton et recherche

Trace : F-15, F-14, F-06.

```gherkin
Given un profil actif et un fournisseur IA configuré sur le Mac
When je lance une recherche depuis le bouton IA discret
Then les résultats pertinents sont présentés sans changer le catalogue ; aucune dictée n'est proposée
```

### US-030-AC-02 — Proposition

Trace : F-15, F-14, F-06.

```gherkin
Given une demande d'organisation produit des changements
When je consulte l'aperçu puis confirme
Then la portée commune ou personnelle est indiquée et seuls les changements confirmés sont appliqués ; aucun fichier vidéo n'est renommé, déplacé ou supprimé
```

### US-030-AC-03 — Données et annulation

Trace : F-15, F-14, F-06.

```gherkin
Given une demande IA en préparation ou un aperçu ouvert
When je lance la demande ou l'annule
Then seules les métadonnées nécessaires et données du profil actif sont transmises au fournisseur ; annuler n'applique rien, vidéos/chemins/secrets/autres profils ne sont pas envoyés
```

### US-030-AC-04 — Plan périmé ou fournisseur absent

Trace : F-15, F-14, F-06.

```gherkin
Given le profil ou les fiches ont changé, une proposition est invalide, le fournisseur manque ou une limite est dépassée
When je demande ou confirme une proposition
Then un message explique le problème sans modification partielle ; la recherche et l'organisation manuelles restent disponibles
```

### US-030-AC-05 — Conseil par titres regroupés et métadonnées

Complément demandé le16octobre2026, précision « en fait les deux » et [ADR0020](../../architecture/decisions/ADR-0020-local-title-recommendations.md).

```gherkin
Given le miroir local du profil actif et ses filtres de conseil
When je lance explicitement une demande de conseil
Then le fournisseur reçoit uniquement le contexte autorisé, regroupé en films et séries, avec genres, réalisateurs et acteurs connus
And les suggestions validées conservent son classement dans une ligne de vignettes défilante, sans épisodes individuels ni fusion des homonymes
And aucun changement n'est applicable et quitter/revenir conserve la sélection sans relancer le fournisseur
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-15, F-14, F-06 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

