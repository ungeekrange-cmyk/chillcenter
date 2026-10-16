# Suite, affiches, défilement et matching — remise locale

Mise à jour16octobre14:44UTC : connexion SSH rétablie, serveur seul installé avec sauvegarde privée et rc0 ; web/configuration/NAS inchangés. [Exécution et rollback](../releases/REL-NEXT-POSTERS-SCROLL-MATCHING-001.md), [vérification indépendante APPROVED_SCOPED](../releases/REL-NEXT-POSTERS-SCROLL-MATCHING-001-verification.md) dont vrai001→002 PASS. Le blocage SSH décrit au checkpoint de compilation plus bas est conservé comme historique, pas l'état actuel.

16 octobre 2026. Source produit **df2870f5776d20213bcb8c6c88a5bcb967091e97**, branche `codex/fix-next-posters-scroll-matching`, base99afef9. Aucun tag ni bundle antérieur remplacé.

## Corrections et preuves

- **Enchaînement** : retour humain du candidat94495c1 conservé comme échec réel. Miroir SQLite local lu sans mutation :150 épisodes de la série concernée sans saison/épisode,150 préfixes uniques dans un dossier. Repli numérique strict sur fichiers, pas de tri alphabétique arbitraire ou de numéros écrits. Doublons, années et groupes ambigus refusés ; `order_unknown:true` distingue l'absence d'ordre d'une vraie fin. API2026-10-14 et v1 inchangées hors champ de réponse facultatif,3 champs historiques et ordre saga/liste conservés. La règle courte approuvée du lot précédent est incluse côté serveur, non active sur le Pi tant qu'il n'est pas mis à jour.
- **Fiche sans match** : ancienne cible retirée de l'observer, token/identité invalidés et image vidée avant d'attendre les données. Tests des anciens événements/réponses tardives et des affiches personnelles sansTMDB.
- **Clipping** : cartes et images encore visibles réutilisées au déplacement de fenêtre, images décodées non vidées à chaque sortie d'intersection ; géométrie réelle des lignes alignée. Rendu toujours borné, cartes invalidées sur modification du contenu/ordre/filtre.
- **Matching** : catalogue entier2496 entrées parcouru en lecture seule,19 titres à parenthèses orphelines et90 à parenthèses complètes. Nettoyage partagé par analyse du fichier et recherche automatique d'un ancien titre, parenthèses complètes conservées. Aucune requête fournisseur réelle, modification de titre manuel ou de fichier NAS, reconnaissance forcée globale ou rematching automatique lancé.
- **Course de démarrage trouvée pendant les tests** : catalogue rapide rendu avant installation des hooks d'affiches ; reproduction20cartes/0cible/0appel, corrigée en attendant les modules différés, sans temporisation arbitraire.

Vérifications sur cette source :

- 61/61Node ciblés : player.dom, detail-lifecycle, collection-retention, catalog-loading.dom, collection-pages, series-layout, catalog-window, collection-cover-and-profile.
- 68/68domaine ;11/11scanner ;13/13filtre metadata ;27/27serveur filtre playback ;24/24web bridge/player inchangés.
- Contrat API76chemins et diff check PASS. Les deux anciens échecs globaux `player-qa` ne sont pas requalifiés ni annoncés corrigés.
- Vrai DOM connecté Chromium avec pont synthétique, `scripts/test-catalog-virtual-artwork.cjs` :2496affiches, premières images sans forcer de recharge, défilement aller/retour catalogue et collection, identité des cartes chevauchantes/images inchangée, moins de50cartes, aucune erreur page PASS. `scripts/test-filters-v05.cjs` PASS. Aucun replay général du lecteur ni nouvelle lecture réelle.
- [Revue indépendante](../reviews/next-posters-scroll-matching-df2870f.md) APPROVED_SCOPED. Contrôle restreint des ajouts : aucun bloc certificat/clé privée, adresse du foyer ou affectation suspecte de secret détecté ; ce n'est pas un nouvel audit de tout l'historique privé ni une autorisation de publication.

## Livrable Mac

Utiliser **`artifacts/macos/Chill Center-test-df2870f.app`** du dépôt canonique. Interne0.5.1/build6,81Mio,48dylibs arm64, signature ad hoc profonde/stricte et fermeture des dépendances PASS. Exécutable SHA256 **20316cb655d45327c283184fab3c1675c3c0f9d99a6fb2a3b423c7e61e47d182**. Packaging release hors ligne puis `--verify-bundle` sur le bundle publié : libmpvAPI2.5 PASS, sans ouverture de fenêtre ni lecture. Deux avertissements natifs `dead_code` existants conservés.

Commande : `CARGO_HOME=/private/tmp/video-catalog-cargo CARGO_TARGET_DIR=/private/tmp/chillcenter-series-target.ZXNxst CARGO_NET_OFFLINE=true bash scripts/package-macos.sh <nouvelle-destination-test-df2870f.app>`.

Bundle0.5.1 figé toujours SHA **815c2a0c76e70c533fa909fcd6fc9d23fe72e8eb0ca4cd1b5e978ff783e35799**,0.5 et tous les candidats précédents conservés. Retour arrière client : rouvrir le précédent bundle, pas de restauration de base. Pas de nouveau numéro de release implicite, aucun push GitHub ni changement de branche canonique active/chantier web.

## Checkpoint de compilation : déploiement alors bloqué par SSH

Binaire immuable conservé dans `artifacts/linux/chill-server-only-df2870f/` du dépôt canonique, avec manifeste exact, sans assets web/configuration/unité. SHA256 **14d84a0478b9f394bfe59b3b88588edb4a9305dfedfc0fb0b66449b39d034169**, ELF64aarch64 validé. Reconstruction sur source figée : `cargo zigbuild --release --locked --offline -p chill-server --target aarch64-unknown-linux-gnu.2.28`, Rust/Cargo1.99 existants ; même hash après reconstruction. Avertissement existant du linker conservé. Pas d'installation induite par cette compilation.

Le socket SSH partagé demandé n'est pas présent. Aucun arrêt, sauvegarde distante, upload, bascule ou sonde Pi/NAS n'a donc été réalisé. Dernier état **historiquement vérifié** : sourcef3fdac5 /SHA d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90 ; ne pas en déduire qu'il s'agit d'une mesure actuelle. Avant toute bascule : comparer à nouveau le binaire/processus courant et les empreintes web/configuration/unité/TLS/montage, arrêter en cas de livraison concurrente non attribuée.

Le candidat conserve les fonctionnalités web existantes. L'opérateur serveur seul, revu indépendamment, exige les adresses à l'exécution, garde les assets web/configuration/NAS inchangés, sauvegarde hors ligne avant publication atomique, conserve l'ancien binaire et restaure seulement celui-ci si échec. Aucun script privé ni donnée de connexion commité. Pas de migration SQL ; états vu déjà enregistrés non effacés par rollback.

Restent : bascule réelle avec sauvegarde privée, vérification indépendante processus/API datée/v1/web inchangé, réponse suivante001→002 sur le catalogue réel ; puis validation humaine Tauri du défilement, des fiches vierges et du compte à rebours. Les tests DOM/unitaires ne prouvent ni ces résultats natifs ni la fluidité réseau Pi/NAS. BUG0081–0084 demeurent ouverts.
