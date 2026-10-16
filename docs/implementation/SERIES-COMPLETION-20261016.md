# Séries et fin de lecture — candidat Mac, serveur non déployé

Source produit **94495c1ad330f664c1ebcaa85eccd223a242c712**, branche `codex/series-season-grid`, base b3ff2ba. Changements précédents de décor vidéo inclus ; aucune release/tag figé remplacé.

## Changements

- Entrée dans une série : première saison contenant des épisodes non vus, sinon première saison ; tri saison/épisode, saison inconnue à la fin. Actualisation de la même série conserve les plis manuels.
- Affiche à côté du synopsis, compteurs et cinq étoiles ; saisons et vraies grilles de cartes ensuite, outils communs TMDB/images/édition à la fin. Outils saison conservés.
- Proposition suivante existante préservée avec vignette, titre, transition de saison et délais 0/5/10/15/20/30/off. Dernier élément ou film seul : fermeture et retour à sa série/saga/liste/fiche. Annuler revient également. La fiche d'un membre transmet maintenant le contexte de sa collection à la lecture.
- Une synchronisation personnelle détachée par EOF confirmé ; annuler réutilise cette promesse. Retour différé abandonné si profil/serveur/navigation/lecture ont changé. Erreur décodeur ou réponse suivante absente n'est pas assimilée à une fin de collection.
- Exception utilisateur approuvée « Adapter la règle aux vidéos courtes » : durée <100s, fin atteinte après couverture réelle continue du dernier dixième, mesurée côté serveur dans le bail courant. Positions et temps monotone bornent la couverture, sans crédit de tolérance. Pause immobile conservée ; seek/rupture/buffering avec avance invalident. Nouveau bail/transfert/reconnexion = segment à relire pour cette exception. À partir de 100s, dix secondes réelles cumulées inchangées. Pas de nouveau DTO, champ, route ou migration SQL ; `final_watched_ms` n'est pas gonflé.

## Vérifications exécutées

- Node : **43/43** ciblés (`player.dom`, `series-layout`, `collection-pages`, `series-command`, `detail-lifecycle`, `collection-cover-and-profile`).
- Rust domaine : **65/65** ; serveur filtre `playback` : **27/27**, dont segment court aux sept vitesses, durées 1ms–99,999s, saut instantané/fragmenté, pauses, défauts et protocole HTTP sur v1/date. Test horloge client conservée : **1/1**.
- Client web existant : **24/24** tests bridge/player inchangés. EOF natif en lecture et EOF web en pause acceptés sur les payloads existants ; isolation de profil vérifiée par HTTP synthétique.
- DOM connecté avec pont fictif : `scripts/test-series-layout.cjs` PASS sur 1440px/760px, grille et ordre header/outils, replis et retour EOF avec compteurs actualisés. Captures `/private/tmp/chillcenter-series-evidence`, inspectées ; pas une preuve de rendu vidéo natif.
- `git diff b3ff2ba..94495c1 --check` PASS ; ajout de dépendance/runtime : aucun. Vérification restreinte des lignes ajoutées : aucun bloc clé privée/certificat, affectation suspecte de secret ou adresse IPv4. Ce contrôle ne remplace pas l'audit historique précédent ni n'autorise publication de l'historique privé.
- [Revue indépendante](../reviews/series-completion-94495c1.md) finale **APPROVED_SCOPED**. Les deux révisions défavorables restent consultables.

Les deux anciens échecs de la suite globale `player-qa` ne sont pas annoncés corrigés : pas de nouvelle certification de l'ensemble du produit. Aucun parcours vidéo réel rejoué, conformément à la demande de limiter les tests aux défauts.

## Remise et limites

Bundle de test créé et vérifié : `artifacts/macos/Chill Center-test-94495c1.app`, identité interne **0.5.1/build6**, **81Mio/48dylibs arm64**, signature ad hoc profonde/stricte et libmpv **API2.5** vérifiés par le script de packaging puis `--verify-bundle` sur le fichier publié. Exécutable SHA-256 **d642758aa39c12cca7aa8d6ed887a8b059897b956569c2f93e0b46dd65a343ba**. Compilation release/packaging PASS, deux avertissements `dead_code` natifs existants conservés. Application non ouverte, aucune lecture ou connexion réseau lancée par ce contrôle.

Commande : `CARGO_HOME=/private/tmp/video-catalog-cargo CARGO_TARGET_DIR=/private/tmp/chillcenter-series-target.ZXNxst CARGO_NET_OFFLINE=true bash scripts/package-macos.sh <destination-neuve-test-94495c1.app>`. Caches réutilisés, pas de nettoyage ni suppression. Captures de fenêtre 1440/760 également inspectées dans `/private/tmp/chillcenter-series-evidence-viewport` ; captures DOM, pas de certification vidéo AppKit.

Versions figées0.5/0.5.1 et anciens bundles conservés, pas de nouvel incrément de release implicite. SHA du bundle0.5.1 antérieur toujours **815c2a0c76e70c533fa909fcd6fc9d23fe72e8eb0ca4cd1b5e978ff783e35799**. Branche enregistrée localement dans le dépôt canonique sans changer sa branche active `codex/player-streaming`, son dossier `.claude/` ou le chantier web ; aucun push GitHub.

**Pi, NAS, configuration et web installé inchangés.** La nouvelle règle courte ne s'activera sur l'installation qu'après mise à jour du serveur depuis un candidat compatible web ; le bundle Mac seul ne peut pas l'imposer à l'ancien serveur. Aucun binaire Pi ni version active vérifiés dans ce lot. Avant déploiement futur : relever état courant/chantier web, sauvegarde privée, préserver les assets/config et mécanisme de rollback existant. Pas de déploiement implicite déduit des tests HTTP fictifs.

À vérifier humainement : rendu Tauri des grilles/fiche, retour natif à la fiche et actualisation vu réelle après activation du nouveau serveur. BUG0079–0084 restent ouverts, sans correction revendiquée.

Retour arrière client : bundle précédent, sans restaurer de base. Retour arrière serveur futur : ancien binaire suffisant (aucune migration), les états vu déjà enregistrés ne sont pas effacés automatiquement. Sources et toutes les révisions historiques préservées.
