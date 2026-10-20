# Contrats proposés

Révision `CC-DOC-001`, `ACCEPTED` dans [G-02](../gates/G-02-CC-DOC-001.md). Statut de gouvernance actualisé, contrats inchangés ; ce document définit les frontières nécessaires aux stories, pas une API déjà disponible.

## Données

| Entité | Identité / clés | Portée |
| --- | --- | --- |
| `server` | `server_id`, version API, dates TLS | Persistant ; distinct de l'adresse réseau. |
| `media` | UUID, localisation sous racine, type, métadonnées, disponibilité, révision | Catalogue commun ; pas de `watched` ni `rating` global. |
| `series` / `saga` | UUID, métadonnées, membres ordonnés, révision | Commun. |
| `profile` | UUID, nom, avatar facultatif, révision | Commun et sélectionnable sans secret. Avatar prédéfini ou import borné. |
| `profile_media_state` | PK `(profile_id, media_id)` | Reprise, vu, note, compteur de visionnage final, dernière session/seq acceptée. |
| `profile_preferences` | PK `profile_id` | Audio, sous-titres, délai d'enchaînement. |
| `series_playback_preferences` | PK `(profile_id, series_id)` | Dernière vitesse, défaut ×1. |
| `playlist` / `playlist_member` | UUID + propriétaire ; positions et révision | Personnel ; unicité du nom par profil ; ordre atomique. |
| `playback_lease` | `session_id`, `profile_id`, `device_id`, `media_id`, `media_revision`, génération, échéance, epoch serveur | Volatil ; aucune restauration après redémarrage/import ; association au média courant contrôlée pour chaque flux/événement. |
| `local_client_settings` | appareil + `server_id` | Serveurs approuvés, dernier profil, apparence, configuration IA locale. |

Contraintes FK, transactions atomiques, validation des vitesses et délais ; note 1 à 5 ou absente. Supprimer un profil retire ses états et playlists, révoque son bail et laisse catalogue/sagas/vidéos intacts. Les notes série/saison gardent le comportement Cinéfolio mais seulement pour le profil et les épisodes explicitement présents ; une note commune de série n'est pas créée.

## API HTTPS v1

| Groupe | Opérations prévues |
| --- | --- |
| Santé et identité | `GET /api/2026-10-14/server`, santé de service et disponibilité séparées ; aucun secret. |
| Profils | Lister, créer, modifier, supprimer ; suppression confirmée dans le client. |
| Catalogue | Pagination, recherche, filtres, fiches, séries, sagas ; métadonnées modifiables avec révision attendue. |
| Personnel | `/profiles/{profile_id}/...` : états, préférences, notes, playlists ; profil explicite à chaque opération. |
| Lecture | Création/acquisition, transfert confirmé, heartbeat, progression, pause/stop, événements de contrôle SSE. |
| Fichiers | Vidéo Range, affiches et sous-titres indexés ; aucun chemin libre ni route générale du filesystem. |
| Administration | Racines, scans, TMDB, export/import, conflits d'identité et état des tâches. |
| IA | Snapshot borné pour le profil et validation/application d'un plan ; aucun exécutable distant. |
| TLS | Certificat public courant et calendrier de renouvellement ; jamais clé privée. Le handshake valide le pin SPKI sans endpoint de rotation préalable. |

DTO versionnés partagés avec le client Rust ; la compatibilité inclut `api_version` et des capacités. Erreurs JSON : `code`, message français, identifiant de requête, sans détail sensible. `409` pour conflit de révision/bail, `404` pour identifiant absent, erreur d'indisponibilité distincte d'une suppression, `413` pour dépassement de limites et `416` pour plage invalide. Les endpoints de fichier vérifient le bail de lecture actif ; un identifiant de session est une coordination, pas une authentification utilisateur.

Administration sans auth ne signifie pas exposer des écritures à n'importe quel site web : pas de CORS permissif, contrôle de Host/Origin et types de corps, capacités Tauri limitées aux commandes nécessaires, pas de navigation arbitraire dans la webview. Ces défenses ne transforment pas le LAN en réseau authentifié.

## Transfert de lecture

1. B demande un bail pour P ; A le possède : réponse `409 PLAYBACK_ACTIVE`, aperçu de la lecture et jeton de confirmation à durée limitée lié à cette génération.
2. B confirme ; serveur vérifie ce bail et sa liaison média, passe en `TRANSFERRING`, fige `transfer_deadline` à l'échéance actuelle, refuse tout renouvellement de A et les demandes concurrentes, puis ordonne sa pause.
3. Les anciens flux sont annulés. A arrête son lecteur, transmet sa position finale avec sa séquence, média/révision, et accuse réception. Le serveur accepte un seul ACK final pour cette génération avant `transfer_deadline`, puis clôt atomiquement le bail ; après cette date ou clôture, même l'ACK est refusé.
4. ACK reçu : transaction de clôture de A et activation de B à la position enregistrée. Absence d'ACK : attendre `transfer_deadline`, que les heartbeats retardés ne prolongent jamais. A doit se mettre en pause localement avant cette échéance ; B reste en attente.
5. B reçoit une nouvelle génération et démarre. Tout événement/flux de A est désormais refusé. Deux confirmations concurrentes ne peuvent pas activer deux successeurs.

Délais mesurés avec une horloge monotone. Le client calcule une échéance conservatrice à partir de la durée restante annoncée ; suspension/réveil force une vérification avant de jouer. `playing → paused/released`, `playing → transferring → released`, `playing → expired`, `released → acquire` ; aucune lecture nouvelle avant réponse du serveur.

Toute acquisition lie le bail au média et à sa révision. À l'enchaînement, la position finale du précédent est enregistrée avant sa libération ; la vidéo suivante acquiert une nouvelle génération atomiquement liée à son UUID/révision. Tous les flux/événements du précédent sont refusés même s'ils portent le bon profil ou un ancien `session_id`. Pendant le compte à rebours, une acquisition par un autre appareil annule l'autoplay ; le transfert n'est proposé que sur confirmation. Le contexte playlist/saga/série est conservé séparément du bail.

## Progression et vu

Événement : `(profile_id, media_id, session_id, generation, sequence, position_ms, duration_ms, speed, playing, buffering, seek_marker, elapsed_playing_ms_in_final_tenth)`.

Incréments du compteur final calculés par le client à partir du temps monotone de lecture ; passage de la frontière 90 % proratisé. Le serveur contrôle durée, plage, vitesse autorisée, incrément maximal depuis le dernier heartbeat et séquence. Pauses, seeks et buffering interrompent l'accumulation. Progression conservée uniquement pour la génération active, plus l'ACK final prévu au transfert. Compteur cumulé proposé en ms ; passage à vu à 10 000 ms. Le marquage manuel non vu réinitialise ce compteur ; il ne supprime pas la reprise. Un stop proche de zéro enregistré volontairement reste une nouvelle position, pas une valeur à ignorer arbitrairement.

## Limites de départ

| Ressource | Proposition |
| --- | --- |
| Page catalogue | 100 éléments ; filtrage en base. |
| Corps JSON ordinaire | 1 MiB maximum, limites de champs réutilisées de Cinéfolio. |
| Affiche importée | 8 MiB, formats JPEG/PNG/WebP validés. |
| Sous-titre externe | 16 MiB maximum ; signaler les fichiers plus volumineux sans les tronquer. |
| IA | 3 000 fiches sélectionnées maximum, prompt ≤1 MiB ; limiter aux fiches pertinentes si catalogue supérieur. |
| Archive import | 1 GiB compressé, 2 GiB décompressé, 20 000 entrées, manifeste ≤64 MiB ; streaming sur disque de staging local, pas en RAM. |
| Export | Construire en flux vers disque/réseau, avertir d'un dépassement des limites d'import. |

Ces limites sont des propositions testables, pas des mesures du NAS. Pas de chemin absolu, `..`, lien symbolique, archive imbriquée ou entrée écrasant un fichier arbitraire à l'import. Checksums des affiches et intégrité du manifeste ; profil et racines mappés dans l'aperçu. Une limite dépassée conserve la base et l'archive originale.
