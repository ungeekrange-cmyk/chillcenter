# US-016 — Lire une vidéo dans l'application Mac

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-004 — Lecteur Mac.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Formats réels, plein écran et états de lecture.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-003, US-007, ENABLER-001, ENABLER-003, ENABLER-005.
- Blocks : US-017, US-019, US-020, US-021, US-024, ENABLER-006, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **lancer une vidéo dans le lecteur intégré**, afin de **la regarder sans lecteur externe ni accès personnel au NAS**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-016-AC-01 — Lancer

Trace : F-02, F-03, F-09, F-22.

```gherkin
Given une vidéo disponible dans le catalogue
When je choisis son bouton Lecture
Then la vidéo démarre dans l'application et ses données sont fournies par le serveur sans montage NAS sur mon Mac
```

### US-016-AC-02 — Formats prioritaires

Trace : F-02, F-03, F-09, F-22.

```gherkin
Given les fichiers MKV, MP4 et AVI représentatifs de la matrice de validation
When je les lis
Then les fichiers pris en charge sont lus avec son et image ; un codec non pris en charge est expliqué plutôt que annoncé compatible
```

### US-016-AC-03 — Affichage et pause

Trace : F-02, F-03, F-09, F-22.

```gherkin
Given une vidéo en cours
When j'alterne pause/lecture, redimensionnement et plein écran
Then la lecture reste contrôlable et les informations de la fiche ne sont pas perdues
```

### US-016-AC-04 — NAS ou réseau perdu

Trace : F-02, F-03, F-09, F-22.

```gherkin
Given une lecture en cours puis une source indisponible
When le tampon ne permet plus de continuer
Then le lecteur signale attente ou indisponibilité sans lancer une autre vidéo ni marquer vu par simple ouverture
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-02, F-03, F-09, F-22 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

