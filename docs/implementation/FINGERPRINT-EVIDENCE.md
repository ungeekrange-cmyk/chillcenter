# Preuves d'identité média — implémentation US026

## Contrat livré

Les empreintes, chemins absolus et jetons de version restent dans SQLite et dans le processus serveur. L'API conserve uniquement l'allowlist UUID et les états `queued`, `running`, `complete` ou `failed`, avec une force `partial` ou `full`. Aucun hash ni chemin n'est sérialisé vers le client.

Une empreinte partielle (taille et trois échantillons bornés) produit seulement un candidat. La réassociation automatique utilise exclusivement un SHA-256 complet déjà capturé, de même taille, unique entre une identité indisponible et la nouvelle identité disponible. Les protections CAS, révisions, profils, playlists et cible protégée restent celles de la transaction de réassociation existante. Une ambiguïté ne fusionne rien.

## Cycle de capture

Le scan déclenche un pump unique paginé par UUID, 16 entrées à la fois. Il ne charge pas le catalogue entier et avance son curseur même lorsqu'un fichier échoue. Une nouvelle invocation pourra retenter une erreur, au plus une fois par passage.

Avant d'inspecter une entrée, le worker invalide temporairement son éligibilité (`queued`/`running`). Il ouvre le fichier via la vérification de confinement existante, puis compare le jeton de version privé. Un jeton identique restaure la preuve sans lire le contenu. Un jeton différent lance une capture partielle et retire toute ancienne preuve complète. Le jeton contient uniquement un digest de métadonnées du descripteur et n'est jamais utilisé pour conclure que deux fichiers sont identiques.

Les captures partielles et complètes utilisent le même worker exclusif, des lectures positionnelles de 64 Kio, un plafond de 1 Mio/s, la pause pendant playback et l'annulation si la révision de racine change. Une capture complète explicite est prioritaire dès qu'elle est en attente ; elle ne contourne aucun de ces contrôles.

## Migration et restauration

`cc_media_fingerprint.file_version_token` est une colonne additive privée et nullable du schéma 3. À l'ouverture, une ligne historique avec `full_sha256` mais sans jeton perd seulement sa force complète ; son empreinte partielle et son historique sont conservés. La sauvegarde opérationnelle SQLite inclut naturellement cette colonne. Le transfert atomique de preuve pendant une réassociation conserve également le jeton, sans élargir les contrats d'archive/export.

## Limite mesurée

Le contrôle inchangé effectue un `open`/`stat` mais zéro lecture de contenu. Une modification entièrement apparue puis disparue entre deux observations ne peut pas être détectée sans surveillance de montage fiable ou hash continu, tous deux hors du contrat accepté. Le système ne transforme donc jamais le jeton de métadonnées en identité certaine.
