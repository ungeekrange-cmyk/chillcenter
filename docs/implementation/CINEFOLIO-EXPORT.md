# Export portable Cinéfolio, sans modifier la source

Le mode CLI partage l'écrivain ZIP/manifeste du serveur. Il ne démarre aucun serveur Mac, ne scanne pas le NAS et ne lit pas de clé/jeton. Utiliser des chemins absolus sans liens symboliques, un cache d'images explicitement choisi et un **nouveau** fichier de sortie :

```sh
chill-server --export-cinefolio --source-db /chemin/cinefolio.sqlite --image-cache /chemin/cache-affiches --output /chemin/nouveau-export.zip
```

Pour exécuter cet outil localement sur le Mac, compiler `cargo build --release --locked -p chill-server` ; le service à déployer reste Linux uniquement. Ne pas utiliser les chemins NAS comme cache d'images. Une affiche locale extérieure au cache est refusée, jamais recherchée ou téléchargée automatiquement. Sur macOS, les alias `/tmp` et `/var` sont des liens : préférer leurs chemins `/private/tmp` et `/private/var` pour fixtures, ou un dossier utilisateur ordinaire.

La base0/1 est lue par SQLite Backup read-only, WAL inclus ; seule sa copie cohérente est convertie dans un store temporaire privé. Les références deviennent UUID, fiches/sagas restent communes, les notes/vus/playlists rejoignent un profil source « Cinéfolio » à affecter explicitement à l'import. L'ordre enregistré et les dates sont conservés. Affiches fournisseur en cache et images personnalisées sont copiées, jamais les vidéos. Les anciens réglages/tokens/chemins absolus ne sont pas exportés. Aucun import des données du foyer n'a été exécuté.

SQLite peut créer/utiliser ses fichiers de coordination `-wal`/`-shm` même lors d'une ouverture read-only d'une base WAL fermée. L'outil n'écrit aucune transaction source et conserve base/WAL actifs ; il ne supprime jamais ces sidecars, qui pourraient être utilisés par Cinéfolio. « Lecture seule » n'est pas une promesse d'absence de verrous SQLite ou de sidecars.

Limites : snapshot256MiB, données JSON64MiB,100000 médias,1000 collections,1000 membres par playlist/saga convertie, images8MiB/4096px validées, cache copié1GiB maximum, ZIP1GiB/20000 entrées. Le fichier est publié par hard link sans remplacement, mode0600, après fsync. Une erreur de fsync du parent après publication conserve le fichier et signale la durabilité incertaine : vérifier sa présence avant nouvelle tentative. Le staging ne supprime que les chemins créés par cet outil ; fichiers inattendus conservés.

Puis, dans Administration Tauri : Importer une archive → aperçu → correspondances complètes/injectives des profils et préfixe relatif → politique de conflits → préparer → confirmer. Une archive refusée ne change pas le catalogue ; un reçu export+mapping rend sa répétition idempotente. L'import est distinct d'une restauration remplaçante offline.

Preuve exécutée : base de fixture avec WAL actif, affiches fournisseur, dates et état personnel → ZIP → validation/import vers un profil explicitement choisi. Octets base/WAL source inchangés ; image hors cache et destination symlink/existante refusées. Le test du vrai binaire CLI et les régressions archives sont enregistrés dans le journal de remise avec le SHA courant ; ne pas confondre avec une migration réelle ou un test Pi.

Tests locaux exécutés du lot : `cargo test --offline --locked -p chill-server`60 tests lib PASS, puis `cargo test --offline --locked -p chill-server --test legacy_cli`2/2 PASS après correction de l'attente du harnais concernant les sidecars read-only SQLite. `legacy_export::tests`2/2 inclus dans60 : source/WAL actifs exacts, image fournisseur et import typé. Import/export partagent désormais un vrai permis de travail lourd et2GiB de staging cumulé (incluant orphelins/plans/snapshots/réservations), test croisé d'inspection annulée empêchant le départ export et test du quota partagé PASS. Les requêtes ZIP ne peuvent libérer prématurément un worker encore occupé.
