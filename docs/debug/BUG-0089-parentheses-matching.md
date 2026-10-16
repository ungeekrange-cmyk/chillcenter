# BUG-0089 — Parenthèses résiduelles dans les requêtes automatiques de matching

16 octobre 2026. **FIXED_LOCAL — fournisseur réel non sollicité.**

Inventaire demandé par l'utilisateur : miroir2496entrées parcouru en lecture seule (titres/chemins relatifs seulement),19titres avec ouvrante orpheline,90titres à parenthèses équilibrées. Des suffixes techniques tronqués et anciens titres indexés conservent un délimiteur qui fuite par `title_for_search` vers la requête automatique.

Correction partagée dans `scanner::tidy` : supprimer les délimiteurs réellement sans partenaire, conserver le texte et les groupes complets faisant partie du titre (ex. Birdman et son sous-titre). Parenthèses/crochets vides et années déjà couverts restent traités ; parsing des nouveaux fichiers et nettoyage de recherche des titres déjà indexés partagent le correctif. Pas de suppression aveugle de tout texte entre parenthèses.

Preuves : test RED avant/GREEN après,11tests scanner et13tests liés métadonnées PASS. Couverture d'ouvrantes orphelines, épisode avec `(Part 1)` puis suffixe1080p, parenthèse valide dans titre et cas source mal formé. Aucun nom/ID/fichier NAS ou titre manuel réécrit, aucune reconnaissance massive/provider exécutée. Une recherche explicitement saisie par l'utilisateur reste conservée ; le correctif concerne le nom extrait et les requêtes automatiques.
