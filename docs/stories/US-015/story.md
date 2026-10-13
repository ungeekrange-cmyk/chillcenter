# US-015 — Réordonner une playlist

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-003 — Notes et playlists personnelles.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Ordre persistant, filtres et modifications concurrentes.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-014.
- Blocks : US-024, US-027, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **choisir l'ordre de mes playlists**, afin de **lire les vidéos dans la séquence souhaitée**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-015-AC-01 — Déplacer

Trace : F-06, F-14, F-12.

```gherkin
Given une playlist personnelle
When je déplace une carte par glisser-déposer ou boutons haut/bas
Then le nouvel ordre est sauvegardé et reste le même sur un autre appareil
```

### US-015-AC-02 — Filtrer

Trace : F-06, F-14, F-12.

```gherkin
Given une playlist dont certains membres sont masqués par filtre
When je déplace un membre visible
Then les membres masqués restent dans la liste et leur ordre relatif est conservé
```

### US-015-AC-03 — Ajouter et rouvrir

Trace : F-06, F-14, F-12.

```gherkin
Given une liste réordonnée
When j'ajoute une vidéo puis rouvre la liste avec un tri global différent
Then la nouvelle vidéo reste à la fin et le tri global ne remplace pas l'ordre de playlist
```

### US-015-AC-04 — Modification concurrente

Trace : F-06, F-14, F-12.

```gherkin
Given un autre appareil a changé la liste depuis mon affichage
When je valide un ordre périmé
Then un conflit est affiché sans écraser l'ordre ou les membres plus récents
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-06, F-14, F-12 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

