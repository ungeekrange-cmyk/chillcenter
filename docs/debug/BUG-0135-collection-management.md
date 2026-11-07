# BUG-0135 — Jaquettes et gestion des collections insuffisantes

2026-11-07 · Branche `codex/updates-collection-management`, base `daba984`.
Statut : corrigé et vérifié localement ; aucune nouvelle release publiée.

Symptômes reproduits dans l'interface : gestion repliée, suppression difficile à
trouver, choix limité à l'import ou une action longue depuis un membre de saga,
pas de recherche TMDB propre au groupe ni de copie depuis un membre de playlist.
Cause confirmée par lecture des handlers et des routes : capacités et parcours
incomplets, pas une panne de fournisseur.

Correction : barre visible Ajouter/Renommer/Supprimer, actions sur les cartes de
l'index, confirmation nommée avec vidéos conservées, sélecteur commun à trois
sources. Sélection explicite puis application CAS. Contrats et limites décrits
dans ADR-0037 et OpenAPI ; mêmes contrôles pour web et Mac.

Vérification locale : 241 tests Node de l'interface et des adaptateurs passent.
Rust chill-client : 54 passent, un test ignoré préexistant ; chill-updates :
18 tests bibliothèque et 4 tests binaire passent. Contrat OpenAPI : 85 routes.
La validation a révélé une déclaration de paramètre playback-progress au niveau
opération incompatible avec le validateur interne : déplacée au niveau chemin,
sans changement de route ou du DTO.

Essais manuels dans le vrai navigateur sur données synthétiques (aucune donnée
familiale modifiée) : copie d'une affiche de membre saga et playlist ; recherche,
sélection et application TMDB simulées ; renommage ; annulation et suppression
d'une playlist de test avec retour à l'index. Rendus 1440×900 et 375×812 contrôlés.
Capture conservée hors Git dans l'espace de travail du lot.

Limites : aucun téléchargement TMDB réel avec la clé familiale, aucun nouveau
binaire Mac lancé, aucune installation Pi/web de ce lot. Les tests provider ne
prouvent pas sa disponibilité en production. Les suites serveur/revue finale
sont consignées dans l'évidence du candidat avant livraison.

Rollback : déployer les versions précédentes des composants ; aucune migration
SQL ni suppression de fichier vidéo. Le remplacement d'affiche conserve les
métadonnées et identités des membres.
