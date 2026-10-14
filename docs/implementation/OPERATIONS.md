# Sauvegarde et restauration opérationnelles

Révision : US-029 / MIG-002. Cette procédure est une sauvegarde privée du
service, distincte de l'export portable du foyer. Elle ne lit, ne copie et ne
modifie jamais les vidéos ou le NAS.

## Préconditions

- Exécuter le binaire avec l'utilisateur système dédié qui possède `data_dir`.
- Arrêter le service avant toute commande. L'outil ne tue aucun processus et
  refuse l'opération si le verrou exclusif est occupé.
- Conserver le fichier de configuration statique à son emplacement administré
  (par exemple `/etc/chillcenter/server.json`). Une copie est incluse dans la
  sauvegarde pour référence, mais une restauration ne la réapplique jamais :
  les adresses d'écoute, noms TLS et chemins configurés courants restent un
  choix explicite de l'opérateur.
- Ne pas utiliser `/`, le dossier personnel, le workspace ou une racine située
  sous `video_root` comme `data_dir`.

Le service prend le même verrou exclusif avant d'ouvrir TLS ou SQLite et le
conserve pendant toute sa durée de vie. Un journal de restauration présent à
côté de `data_dir` bloque aussi tout démarrage avant ouverture de la base.

## Créer une sauvegarde

```text
chill-server --backup --config /etc/chillcenter/server.json
```

La commande crée un UUID privé sous `data_dir/backups/`. Le manifeste de format
1 est écrit en dernier, après synchronisation des données. Une sauvegarde sans
`manifest.json` final n'est pas complète.

Contenu autorisé uniquement :

- snapshot SQLite/WAL cohérent `catalog.sqlite`, limité à 256 Mio, vérifié par
  `quick_check`, schéma 3 ;
- `library.json` ;
- affiches régulières sous `posters/`, au plus 8 Mio par fichier, 20 000
  fichiers et 1 Gio au total avec le reste de la sauvegarde ;
- copie de référence `server-config.json` ;
- certificat, calendrier et clé TLS sous `secrets/tls/`, séparés du contenu
  public, fichiers `0600` et dossiers `0700`.

Le manifeste lie chaque fichier à sa taille et son SHA-256 ainsi qu'au
`data_dir` exact. `videos`, imports, exports, anciennes sauvegardes et fichiers
inconnus sont exclus. Ne jamais publier une sauvegarde opérationnelle : elle
contient la clé privée TLS.

## Restaurer

```text
chill-server --restore /var/lib/chillcenter/backups/UUID \
  --config /etc/chillcenter/server.json --confirm-replace
```

Sans confirmation ou avec un service actif, la commande refuse avant toute
préparation de remplacement. Elle vérifie d'abord le manifeste, le chemin
original exact, la whitelist, l'injectivité des chemins, les limites, tous les
hashes, `quick_check` et le schéma. Elle construit ensuite un arbre complet
privé à côté de `data_dir`, puis sauvegarde l'état courant avant publication.

La publication conserve l'ancien arbre sous
`.NOM.pre-restore-UUID`; aucune suppression n'est effectuée. Le fichier de
configuration statique n'est pas restauré automatiquement. Les chemins absolus
d'affiches contenus dans la base restent valides parce qu'une sauvegarde ne
peut être restaurée que vers son `data_dir` original. Les baux de lecture sont
volatils et disparaissent avec l'ancien processus arrêté ; les Macs doivent se
reconnecter et sélectionner un profil existant si nécessaire.

Après vérification fonctionnelle, l'opérateur peut archiver manuellement les
anciens arbres et sauvegardes selon sa politique de rétention. Cette procédure
ne les efface pas.

## Reprendre après une coupure

Si une coupure survient après le premier renommage, le prochain démarrage
affiche qu'une récupération est requise. Garder le service arrêté puis lancer :

```text
chill-server --recover --config /etc/chillcenter/server.json --confirm-replace
```

La commande lit uniquement l'UUID du journal privé et recalcule les trois noms
de chemins attendus. Elle remet `.NOM.pre-restore-UUID` en place. Si le nouvel
arbre avait déjà été publié, il est conservé sous
`.NOM.recovery-UUID` comme candidat d'analyse ; il n'est jamais supprimé. Le
journal est retiré en dernier, après synchronisation durable du parent.

En cas de refus (journal corrompu, chemins ambigus, permissions non privées),
ne renommer aucun dossier à la main : conserver les arbres et faire examiner
l'état par l'administrateur. Le rollback d'un ancien binaire exige toujours une
base de schéma compatible, ou la restauration de la sauvegarde pré-migration.

## Retour arrière

Le retour normal consiste à arrêter le service puis restaurer la sauvegarde
pré-remplacement conservée dans l'ancien arbre. Les informations créées après
la sauvegarde choisie peuvent être perdues du catalogue restauré, mais restent
présentes dans l'arbre `.pre-restore-*`. Les vidéos et le NAS ne sont jamais
touchés par ce mécanisme.
