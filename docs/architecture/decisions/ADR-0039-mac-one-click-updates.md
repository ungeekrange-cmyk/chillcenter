# ADR0039 — Mise à jour Mac au clic et vérification au lancement

8 novembre 2026. ACCEPTED_BY_USER_CHANGE_REQUEST, implémentation candidate.
Supersedes : ADR0029, uniquement la planification et la seconde confirmation
du client Mac. Les politiques des composants serveur/web/ambiances demeurent.

Demande explicite : version installée, Vérifier maintenant, Installer ; contrôle
en arrière-plan à chaque ouverture ; petite icône près de Configuration lorsqu'une
version est disponible ; un clic installe et relance sans dialogue supplémentaire.

Le backend lance une vérification Mac détachée du démarrage, indépendamment de
l'ancienne politique locale. Aucune installation Mac planifiée. L'interface
lit l'état local avant d'interroger le serveur, sans verrou global de navigation.
Le bouton de la carte et l'icône partagent une seule action en cours. Les erreurs
restent visibles et permettent de réessayer. La configuration générale de
l'application reste accessible ; seul le formulaire de politique Mac disparaît.

Le clic est le consentement d'installation. Rust conserve les contrôles de
lecture avant téléchargement et activation, le verrou entre processus, les URL
fixées, les signatures, l'identité/version signée, l'expiration, les reçus exacts
et le compteur anti-retour arrière. Une éventuelle autorisation imposée par
macOS sur un dossier protégé reste sous le contrôle du système.

Le même verrou OS couvre désormais la vérification entière. Sous ce verrou,
une ancienne vérification Pending/Running peut être marquée CHECK_INTERRUPTED
et relancée, sans effacer l'historique ni les reçus d'installation. Un worker
vivant empêche la récupération concurrente. Voir [BUG0138](../../debug/BUG-0138-mac-update-orphan-check.md).

Les métadonnées ont un budget de 30 secondes par requête ; le paquet Mac de
plusieurs dizaines de Mo dispose de 15 minutes. Toutes les opérations réseau
restent hors du thread d'interface et des verrous du catalogue/lecteur.

Cette décision n'autorise pas une nouvelle publication GitHub ni le remplacement
de l'application personnelle pendant les tests. La vérification native utilise
une copie de bundle sous le dossier QA déjà existant et les paquets publics.
