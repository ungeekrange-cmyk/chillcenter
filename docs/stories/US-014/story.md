# US-014 — Créer et remplir ses playlists

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-003 — Notes et playlists personnelles.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Sélection multividéo, filtres et ajout atomique.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-007.
- Blocks : US-015, US-026, US-030, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **créer des playlists personnelles et y ajouter des vidéos**, afin de **préparer mes séances sans modifier les listes des autres**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-014-AC-01 — Créer depuis fiche ou listes

Trace : F-06, F-14.

```gherkin
Given mon profil est actif
When je crée une playlist vide ou depuis une fiche
Then elle appartient à mon profil ; depuis une fiche sa vidéo est immédiatement ajoutée
```

### US-014-AC-02 — Sélection

Trace : F-06, F-14.

```gherkin
Given une playlist et une sélection de vidéos sur plusieurs pages/filtres
When je confirme l'ajout
Then les vidéos choisies sont ajoutées en ordre de sélection, sans doublon ; les membres existants sont masqués dans le sélecteur
```

### US-014-AC-03 — Annuler ou échec

Trace : F-06, F-14.

```gherkin
Given une sélection en cours ou un ajout impossible
When j'annule ou confirme un ajout qui échoue
Then aucun lot partiel n'est ajouté ; l'erreur conserve ma sélection pour réessayer
```

### US-014-AC-04 — Personnel et suppression

Trace : F-06, F-14.

```gherkin
Given deux profils possèdent leurs listes
When je change de profil ou confirme la suppression d'une de mes listes
Then seules les listes du profil actif sont proposées ; supprimer une liste ne supprime aucune vidéo ni celle d'un autre profil
```

### US-014-AC-05 — Sélection bornée héritée

Trace : F-14.

```gherkin
Given une bibliothèque contenant plus d'une page de vidéos
When je parcours le sélecteur et ajoute une sélection à ma playlist
Then la pagination de quarante-huit cartes conserve la sélection entre pages et filtres ; une validation porte sur mille vidéos maximum et un dépassement est expliqué sans ajout partiel
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-06, F-14 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

