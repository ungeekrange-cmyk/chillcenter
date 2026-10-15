# Lot catalogue/fiches/série/saison — candidat local

Coordinateur `/root`, 2026-10-15 ; base `a59f5e6`, branche `codex/library-ux-sync-matching`. Aucun nouveau déploiement à ce stade. Le Pi et le bundle livré restent c610ea9.

## Régressions et preuves

BUG-0064/0065/0066 et ADR-0014 documentent les causes et limites. Première revue `/root/review_series_sync` : CHANGES_REQUESTED (édition des clés, homonymes inconnus, libellé de recherche, saison sans parent, révision d’image, nettoyage). Corrections apportées, nouvelle revue du SHA final encore nécessaire.

- Node ciblé : 28 tests PASS (`catalog-loading.dom`, `admin-navigation.dom`, `tmdb.dom`, `tests/*.test.js`). Catalogue conservé jusqu’au commit, réponses fiche désordonnées et cinq étoiles couverts.
- Rust : `cargo test --locked --offline -p chill-domain -p chill-client -p chill-server --lib` : 37 client + 59 domaine + 101 serveur PASS au checkpoint de travail. Ces tests sont isolés ; aucune nouvelle lecture sur le NAS ou manipulation du lecteur réel.
- Domaine : fixture deux arborescences Doctor Who, deux saisons/deux épisodes par saison ; deux cartes de quatre épisodes, édition/matching bornés au groupe choisi, recherche par titre après identification. Échec DB d’import série/saison : image fraîche retirée, révision de saison réelle.
- Navigateur jetable `scripts/test-library-ux.cjs` PASS : contrôles d’affiche, étoiles, fiche sans ancien contenu, import/matching sans fermer, fermeture extérieure, scroll conservé, recherche utilisable pendant synchro suspendue ; file de matching arrêtée avec réponse fournisseur retardée. Pas de données personnelles/serveur réel.
- Lancement initial de Chromium dans le bac à sable refusé par macOS (MachPort) ; relance autorisée du navigateur jetable réussie. Aucun pilote installé, aucun onglet utilisateur attaché.
- Contrat OpenAPI : validation à 75 chemins ; version 2026-10-14, alias v1 conservé. Validation finale à réexécuter après gel.

## Limites / restant

Revue finale, mesures réelles durée/RSS du delta Pi et vérification du bundle Tauri restent distinctes des preuves ci-dessus. Le matching automatique des titres français/originaux n’est pas couvert par ce lot. Le regroupement inconnu s’appuie sur l’arborescence source ; les médias indistinguables dans une même arborescence ne peuvent pas être désambiguïsés par leur seul nom. Ne pas annoncer toutes les US achevées sur cette base.
