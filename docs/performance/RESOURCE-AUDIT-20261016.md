# Audit RAM/stockage —16octobre2026, lecture seule

Demande : chercher des optimisations sans modifier le fonctionnement. Sources/paramètres du candidat Configuration, client installé et serveur du foyer. Aucun effacement, VACUUM, modification de rétention ou réduction du tampon effectué. L'audit ne remplace ni les tests correctifs ni une mesure comparative d'une optimisation implémentée.

## Mesures

- Serveur avant déploiement : PID1106, RSS50240Kio = **49,1Mio**, NRestarts0 ; MemoryCurrent/Peak systemd non renseignés, pas zéro. C'est un instantané au repos, pas un pic ou benchmark à trois lectures. Données locales serveur680800Kio≈664,8Mio, web1200Kio≈1,17Mio ; aucun parcours du NAS. Sauvegardes et données utiles ne sont pas des candidats de purge automatiques.
- Client fermé confirmé par l'utilisateur et aucun processus Chill Center trouvé : **RAM client non mesurée**, pas artificiellement égale à zéro. Stockage AppSupport : **183500420octets logiques≈175Mio**, estimation allouée193429504≈184,5Mio,3800 fichiers ; inventaire scanner sans erreur. SQLite7,1Mio et WAL7Mio conservés comme ensemble ; pas d'ouverture SQL/immutable pendant cet audit ni de fragmentation prétendument mesurée.
- Hachage borné à64Mio :1139 fichiers vérifiés,2124 candidats hors budget,3 fichiers SQL/état protégés exclus. **25 groupes de doublons** identifiés, les dix principaux totalisent62435400octets alloués estimés de copies excédentaires≈59,5Mio. Ce n'est ni une couverture complète ni un gain disque immédiatement garanti (APFS/snapshots/clones). Toutes les copies peuvent être référencées ; aucune n'est déclarée supprimable par le seul hash.
- Dépôt canonique≈6,43Gio alloués estimés : `.claude`≈5,16Gio (chantier web/worktree/builds partagés à préserver), `artifacts`≈1,22Gio (versions livrées à conserver). Scanner64Mio seulement,5 gros fichiers hachés et25151 hors budget ; pas de promesse d'absence de doublons. Source/git/historiques non candidats de purge.
- Cible de compilation active temporaire≈17Gio, environnement cross569Mio ; cache Cargo≈1,3Gio **partagé**, non attribué intégralement à Chill Center. Ils sont encore utilisés par builds/tests/revues ; aucune suppression autorisée ici. Le Git commun est partagé avec des worktrees historiques ; avertissement de fichier auxiliaire64octets observé, pas de prune ou modification.

## Optimisations possibles, pas appliquées dans le lot

| Priorité | Proposition sans changement fonctionnel | Preuve et validation nécessaire |
| --- | --- | --- |
| RAM1 | Déplacer les éléments d'un snapshot reset vers le transfert retenu et partager son Arc pour le checkpoint, au lieu de recopier des graphes complets | `sync_api.rs` clone items puis snapshot ; borne32Mio sérialisés≠RSS. Conserver admission avant checkpoint, refus429, expiration120s, pages/reprise/durabilité. Mesurer pic RSS avec10000 éléments. |
| RAM2 | Consommer une seule fois le résultat reçu à la publication SQLite, au lieu de `self.result.clone()` | `CatalogPublishTask::publish_owned`, résultat jusqu'à10000 éléments/34Mio. Tester succès/CAS périmé/annulation et miroir précédent conservé ; mesurer allocation/RSS avant-après. Aucun gain chiffré non mesuré. |
| Disque1 | Dédupliquer les affiches identiques par contenu, avec références conservées pour chaque média/profil | Les copies mesurées existent réellement mais peuvent toutes servir. Concevoir publication atomique/références/réversibilité, jamais effacer des chemins encore référencés ou casser le hors-ligne. Le gain logique vérifié dans le sous-ensemble est conditionnel, pas un nettoyage proposé immédiatement. |
| Disque2 | Mesurer les pages SQLite libres après publication puis envisager un compactage explicite hors usage | Pas de fragmentation démontrée. Ne pas supprimer WAL/SHM ni ajouter VACUUM automatique, qui bloque et réclame de l'espace temporaire. |

Le tampon garde60s avant/environ30s arrière et les budgets Mio configurés. Rien n'est gagné en désactivant le catalogue hors ligne ou en effaçant des versions utiles. Pour une optimisation future, appliquer un refactor séparé avec même protocole, mêmes comportements et preuve avant/après ; ce lot ne revendique aucune baisse RAM mesurée.
