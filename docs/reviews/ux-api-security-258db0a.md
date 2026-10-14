# Revue de sécurité indépendante — Chill Center UI/API

## Décision

- **Verdict : CHANGES_REQUESTED**
- **Candidat revu :** `258db0ad27f05fafb12bd3592013ef300ead93fd`
- **Base :** `5665f60`
- **Relecteur :** `/root/ux_api_security_review` (indépendant de l'auteur)
- **Date :** 2026-10-14 UTC
- **Périmètre accepté :** serveur domestique sur LAN, sans authentification pour cette version. Cette acceptation ne couvre ni une exposition Internet, ni un modèle multi-tenant, ni une absence de bornes de ressources.

Le candidat ne doit pas être livré tel quel : la file TMDB permet à un client du LAN de provoquer une croissance persistante et non bornée de la mémoire, du disque et du nombre de tâches. Sur la cible Raspberry Pi 3, ce chemin d'indisponibilité est bloquant.

## Résultat synthétique

| Gravité | Ouvert | Résolu/contrôlé |
|---|---:|---:|
| CRITICAL | 0 | 0 |
| HIGH | 1 | 0 |
| MEDIUM | 0 | 0 |
| LOW | 0 | 0 |

## SEC-258-01 — File et historique TMDB persistants sans borne

**Gravité : HIGH — disponibilité persistante (CWE-400/CWE-770).**

### Preuve exacte

- `crates/chill-server/src/tmdb_api.rs:132` conserve tous les jobs dans un `BTreeMap` sans plafond ni éviction.
- `crates/chill-server/src/tmdb_api.rs:202-225` recharge l'intégralité de cet historique au redémarrage.
- `crates/chill-server/src/tmdb_api.rs:228-243` clone puis sérialise toute la map à chaque persistance avant réécriture atomique du fichier.
- `crates/chill-server/src/tmdb_api.rs:281-304` insère tout nouveau job et sa liste complète d'identifiants sans limite globale de jobs ou de taille d'état.
- `crates/chill-server/src/tmdb_api.rs:497-526` limite un job à 3 000 identifiants, mais crée une tâche Tokio par requête sans plafond de jobs actifs/en attente.
- `crates/chill-server/src/tmdb_api.rs:545-562` permet en plus de dupliquer un ancien worklist par retry.
- `crates/chill-server/src/tmdb_worker.rs:135-136` sérialise l'accès au fournisseur, mais ne borne ni les tâches en attente ni l'historique persistant.

### Scénario d'abus

Un appareil présent sur le LAN — niveau d'accès volontairement non authentifié — envoie de façon répétée des `POST /api/2026-10-14/admin/tmdb/jobs` valides. Chaque réponse ajoute un job persistant pouvant contenir jusqu'à 3 000 chaînes et démarre une tâche qui attend le mutex global. Les jobs terminés/annulés ne sont jamais supprimés. L'état croît donc sans borne, chaque mise à jour réécrit un volume croissant, et le redémarrage recharge tout en mémoire. Le résultat attendu est une amplification I/O/CPU, puis l'épuisement RAM ou disque et l'indisponibilité durable du service. La limite HTTP de 1 Mio et les 16 connexions simultanées ralentissent l'abus mais ne le bornent pas.

### Correction exigée

1. Remplacer le `spawn` par job par une file bornée (canal borné et worker unique) ou refuser au-delà d'un plafond très bas adapté au Pi avec `429`/`Retry-After`.
2. Borner le nombre de jobs actifs et l'historique terminal ; évincer/pruner durablement les anciens jobs.
3. Borner explicitement la taille sérialisée de `tmdb-jobs.json` et refuser avant mutation lorsque le budget serait dépassé.
4. Ajouter des tests de saturation, retry, redémarrage/pruning et échec de persistance, plus une mesure RSS/disque sur le Pi.

Le scénario destructif n'a pas été exécuté : la preuve statique est déterministe et un essai aurait volontairement consommé les ressources du candidat.

## Contrôles vérifiés sans anomalie bloquante supplémentaire

- **Frontière LAN / même origine :** l'écoute publique/wildcard est refusée ; la connexion rejette une adresse pair non locale ; les requêtes navigateur portant `Origin` ou `Sec-Fetch-Site` sont refusées et le `Host` doit correspondre aux noms TLS configurés. Il n'y a pas de CORS permissif.
- **Profils :** les opérations personnelles restent explicitement liées au profil. Cela assure l'isolation fonctionnelle entre profils, mais ne constitue pas une autorisation de sécurité en l'absence assumée d'authentification.
- **Secret TMDB :** fichier de données privé, écriture atomique `0600`, `fsync` fichier puis répertoire, suppression durable ; statut/API et structures `Debug` sont expurgés. Le secret et l'état du worker sont exclus des exports. Aucun secret statique n'a été trouvé par le balayage ciblé.
- **Sorties TMDB :** hôtes HTTPS fixes, redirections désactivées, timeouts et limites de réponse ; aucun commutateur public de test/base URL dynamique ; les erreurs fournisseur sont normalisées sans URL, corps ou credential.
- **API datée :** `/api/2026-10-14` et alias `/api/v1` partagent le même routeur ; enveloppes d'erreur et en-tête de version sont cohérents. Limite JSON par défaut 1 Mio, exceptions explicites et bornées pour affiches/archives.
- **Client natif / IPC :** CSP fermée, navigation externe refusée, commandes sensibles typées et sans journalisation de secret. Les grants, transports, profils et générations restent détenus côté Rust.
- **Lecteur :** lease et génération sont vérifiés, heartbeat/expiration indépendants, ancien flux révoqué, cache de sous-titres borné et privé, tâche de sous-titres annulée au transfert/release/stop, choix manuel préservé.
- **UI JS :** contenu distant injecté avec `textContent`/création DOM, sans `innerHTML` dans le runtime modifié ; profil et serveur sont révalidés avant les actions asynchrones sensibles.
- **Dépendances :** aucun manifeste ni lockfile de dépendances n'est modifié entre la base et ce candidat.

## Vérifications exécutées

- `git diff --check 5665f60..258db0ad27f05fafb12bd3592013ef300ead93fd` — PASS.
- Validateur de contrat — PASS : `66 chemins, OpenAPI 2026-10-14`.
- Tests DOM/Node — PASS : `37/37`.
- `cargo test -p chill-server --lib --locked --offline` avec cible temporaire — PASS : `90/90`, 0 échec.
- Balayage ciblé de motifs de clés privées/tokens usuels sur le commit — aucun résultat. Ce contrôle par motifs ne remplace pas un scanner d'entropie.
- L'évidence transmise pour le candidat annonce `282` tests Rust et un démarrage réel du shell Tauri ; cette revue n'a réexécuté que la suite serveur ci-dessus et ne revendique pas une validation indépendante de ces deux éléments.

## Limites et état du freeze

- Aucun token TMDB réel n'a été utilisé ; aucune clé, credential ou donnée externe n'a été lu ; aucun appel fournisseur réel n'a été effectué.
- Aucun Pi/NAS, compte SSH, cloud ou état externe n'a été modifié.
- Le test TLS/client disponible emploie une clé synthétique et le chemin CLI ; il ne prouve pas un cycle réel de renouvellement sur appareil.
- Le plein écran natif du commit exact reste sous diagnostic : géométrie observée `1073` contre `1107` attendue dans la zone sûre. **Aucun PASS natif n'est accordé sur ce point.**
- Pendant la revue, des modifications postérieures non commitées sont apparues dans `apps/desktop/src/native_player.rs`, `docs/api/openapi-2026-10-14.json` et `ui/connected/player-qa.js`, avec deux fichiers non suivis. Elles sont exclues du verdict ; les références sensibles ci-dessus ont été revérifiées via `git show` sur le SHA exact. Toute correction native/API post-candidat requiert une revue delta ciblée.
- L'ADR d'API/TMDB reste une entrée de conception fournie ; cette revue de code ne vaut pas auto-approbation architecturale par son auteur.

## Condition de nouvelle revue

Fournir un SHA immuable corrigeant `SEC-258-01`, avec tests de file bornée, rétention/pruning, redémarrage et limites disque/RAM. Joindre séparément le SHA de la correction plein écran post-candidat afin de contrôler son delta natif et contractuel.
