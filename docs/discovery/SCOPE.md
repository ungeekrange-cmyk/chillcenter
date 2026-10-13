# Périmètre fonctionnel et journal de discovery

Révision `CC-FONC-002` ; statut `APPROVED_BY_USER` ; date 2026-10-13. Origine : échanges de ce chat, repris ci-dessous sans inventer une recherche concurrentielle. Les propositions de l'assistant sont distinguées des corrections de l'utilisateur. L'approbation de ce périmètre ne vaut pas approbation de l'architecture.

| ID | Exigence validée | Preuve utilisateur / disposition |
| --- | --- | --- |
| F-01 | Serveur Linux, compatible Pi 3 B/B+, maximum de Rust, consommation maîtrisée. Pas de serveur Mac. | Demande initiale puis « vise directement un serveur sur linux […] le reste ok ». |
| F-02 | Client Mac Apple Silicon en premier ; sélectionner le serveur DNS ou IP. Client web reporté. | « Version mac silicon en premier » ; inchangé dans la validation finale. |
| F-03 | NAS accessible au serveur seulement ; fichiers fournis au client, aucun transcodage serveur. | Demande initiale ; NAS 1 Gb/s et Pi probablement en Wi-Fi. |
| F-04 | Deux ou trois utilisateurs simultanés maximum, essentiellement 1080p ou moins. | « Deux ou trois utilisateurs simultanés max, surtout 1080p voire moins ». Cible de performance, pas mesure acquise. |
| F-05 | Créer, choisir, changer et administrer les profils. Première ouverture : choix ; suivantes : dernier profil de l'appareil et du serveur. Sélecteur en haut à droite. | Demande initiale ; répartition des réglages approuvée par « oui ». |
| F-06 | Reprise synchronisée par profil ; notes, vu/non vu et playlists personnels. | « dernier moment de visionage […] synchronisé entre les appareil » ; « notes par user ». |
| F-07 | Une lecture active par profil ; transfert sur confirmation, ancien lecteur en pause. Profils différents indépendants. | « deuxième choix » parmi lectures simultanées ou transfert d'une lecture unique. |
| F-08 | Marquer vu après au moins 10 secondes de lecture effective dans les derniers 10 % de la vidéo. Saut seul insuffisant. | « 90% c'est quand la personne regarde au moins 10 secondes dans les 10 derniers % ». Remplace la proposition de couverture de 90 % du contenu. |
| F-09 | Lecteur intégré : audio/sous-titres, +30 s, −10 s, barre navigable, vitesses 0,5/0,75/1/1,25/1,5/1,75/2. | Demande initiale. |
| F-10 | Vitesse initiale ×1 ; dernière valeur uniquement pour la série et le profil. Audio et sous-titres anglais par défaut. | Correction détaillée de l'utilisateur ; paramètres de lecture par profil approuvés. |
| F-11 | Anglais manquant : audio original si identifié, sinon première piste ; sous-titres désactivés ; signalement discret et sélection manuelle. | « oui » à cette proposition précise. |
| F-12 | Lecture suivante selon contexte : playlist, saga ou série ; fiche d'un film isolé sans enchaînement. Nouvelle saison signalée ; indisponible = message et aucun lancement. | Réponses « oui » ; « saison suivante comme un nouvel épisode […] nouvelle saison ». |
| F-13 | Afficher vignette/titre suivant et deux boutons. Délai 0/5/10/20/30 s ou aucun lancement automatique ; défaut 10 s ; réglage par profil. | « oui » à la correction de 19 en 10 secondes et à l'absence de compte à rebours si désactivé. |
| F-14 | Catalogue, scan, TMDB, séries/saisons, métadonnées manuelles, affiches, filtres, recherches, notes, sagas et playlists conservés. | Demande de partir de Cinéfolio ; synthèse validée. |
| F-15 | IA conservée, bouton plus petit ; prévisualisation/confirmation avant application. Pas de dictée. | « pas la dictée mais l'IA oui, bouton plus petit néanmoins ». |
| F-16 | Historiques conservés après déplacement/renommage ; stockage déconnecté pris en compte ; sauvegarde/restauration. | « Bibliothèque : oui ». L'architecture doit expliciter les limites d'identification de fichiers déplacés. |
| F-17 | Export/import d'informations depuis Administration, format choisi par le projet. Archive JSON versionné + affiches ; vidéos et secrets exclus. | « […] sous un format que tu conviens (json ou autre) […] import […] administration » ; synthèse validée. |
| F-18 | Usage au foyer sur LAN, sans authentification ; administration accessible à tous. Suppression d'un profil confirmée, données du profil seulement. | « utilisation sur réseau local au sein du foyer » ; « oui l'organisation me convient ». |
| F-19 | HTTPS ; acceptation initiale ; certificat autogénéré, renouvelé tous les ans, validité 18 mois ; nouveau certificat automatiquement accepté après expiration de l'ancien. | Demande initiale ; risque signalé puis « je sais, mais réseau local au sein du foyer ». Pas d'autorisation de désactiver TLS globalement. |
| F-20 | Modes système/clair/sombre ; ambiances saisonnières et événements, variantes claire/sombre ; automatique, manuel persistant, désactivation ; propres à l'appareil. | Demande et approbations « oui ». |
| F-21 | Hémisphère Nord : saisons le 1/03, 1/06, 1/09, 1/12 ; Halloween 1–31/10 ; Noël 25/11–25/12 ; retour saison le lendemain. Visuels statiques. | « oui » à l'ensemble du calendrier proposé. |
| F-22 | Formats vidéo prioritaires MKV, MP4, AVI ; sous-titres intégrés et externes. | « MKV, MP4 et AVI essentiellement » ; « […] les deux cas, intégrés et fichier séparés ». |

## Supersessions et exclusions

- `CC-FONC-001` contenait un serveur Mac : remplacé par `CC-FONC-002` à la demande de l'utilisateur.
- Marquage au simple lancement (héritage Cinéfolio) : remplacé par F-08.
- Marquage par 90 % de contenu réellement visionné (proposition) : rejeté au profit de F-08.
- Lecture simultanée du même profil : non retenue ; F-07 s'applique.
- Délai de 19 secondes : corrigé en 10.
- Dictée : exclue de la première version. IA : incluse, aucune exécution locale de modèle requise.
- Client web : ultérieur. Transcodage serveur, authentification, exposition Internet, serveur Mac : hors périmètre.

## Interprétations techniques proposées à l'étape 2

Ces détails ne sont pas de nouvelles réponses de l'utilisateur : temps de visionnage final cumulé en secondes réelles, reprise persistée environ toutes les 5 s, bail de lecture renouvelable, calendrier évalué dans le fuseau de l'appareil, import avec correspondance explicite des racines NAS. Leur validation est celle de `CC-DOC-001`.
