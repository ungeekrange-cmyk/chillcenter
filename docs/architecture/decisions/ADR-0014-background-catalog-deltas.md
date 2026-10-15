# ADR-0014 — Synchronisation détachée et deltas de catalogue

- Statut PROPOSED ; auteur/propriétaire `/root`, 2026-10-15 ; revue indépendante PENDING.
- Branche `codex/library-ux-sync-matching`, base `a59f5e6`.
- Supersedes ADR-0013 uniquement sur transfert complet et contrôle synchrone ; cache privé et isolation conservés. Superseded by NONE. Liens BUG-0064/0065 ; API 2026-10-14 additive.

## Contexte et options

L’utilisateur demande de naviguer pendant le contrôle et de transférer les modifications. Le client précédent détenait son verrou pendant HTTPS, désactivait les contrôles et effaçait la grille. Masquer le chargement sans changer ces mécanismes ne résout pas la cause. Un journal exhaustif de mutations/SSE est possible mais ajoute une politique de rétention/reprise disproportionnée au foyer/Pi 3.

## Décision

`POST /profiles/{profile}/catalog/changes` compare le curseur à deux époques SQLite persistantes. Inchangé : petite réponse sans matérialiser le catalogue. Modifié : lecture cohérente, différences serveur, transfert immuable paginé. Curseur inconnu/évincé : `reset=true`, copie complète cohérente sur le même protocole. Points de comparaison persistants : identifiants opaques et SHA-256 de chaque élément projeté (y compris vu/note), quatre maximum par profil et 16 Mio globalement. Pas de copie des métadonnées dans ces checkpoints. Cache mémoire : une représentation immuable par transfert, trois transferts actifs et 32 Mio sérialisés globalement maximum, expiration 120 s ; un quatrième est refusé avant persistance plutôt que d’évincer un transfert actif. Une réponse complète en une page ne réserve rien ; la dernière page libère son créneau. Perte de cette réponse : reprendre le POST après le 404 éventuel, ancien miroir intact.

Une génération privée persistante, `catalog.sync-generation`, complète les époques dans le curseur opaque. Un redémarrage normal la conserve. La restauration officielle hors ligne publie un nouvel arbre sans ce marqueur (il est volontairement exclu des sauvegardes) : une nouvelle génération empêche les collisions d’époques après restauration. Restaurer SQLite en place en conservant ce marqueur n’est pas une procédure supportée ; employer `--restore`/`--recover`.

Le client prépare le travail sous verrou puis effectue HTTPS sans verrou ni sémaphore de navigation. Construction en O(n), publication du miroir SQLite sur une connexion détachée après réception/validation de toutes les pages ; CAS identité/contexte/révision/génération de mutation, sonde d’annulation jusqu’au commit, résultat périmé ignoré. Le verrou UI ne publie ensuite que le manifeste en mémoire, sans réouvrir SQLite ni parcourir les images. Révision inchangée : aucun travail disque de publication. Une mutation survenue après commit garde le marqueur dirty pour la synchronisation suivante. Grille conservée jusqu’au remplacement complet, indicateur discret. Les recherches et sélections TMDB sont également détachées. Une première copie froide peut encore être nécessaire ; lectures chaudes locales.

Ancien serveur : repli uniquement sur HTTP 404 vers les instantanés existants, toujours hors verrou. API datée et alias v1 conservés. Aucun changement TLS, authentification, NAS ou transcodage.

## Conséquences et retour arrière

Moins de réseau ; une modification exige encore une lecture/comparaison serveur. Rétention disque bornée, éviction causant un reset explicite. Aucun octet vidéo ni coût cloud. Tests delta/éviction/réouverture/CAS/cache sans réseau et navigateur synthétique avec synchro retardée ; RSS/latence Pi restent à mesurer.

Migration additive (points de comparaison/métadonnées de saison). Ancien binaire ignore ces tables. Retour c610ea9 après sauvegarde privée possible sans supprimer les données. L’ADR ne vaut pas preuve de déploiement. La demande utilisateur autorise cette correction ; revue technique et preuve liées au SHA final restent requises avant déclaration de livraison.
