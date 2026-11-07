# ADR-0037 — Choix explicite des jaquettes et gestion des collections

2026-11-07 · Implémenté sur `codex/updates-collection-management`, base `daba984`.
Complète ADR-0012 (pages collections) et ADR-0025 (interface partagée Mac/web).
Demande utilisateur : choisir une affiche d'un membre ou rechercher sur TMDB,
et rendre les actions de gestion/suppression accessibles.

Une seule boîte de choix propose les films du groupe, une recherche TMDB et
l'import existant. Le choix se prévisualise puis s'applique explicitement ; il
ne réassocie jamais les métadonnées TMDB des films. Renommer et supprimer sont
visibles dans le groupe ; modifier/supprimer sont aussi accessibles depuis
l'index. La confirmation nomme le groupe, conserve les vidéos et privilégie
Annuler au focus. Les actions des membres sont visibles au clavier et au toucher.

L'API existante de copie d'affiche saga est étendue aux playlists avec CAS.
Une recherche explicite renvoie au plus 24 jetons opaques, jamais d'URL ou chemin
fournisseur. Le serveur garde au plus 256 jetons pendant 10 minutes, liés au
profil, type et groupe. Aperçu et application contrôlent ce contexte ; l'application
contrôle CAS avant téléchargement et lors de l'écriture. Les deux slots TMDB
existants limitent aussi ces requêtes. Les aperçus validés sont limités à 8 Mio.
Les clients utilisent uniquement des routes fixes datées ; macOS exécute le réseau
hors du verrou client/lecteur. Deux aperçus sont chargés en parallèle au maximum.
Fermeture, changement de profil et nouvelle recherche invalident les réponses.

Ce lot n'est pas publié. Les anciens serveurs répondent 404 aux nouvelles routes ;
un message de mise à jour remplace une recherche silencieusement vide.
Rollback : revenir au commit précédent de chaque composant. Aucun changement de
schéma, migration destructive, accès direct au NAS, secret ou frontière réseau.
Preuves : [BUG-0135](../../debug/BUG-0135-collection-management.md).
