# ADR-0020 — Recommandations locales par titre et conseil IA regroupé

16 octobre2026. **IMPLEMENTED_DETAIL**, demande utilisateur explicite pour les deux surfaces (« Vidéos en relation » puis « en fait les deux »). Auteur `/root`. Complète ADR0007/0013/0015/0019 sans les remplacer ; pas de nouvelle API, fournisseur ou migration.

## Décision

Partager un regroupement Rust dans le miroir local du Mac : une série par identité TMDB ou groupe source, clés publiques opaques, pas de fusion au seul titre. Un UUID représentatif sert uniquement à lire l'affiche ; la carte ouvre la série entière, jamais cet épisode. Annuler saison/épisode seulement dans la représentation de conseil, pas dans la base. État vu de la série = tous ses épisodes vus, au profil actif ; disponibilité choisit un représentant disponible. Métadonnées communes de série prioritaires.

Fiche : score déterministe = nombre de genres, réalisateurs et acteurs communs distincts (trim/casse normalisés), ordre décroissant puis titre/UUID pour stabilité. Aucun résultat sur la seule proximité d'année ou appartenance à une saga. Exclure le titre source et toute sa série. Maximum60 résultats pertinents, tous accessibles dans un rail unique ; motifs réellement communs affichés sous les vignettes. Sans métadonnées correspondantes, pas de résultat inventé.

Conseil : à l'envoi explicite seulement, capturer les films et séries du miroir filtrés par catégorie/vu/disponibilité, sans nouvelle lecture réseau du catalogue. Écarter les titres sans genre/réalisation/distribution connue ; plafond3000 titres après regroupement/filtres, limite de prompt existante1Mio. Une série ne consomme plus autant d'entrées que d'épisodes. Fournir les genres, acteurs et réalisateurs disponibles au CLI existant sous identifiants temporaires, sans chemins, affiches, clé de groupe source ou secrets. Le modèle est instruit de fonder sa sélection sur ces métadonnées, d'expliquer les correspondances et de ne pas inventer les données manquantes. Au plus60 résultats ; dépassement refusé, pas de troncature silencieuse. Ordre du modèle conservé, doublons retirés, identifiants et absence de mutations validés en Rust.

## Conséquences et confidentialité

Tout le calcul catalogue est local et borné (miroir existant≤10000 médias) ; pas de scan, accès NAS, modèle surPi ou transcodage. Une brève capture SQLite a lieu sur worker bloquant sous le verrou de contexte avant l'exécution asynchrone du CLI. Conseil en lecture seule : son contexte local n'est jamais éligible à `ai/apply`. Les chemins relatifs restent privés vis-à-vis du fournisseur ; les crédits de réalisation et de distribution quittent le Mac uniquement après l'action de conseil, consentement UI précisé. Le mode organisation continue son flux de révisions/API existant ; son catalogue n'est pas enrichi avec les crédits locaux.

Le contrat HTTP2026-10-14/v1 et `GET .../related` historique restent inchangés pour le web/tiers. La commande IPC native `related_media` lit le nouveau calcul local ; `ai.start` accepte un booléen interne facultatif `recommendations` (défautfalse). Aucun fichier du client web installé ou de son worktree n'est livré/modifié. Pas de nouvelle dépendance/runtime ni nouvelle souscription.

## Validation et retour arrière

Tests de genres/acteurs/réalisation/classement, homonymes, absence d'épisodes et de réécriture, filtres personnels, vrai miroir sans serveur joignable, prompt whitelist/alias et refus d'application en recherche. Vrai DOM connecté synthétique sur les deux rails, défilement jusqu'au dernier résultat et navigation vers la bonne série. Fournisseur réel, qualité sémantique d'une réponse réelle et rendu Tauri restent des validations distinctes. Retour au bundle précédent suffit, sans restaurer de données ni changer le serveur.
