# BUG-0030 — pagination et invalidation des preuves fingerprint

## Symptômes confirmés

Deux défauts indépendants affectaient la preuve privée de réassociation de US026.

1. Le démarrage post-scan ne chargeait qu'une page de 16 identités. Aucun rappel ne chargeait la page suivante. Une erreur dans cette première page pouvait donc aussi maintenir les mêmes lignes candidates au début du scan suivant.
2. La base conservait un SHA-256 complet avec seulement le chemin, la taille et l'empreinte partielle. Une nouvelle version physique au même chemin, de même taille et dont la modification était hors des trois échantillons, pouvait réutiliser à tort l'ancien SHA-256 complet.

## Correction

- Un seul pump de scan est actif. Il parcourt des pages déterministes de 16 UUID avec un curseur strict, avance après chaque succès ou échec, et ne retente jamais une ligne dans le même passage.
- Chaque ligne est placée en état `queued`, puis `running`, avant l'observation du descripteur. Ainsi une preuve ancienne n'est plus éligible à la réassociation dès que le worker commence à la vérifier.
- Un jeton privé `file_version_token` est dérivé des métadonnées du descripteur ouvert : taille, device, inode, mtime et ctime avec nanosecondes. Il sert uniquement à invalider une preuve ; il ne constitue jamais une identité ni une preuve de contenu.
- Si le jeton observé est inchangé, l'état `complete` est restauré après le seul `stat`, sans lecture du contenu. S'il diffère, une nouvelle empreinte partielle remplace la preuve et le SHA-256 complet antérieur est supprimé, même si taille et empreinte partielle sont identiques.
- Les anciennes preuves complètes sans jeton sont rétrogradées conservativement en preuves partielles à l'ouverture de la base. Le schéma reste en version 3 et l'ajout est compatible avec les sauvegardes SQLite.
- Une identité devenue indisponible n'est pas rescannée : sa preuve complète et son jeton observé restent disponibles comme côté historique d'une réassociation certaine.

## Bornes et concurrence

Le pump et les demandes complètes explicites partagent le même sémaphore à un fichier. Une demande complète en file est prioritaire entre deux candidats du pump. Toutes les lectures utilisent des blocs de 64 Kio, le limiteur commun de 1 Mio/s, la suspension pendant lecture et l'annulation au changement de révision de racine.

Le système détecte une modification présente lors d'une observation ultérieure. Une modification de A vers B, suivie d'un retour complet vers A entre deux observations, est intrinsèquement invisible avec ces seules métadonnées et sans lecture/hash intégral continu du NAS. Aucune garantie contraire n'est revendiquée.

## Régression

- catalogue réel de 20 fichiers : le premier fichier supprimé finit `failed`, les 19 autres, y compris ceux après la première page, finissent `complete/partial` ;
- fichier de 512 Kio remplacé au même chemin, même taille, modification hors échantillons : l'empreinte partielle reste identique mais la preuve passe de `full` à `partial` ;
- descripteur inchangé : le chemin court retourne après métadonnées, avant toute fonction de lecture de contenu ;
- mutation pendant capture et changement de racine : aucune preuve n'est publiée.
