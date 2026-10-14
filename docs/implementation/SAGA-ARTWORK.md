# Affiches de saga — US011-AC03

Le client ne transmet jamais une URL ni un chemin d'affiche. Il emploie deux
commandes IPC fermées : `saga_artwork` lit l'affiche via la route profilée et la
convertit en JPEG data URI borné par le helper Rust existant (360×540,
512 KiB) ; `saga_member_artwork` envoie seulement l'identifiant de saga, la
révision attendue et l'UUID du média.

Le serveur copie alors l'affiche déjà sélectionnée du membre vers le cache
immuable de la saga à travers `PUT /sagas/{id}/poster/member`. Cette copie reste
disponible si l'image source est ensuite retirée. La révision est un CAS : le
client ne rejoue pas une requête en conflit et recharge explicitement la saga.

Une saga vide peut toujours recevoir une image personnelle. Pour un membre avec
une affiche fournisseur (`poster_custom:false`), l'interface présente le choix
« Utiliser l'affiche TMDB du film » ; une affiche personnelle est nommée comme
telle. Aucun fournisseur externe n'est appelé par ce parcours.

La recherche TMDB de la fiche média possède un champ éditable borné à 500
caractères, validé de nouveau par la commande Rust. Preuves locales :
`node ui/connected/tests/saga-artwork.test.js` et les tests unitaires
`chill-client`. Vérification avec fournisseur réel, macOS et Pi : `NOT_RUN`.
