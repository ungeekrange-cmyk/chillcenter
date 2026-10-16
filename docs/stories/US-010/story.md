# US-010 — Enrichir les fiches et personnaliser les affiches

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-002 — Catalogue commun.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Service externe, variantes série/épisode, cache et images personnelles.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-009.
- Blocks : US-011, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **choisir les bonnes fiches TMDB et leurs affiches**, afin de **compléter et illustrer le catalogue**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-010-AC-01 — Choisir une correspondance

Trace : F-14.

```gherkin
Given TMDB configuré et une recherche ambiguë
When je recherche un film ou une série puis sélectionne une proposition
Then les champs correspondant au niveau choisi sont enrichis ; une fiche de série ne remplace pas les informations propres aux épisodes
```

### US-010-AC-02 — Affiche personnelle

Trace : F-14.

```gherkin
Given une fiche ou saga avec une affiche fournisseur
When j'importe une image JPEG, PNG ou WebP valide de huit MiB maximum
Then une copie est affichée, l'image d'origine reste intacte et cette affiche personnelle résiste aux enrichissements
```

### US-010-AC-03 — Actualiser ou restaurer

Trace : F-14.

```gherkin
Given une fiche associée à TMDB avec corrections et affiche personnelle
When je demande explicitement l'actualisation fournisseur ou la restauration de son affiche
Then les champs de l'action choisie sont mis à jour ; une correction concurrente n'est pas écrasée et une restauration échouée conserve l'affiche personnelle
```

### US-010-AC-04 — Fournisseur ou image indisponible

Trace : F-14.

```gherkin
Given TMDB échoue, le jeton manque ou l'image est invalide/trop volumineuse
When je lance l'action
Then un message explique le problème et les informations précédentes ainsi que les images en cache restent consultables
```

## 5. Contraintes fonctionnelles de qualité

Amendement16octobre2026, **US-010-AC-05** : valider une proposition ferme immédiatement les choix, affiche « Association en cours avec XXX… » et un indicateur local ; succès seulement après réponse serveur, erreur explicitement non confirmée et ancienne réponse incapable de repeindre une nouvelle fiche. Reste de l'application utilisable. **AC-06** : recherche automatique avec nettoyage des suffixes de release et au plus un secours prudent après zéro résultat, sans réécriture d'une saisie manuelle ni acceptation d'un titre ambigu/mauvaise année. BUG0081/0094.

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-14 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

