# Revue indépendante — synchronisation catalogue détachée

- Base : `a59f5e6921f5e495c7fadc039dadb99c814d844a`
- Candidat source : `f544684b002ad8c1bf5d31f98006da573f576019`
- Périmètre : delta serveur, cache client, hôte Tauri, chargement catalogue et ADR-0014
- Mode : `DEV_STANDARD`, revue source non mutante
- Verdict code : **CHANGES_REQUESTED**
- Verdict ADR-0014 : **CHANGES_REQUESTED**

## Constats bloquants

1. **P1 — Les curseurs persistants recopient le catalogue complet par profil et ne possèdent aucune borne disque globale.** `catalog_delta_checkpoint` sérialise chaque `SyncSnapshot` complet puis conserve jusqu'à quatre copies et 32 Mio **par profil** (`crates/chill-domain/src/catalog_delta.rs:84-131`). Un simple changement personnel, par exemple une note, produit donc une nouvelle copie de toutes les métadonnées. Le nombre de profils n'est pas borné, ce qui rend la croissance totale non bornée sur le Pi. En outre, l'endpoint écrit cette copie avant de vérifier si le cache mémoire peut accepter le transfert (`crates/chill-server/src/sync_api.rs:177-186`, puis rejet possible `:222-230`) : des requêtes qui terminent en 429 provoquent quand même le coût CPU/SQLite. Reproduction déterministe : avec un instantané de 8 Mio, créer/synchroniser plusieurs profils et faire avancer quatre fois leur révision personnelle permet environ 32 Mio par profil ; 100 profils atteignent environ 3,2 Gio malgré la « borne ». Correction requise : stocker des checkpoints compacts `stable_id -> signature/révision` (et des signatures de documents), ajouter une borne globale/une politique d'éviction, puis réserver la capacité de transfert avant toute matérialisation persistante. Ajouter un test multi-profils qui prouve la borne globale et qu'un 429 n'écrit rien.

2. **P2 — Un changement de profil peut repeindre les cartes de l'ancien profil et permettre une action dans le nouveau contexte.** `loadPage` conserve volontairement `catalogItems` jusqu'au commit (`ui/connected/app.js:119-135`), mais `selectProfile` ne vide que le DOM (`:67`) et ne réinitialise ni les éléments ni les compteurs. Pendant l'attente de la première page du nouveau profil, un scroll/resize/changement d'affichage rappelle `renderCatalogWindow` (`:107-108`, `:149`) et réinsère les anciennes cartes (`:72-105`). Leurs boutons envoient alors `set_watched` avec le profil désormais actif. Le test existant ne vérifie que le résultat final tardif, pas l'absence de cartes intermédiaires. Correction requise : associer le tableau rendu à une clé serveur/profil/filtres ; conserver l'ancien tableau uniquement lors d'une recharge dans la même clé, le supprimer immédiatement sur changement de serveur/profil. La single-flight de synchro doit aussi être relancée pour le nouveau contexte lorsqu'une synchro de l'ancien profil est écartée. Ajouter une régression qui bloque la première page B, force `renderCatalogWindow`, et affirme qu'aucune carte A ni action A n'est disponible.

3. **P2 — Le cache de transfert évince un instantané paginé encore actif et sous-estime sa mémoire.** Dès qu'il contient trois valeurs, l'insertion supprime l'entrée la plus ancienne sans tenir compte du TTL ni d'un transfert encore paginé (`crates/chill-server/src/sync_api.rs:207-220`). La page suivante de ce client reçoit alors `SYNC_SNAPSHOT_NOT_FOUND`. Le budget additionne uniquement `baseline.serialized_bytes` (`:223-228`) alors que chaque entrée garde également `snapshot`; un reset conserve donc deux images complètes en mémoire (`:197-205`). Reproduction : ouvrir trois resets de plus de 100 éléments sans récupérer leurs pages, ouvrir un quatrième reset, puis demander la page 100 du premier identifiant ; elle retourne 404 avant les 120 s annoncées. Correction requise : ne jamais évincer un transfert non expiré (refuser la nouvelle ouverture avec 429, ou suivre un bail/toucher explicite), et compter les deux représentations ou ne conserver qu'une seule image. Ajouter ce scénario de route.

4. **P2 — La phase locale de publication peut encore bloquer l'application avec un coût quadratique sous le mutex UI.** `commit_sync` recherche chaque upsert par balayage linéaire et chaque suppression par balayage imbriqué (`crates/chill-client/src/catalog_cache.rs:557-570`), puis réécrit tout le miroir SQLite. Cette méthode est appelée en tenant `Bridge.client` (`apps/desktop/src/main.rs:55-60`). Un reset autorisé de 10 000 éléments effectue donc environ 50 millions de comparaisons avant les insertions et bloque toutes les commandes natives pendant ce temps, ce qui contredit l'usage complet pendant la vérification. Correction requise : fusion en `HashMap`/`HashSet` en O(n), mesure à la borne de 10 000, et publication préparée hors verrou avec un très court échange/CAS final si l'architecture du cache le permet.

## Points conformes observés

- Le HTTPS de `SyncCatalog` est réellement attendu hors du mutex `UiClient` et hors du sémaphore de navigation ; préparation et commit seulement reprennent le verrou (`apps/desktop/src/main.rs:34-63`).
- Le commit vérifie serveur, profil, révision de base et génération de mutation (`crates/chill-client/src/ui.rs:653-684`).
- Les pages sont validées et le miroir est remplacé dans une transaction SQLite ; l'ancien miroir reste disponible en cas d'échec.
- Le repli ancien serveur est limité au 404 `ROUTE_NOT_FOUND`.
- Aucun runtime ni dépendance n'est ajouté dans ce delta.

## Preuves exécutées sur l'archive exacte du candidat

- `cargo test --locked --offline -p chill-server catalog_changes_contract` : 1 PASS.
- `cargo test --locked --offline -p chill-domain checkpoint_survives` : 1 PASS.
- `node --test ui/connected/catalog-loading.dom.test.js` : 5 PASS.
- `git diff --check base..candidate` : PASS.

Ces preuves sont ciblées. Aucun bundle natif, Pi, NAS, RSS ou latence réelle n'a été vérifié dans cette revue.

## ADR-0014

La séparation réseau/verrou, le reset explicite et la compatibilité 404 sont justifiés. L'ADR ne peut toutefois pas être accepté tant que sa décision assume des métadonnées complètes par checkpoint (`ADR-0014:13`) et affirme une rétention disque bornée alors qu'elle ne l'est que par profil. Il doit être révisé avec la représentation compacte, la borne globale, la protection des transferts paginés actifs et le comportement après restauration d'une ancienne base (risque d'ABA des compteurs de révision).
