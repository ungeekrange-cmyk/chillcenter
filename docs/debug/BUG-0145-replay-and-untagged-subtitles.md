# BUG0145 — Relancer une vidéo vue/terminée et sous-titres sans langue

20 novembre 2026, correctif sur `codex/playback-restart`. Sources candidates,
non encore publiées : serveur0.7.8 et web0.7.11 ; Mac0.7.8 reste compatible.

Symptômes : Lecture reprend une ancienne position d’un film marqué vu ; à EOF,
le lecteur web ne peut pas amorcer le buffer. Avatar Seven Havens S01E01 ne
sélectionne pas ses sous-titres. Pi encore web0.7.9 au signalement, puis web0.7.10
installé et ses90 ressources servies vérifiées avant ces investigations.

Causes confirmées : grant brut inchangé malgré watched=true, absence de garde
EOF web, et sélection uniquement par langue alors que l’épisode possède une
unique piste S_TEXT/UTF8 sans langue (`und`, défaut0). Reproduction avant/après
par plages SSH bornées et Chrome isolé, sans bail ni écriture du profil du foyer.
Audio E-AC-3 converti en AAC chez le client, vidéo HEVC native, aucun transcodage
serveur. Ne pas inférer une langue à partir du nom du fichier.

Correction : lecture fraîche reçoit une copie de PersonalState avec position0
si watched ou fin mesurée atteinte (250ms, aucun seuil90%). Durée lue par une
requête indexée profil/média ; absence de durée conserve la reprise héritée.
L’état durable, la note, le compteur et le statut vu restent inchangés.
Une continuation transférée conserve sa position ; une autre cible est une
nouvelle lecture. Fallback web pour serveurs précédents et garde EOF après
lecture du conteneur. Préférence de sous-titres active : unique piste sans langue
sélectionnée ; Off, langue connue différente et ambiguïté restent respectés.

Preuves privées sous `/private/tmp/chillcenter-web-wake.yBYp2S/` :
`avatar-baseline-browser.jsonl` (piste désactivée), `avatar-fixed-browser.jsonl`
(piste native showing, cue active56.39–59.227s, vidéo avance),
`project-restart-browser.jsonl` (position initiale9382.421s ramenée au début,
lecture2.58s sans erreur). Aucun dialogue conservé dans ces preuves.
87 tests domaine et163 serveur réussis (1 ignoré préexistant),341 JS et28 docs
réussis ; Clippy strict et formatage des fichiers Rust modifiés PASS. Revue finale
indépendante en cours après correction de ses deux findings sur les transferts.
Tests physiques iPhone/Pixel non réalisés ; lecture réelle dans l’application Mac
non lancée pour préserver le profil. Contrat commun testé sur fixtures isolées.

Retour arrière : anciens slots et sauvegardes du helper conservés ; aucune
migration de schéma ni modification NAS/certificat/identité. Utiliser le mécanisme
de récupération officiel, sans abaisser les séquences manuellement.
