# ADR-0015 — Identité de série distincte du titre, métadonnées de saison

- PROPOSED, 2026-10-15 ; auteur/propriétaire `/root`, implémentation `/root/build_series_season_tmdb` puis `/root` ; revue `/root/review_series_sync` initiale CHANGES_REQUESTED, nouvelle revue PENDING.
- Branche `codex/library-ux-sync-matching`, base `a59f5e6`. Supersedes/Superseded by NONE ; BUG-0066, API 2026-10-14 additive.

## Contexte, options et décision

Le nom « Doctor Who » ne distingue pas les séries 2005/2023. Fusionner par titre a un effet sur éditions communes, notes de groupe et enchaînement. On retient un identifiant TMDB TV pour les séries reconnues ; avant identification, un groupe interne titre/racine de dossier (un dossier de saison conventionnel est ignoré). Les clés publiques sont `tmdb:tv:<id>` ou `media:<uuid>` d’ancrage ; jamais une racine NAS. Les clés de simple nom existantes ne sont acceptées que pour un groupe non ambigu. L’identité ne sert pas de libellé de recherche/tri.

Une identité UUID de série autonome serait plus stable face aux réorganisations arbitraires de fichiers, mais requiert une migration/gestion explicite des appartenances. Ce lot correctif reste réversible et conserve les UUID vidéo existants. Limite assumée : titres et sources indiscernables ne permettent pas de deviner deux groupes ; une séparation explicite de leurs sources/associations reste nécessaire.

TMDB ne recherche pas une saison comme un film : choisir la série TV puis demander `tv/{id}/season/{n}`. Une saison sans parent choisi est rejetée avant I/O fournisseur. Les données saison sont séparées des champs propres aux épisodes, indexées `(tv_id, saison)`, transférées dans le miroir et projetées en URLs opaques pour les affiches. Import série : groupe figé complet ; import saison : cette saison seulement. CAS sur révisions et groupe, images fraîchement créées retirées en cas d’échec, jamais celles déjà référencées.

La transaction de publication IMMEDIATE relit les révisions communes/média et recalcule le groupe courant depuis son ancre avant toute écriture ; le contrôle avant téléchargement ne suffit pas. Un scan intermédiaire entraîne un conflit explicite, jamais une association partielle. Métadonnées, révisions et migration des vitesses vers l’identité sont committées ensemble. Le contrôle commun exclut les activités personnelles indépendantes.

## Conséquences et validation

SQLite ajoute des tables dérivées de regroupement et un stockage saison. Le regroupement est recalculé une fois par révision commune, pas à chaque lot de 48 titres. Les anciens champs et routes demeurent ; compteur catalogue enrichi de `series_key`, réponse d’édition de `key`. Les clients anciens échouent de façon explicite face à un simple nom devenu ambigu, au lieu de modifier deux séries.

Pas de coût cloud ou nouveau secret, pas de déplacement/renommage NAS. Retour à l’ancien binaire possible sans supprimer les tables ajoutées (mais réintroduit le risque d’ambiguïté ancien). Tests : deux sources homonymes avec plusieurs saisons, édition d’un seul groupe, recherche après identification, saison sans écraser épisode, panne SQLite d’import/révision. Revue et résultats finaux liés au SHA restent à joindre avant déclaration de livraison. Autorité produit : demande explicite de gestion des homonymes et éditions série/saison ; pas de nouvelle découverte requise.
