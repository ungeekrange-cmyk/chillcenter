# BUG-0088 — Une série numérotée seulement par ses fichiers ferme le lecteur

16 octobre 2026. **FIXED_LOCAL — déploiement Pi autorisé, non encore effectué.**

Retour humain : à la fin du premier épisode d'« Avez-vous déjà vu », le lecteur du candidat94495c1 se ferme au lieu de proposer le suivant. Cette preuve invalide la validation réelle de l'enchaînement, même si les tests de compte à rebours synthétiques avaient passé.

H1 confirmée sans lire une vidéo : miroir local parcouru en lecture seule,150épisodes de cette série,150sans saison/épisode, un seul dossier,150préfixes ordinaux uniques dans les noms. Le serveur renvoyait `media:null` pour toute numérotation absente. La nouvelle fermeture terminale confondait ce résultat avec le dernier épisode.

Correction Rust : utiliser les ordinaux explicites uniques du fichier quand tous les épisodes sont dans le même dossier plat, sans saison connue ; ne pas inventer/écrire de saison ni changer de titre ou lancer de scan. Année en tête, doublon ou absence de numéro = refus de deviner. Suivant indisponible conservé sans saut silencieux ; vrai dernier élément = fin.

Ajout de réponse facultatif `order_unknown:true` à NextVideo : ordre indéterminé, pas fin confirmée. Absent équivaut à false, trois champs historiques inchangés, API2026-10-14 et aliasv1 maintenus, clients anciens ignorent l'extension. Le Mac garde le lecteur avec une explication si l'ordre est inconnu, sans lancement ni fermeture automatique. Sagas/listes gardent leur ordre explicite ; leurs tests de contexte, disponibilité et fin restent concernés.

Preuves : cas001→002→010etdernier, prochain manquant, doublons/année/titres sans numéro, absence de réécriture, sérialisation ancienne inchangée, contrôleur JS ambigu sans fermeture. Résolution du cas réel par API et lecture humaine à confirmer après déploiement ; pas de succès Pi/NAS déduit du miroir.
