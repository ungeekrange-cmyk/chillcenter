# BUG-0012 — Crédit de visionnage à vitesse ×0,5

2026-10-13 · VERIFIED · auteur /root · base84b08b1645a0c6fbccd17012eb7cbcd0fc7ad74a.

Défaut confirmé du store : plafond du crédit final fixé à `duration_ms / 10`, en durée vidéo plutôt qu'en temps réel. À ×0,5, les derniers10% d'une vidéo de100s prennent20s ; un événement validé par l'horloge serveur était refusé.

H1 unique : plafond indépendant de la vitesse minimale. Test `validated_slow_playback_credit_is_real_time_not_video_duration` RED : « Crédit de visionnage final invalide. » ; même test GREEN après correction. Plafond défensif désormais égal à deux fois la durée vidéo du dernier dixième, vitesse minimale ×0,5. Le serveur conserve le contrôle strict par horloge monotone, séquence, position et vitesse ; aucune confiance supplémentaire dans le client. Frontière90% partagée sans multiplication débordante, test jusqu'à u64::MAX. Tests serveur supplémentaires ×0,5/×1/×2 et passage proratisé de la frontière.

Le seuil de marquage vu reste10 000ms réels cumulés dans les derniers10%, sans pause/seek/buffering. Pas de migration ni modification des positions existantes. Rollback : restaurer le code de la base sur une nouvelle branche ; les données restent lisibles.
