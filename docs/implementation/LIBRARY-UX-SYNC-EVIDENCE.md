# Lot catalogue/fiches/série/saison — candidat local

Coordinateur `/root`, 2026-10-15 ; base `a59f5e6`, branche `codex/library-ux-sync-matching`. Aucun nouveau déploiement à ce stade. Le Pi et le bundle livré restent c610ea9.

## Régressions et preuves

BUG-0064/0065/0066 et ADR-0014 documentent les causes et limites. Première revue `/root/review_series_sync` : CHANGES_REQUESTED (édition des clés, homonymes inconnus, libellé de recherche, saison sans parent, révision d’image, nettoyage). Corrections apportées, nouvelle revue du SHA final encore nécessaire.

- Node ciblé : 41 tests PASS, dont connexion/acceptation de certificat et disponibilité IA après nettoyage des désactivations transitoires. Catalogue conservé jusqu’au commit, réponses fiche désordonnées et cinq étoiles couverts. Revue UI indépendante de 3bbb87d : 50 tests ciblés et navigateur synthétique PASS, APPROVED.
- Rust : `cargo test --locked --offline -p chill-catalog -p chill-domain -p chill-client -p chill-server --lib --quiet` : 75 catalogue + 39 client + 64 domaine + 103 serveur PASS. La suite propriétaire catalogue, omise lors du premier checkpoint, révélait deux échecs : complétude de groupe et attente de collision de titre obsolète, corrigés et revus séparément. Les autres preuves incluent borne globale multi-profils, budget JSON cumulé, ancienne table de cache vidée, transaction finale refusant un scan concurrent, refus du quatrième transfert sans checkpoint ni éviction, libération après dernière page et génération après restauration officielle. Tests isolés, sans NAS ni fournisseur réel. Desktop : 23 tests PASS au checkpoint 3bbb87d ; aucun nouveau parcours lecteur réel.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` PASS après correction de trois avertissements du candidat local. Aucun avertissement ignoré globalement.
- Domaine : fixture deux arborescences Doctor Who, deux saisons/deux épisodes par saison ; deux cartes de quatre épisodes, édition/matching bornés au groupe choisi, recherche par titre après identification. Échec DB d’import série/saison : image fraîche retirée, révision de saison réelle.
- Navigateur jetable `scripts/test-library-ux.cjs` PASS : contrôles d’affiche, étoiles, fiche sans ancien contenu, import/matching sans fermer, fermeture extérieure, scroll conservé, recherche utilisable pendant synchro suspendue ; file de matching arrêtée avec réponse fournisseur retardée, nouveau delta après mutation concurrente, ajout playlist, import série/saison (section conservée). Le bouton IA ouvre les réglages sans démarrer le fournisseur. Pas de données personnelles/serveur réel.
- Lancement initial de Chromium dans le bac à sable refusé par macOS (MachPort) ; relance autorisée du navigateur jetable réussie. Aucun pilote installé, aucun onglet utilisateur attaché.
- Contrat OpenAPI : validation à 75 chemins ; version 2026-10-14, alias v1 conservé. Validation finale à réexécuter après gel.

## Limites / restant

Revue finale, mesures réelles durée/RSS du delta Pi et vérification du bundle Tauri restent distinctes des preuves ci-dessus. Le matching automatique des titres français/originaux n’est pas couvert par ce lot. Le regroupement inconnu s’appuie sur l’arborescence source ; les médias indistinguables dans une même arborescence ne peuvent pas être désambiguïsés par leur seul nom. Ne pas annoncer toutes les US achevées sur cette base.
