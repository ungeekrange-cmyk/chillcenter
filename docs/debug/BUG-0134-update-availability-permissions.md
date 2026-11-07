# BUG-0134 — Recherche web invisible et disponibilité illisible

2026-11-07 · Cause confirmée sur le serveur familial 0.7.2 ; correction source
sur `codex/updates-collection-management`, base `daba984` (non publiée).

Symptôme : « Vérifier maintenant » affiche un état transitoire puis revient à
l'état initial dans les clients Mac/web, alors que la mise à jour Mac fonctionne.
Attendu : résultat réseau explicite, puis proposition de la version web publiée.

1. Journal du helper : une première requête GitHub avait échoué au transport.
   Vérification en lecture : DNS et TLS ensuite valides, HTTP200 sur la même URL.
   Ce défaut externe transitoire n'est pas présenté comme corrigé par du code.
2. Nouveau contrôle web seul : reçu `available`, version0.7.4 séquence5 ; pourtant
   l'API expose `available:null`. Le snapshot est root:groupe-service, mode0600.
   Les fichiers de configuration pointent bien vers le même répertoire d'état.
   Cause : `.mode(0644)` est réduit par UMask0077 ; fchown ne restaure pas le droit
   de lecture groupe. Le serveur ne peut donc pas lire `available.json`.
3. L'interface de polling abandonnait aussi le rafraîchissement lorsque le focus
   restait dans le formulaire, masquant les reçus récents. Corrigé par mise à jour
   des états en place et conservation des champs de politique non enregistrés.

Correction durable : après fchown, fchmod0640 et fsync du seul snapshot UI avant
rename atomique. Reçus autoritaires, séquences, signatures et caches root restent
inchangés. Test avec création0600 vérifie mode0640, propriétaire et contenu.

Réparation ponctuelle exécutée : ouverture sans liens symboliques et contrôles
sur type, propriétaire/groupe, taille, schéma et contenu du snapshot existant,
puis fchmod0640/fsync ; inode, hash et contenu inchangés. Aucun redémarrage ni
installation. Relecture réelle API : web installé0.7.2, disponible0.7.4 séquence5.
Les diagnostics privés restent hors Git. Le helper installé est encore ancien :
un prochain contrôle peut recréer0600 tant que la correction durable n'est pas
livrée ; ne pas confondre réparation ponctuelle et livraison du paquet.

Régression : 7 tests de polling/interaction ; test Rust du mode restrictif et
suite chill-updates passent. Preuves détaillées du lot dans BUG-0135.
Rollback source : commit parent ; le mode0640 du résumé est le droit minimal
requis par le groupe service et ne change aucun secret.
