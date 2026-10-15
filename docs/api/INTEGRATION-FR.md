# Intégrer un client tiers — API Chill Center 2026-10-14

Le client Mac officiel, et tout client tiers, parlent au serveur avec une API JSON sur HTTPS. Les octets vidéo passent aussi par HTTPS, mais uniquement en `Range` HTTP : le serveur ne transcode jamais et un client n'accède jamais au NAS.

Le contrat canonique est [openapi-2026-10-14.json](openapi-2026-10-14.json), servi par chaque instance à `GET /api/2026-10-14/openapi.json`. La version est une **date de contrat**, pas une date calculée au jour de la requête. La base est donc :

```
https://nom-ou-ip-du-serveur:8443/api/2026-10-14
```

`/api/v1` reste un alias de compatibilité pour les anciens clients. Une nouvelle intégration doit utiliser la date. La réponse porte `X-Chill-Api-Version: 2026-10-14`; `GET /server` annonce aussi les versions et capacités réellement servies. Les réponses peuvent recevoir de nouveaux champs : un client doit ignorer les champs de réponse inconnus, mais ne doit jamais envoyer de champs inconnus dans les corps documentés.

## Connexion TLS et périmètre LAN

Chill Center est prévu pour le réseau local du foyer, sans authentification. Toute personne pouvant atteindre l'API peut administrer le catalogue et choisir un profil. Ne l'exposez pas sur Internet et ne placez pas de proxy public devant elle.

À la première connexion, un client doit présenter l'adresse et l'empreinte SPKI du certificat au propriétaire, puis enregistrer l'identité du serveur. Le certificat est renouvelé annuellement et reste valide 18 mois. N'utilisez jamais `curl -k`, `--insecure`, ni une désactivation globale de TLS. Pour un client en ligne de commande, fournissez le certificat public approuvé :

```sh
BASE='https://192.168.1.42:8443/api/2026-10-14'
curl --fail-with-body --cacert ./chill-center-public.pem "$BASE/server"
```

Il n'y a pas de CORS : les requêtes avec `Origin` ou `Sec-Fetch-Site` sont refusées. Un site web ou Swagger UI chargé depuis une autre origine ne peut donc pas appeler le serveur. Intégrez l'API depuis un client natif, un service local de confiance, ou un futur client web explicitement supporté.

## Erreurs et écritures concurrentes

Toute erreur applicative a ce format, sans chemin NAS ni secret :

```json
{"code":"PROFILE_CONFLICT","message":"Opération refusée ou temporairement indisponible.","request_id":"b02b03d5-5c84-4055-b7b5-9a9a7921f41f"}
```

Conservez `request_id` pour un diagnostic. Les codes HTTP comptent : `400/422` requête invalide, `403` origine ou hôte refusé, `404` identifiant absent, `409` conflit/bail actif, `412` média ou sous-titre modifié, `413` limite dépassée, `416` plage vidéo impossible, `429` worker saturé et `5xx` indisponibilité. Une requête annulée ou une lecture de base saturée peut aussi retourner `408`, `503` ou un code stable tel que `BUSY`/`REQUEST_TIMEOUT`; faites un retry borné seulement pour les opérations explicitement sûres en lecture.

Les objets partagés qui indiquent `revision` emploient un verrouillage optimiste. Relisez l'objet après `409`, présentez le conflit, puis renvoyez une mutation avec la révision réellement lue. N'inventez pas une règle CAS globale : certaines mutations personnelles (`watched`, `rating`, préférences, suppression de playlist) n'utilisent pas toutes une `expected_revision`. L'OpenAPI indique les champs requis par opération. Les requêtes sont strictes (champs inconnus refusés) ; les réponses sont extensibles et un client doit ignorer les champs supplémentaires.

## Profils, catalogue et collections

Un profil est toujours explicite dans l'URL. Les sagas sont communes au foyer; les playlists appartiennent au profil de l'URL. `profile`, `media`, `playlist`, export/import et travaux TMDB sont des UUID. En revanche, l'identifiant de `saga` est opaque et le paramètre `series` est un nom URL-encodé : ne les convertissez pas en UUID.

La suppression d’un profil exige l’en-tête `x-chill-confirm-delete` dont la valeur est exactement l’UUID `{profile}` de l’URL. Ce n’est ni `true`, ni une confirmation générique.

`PUT .../preferences` remplace les préférences : envoyez audio_language, subtitle_language et autoplay_seconds ensemble. Un champ omis reprend son défaut historique (anglais/anglais/10 secondes), pas la valeur précédemment enregistrée. Pour son nouveau défaut de 20 s, le client Mac envoie `POST /profiles` avec `initial_preferences: {"audio_language":"en","subtitle_language":"en","autoplay_seconds":20}` : création et préférences sont atomiques. Sans ce nouveau champ optionnel, les anciens clients gardent leur comportement. Les profils existants ne sont pas réécrits. Une chaîne de sous-titres vide désactive la préférence automatique ; `autoplay_seconds: null` désactive l’enchaînement automatique. Les valeurs admises sont 0, 5, 10, 15, 20 et 30 secondes.

Le catalogue est paginé côté serveur (`offset`, `limit`, maximum 100, défaut 48), même si l'interface officielle charge les pages en continu. Pour une intégration légère, demandez seulement ce qui est visible :

```sh
PROFILE='11111111-1111-4111-8111-111111111111'
curl --fail-with-body --cacert ./chill-center-public.pem \
  "$BASE/profiles/$PROFILE/catalog?offset=0&limit=48&category=film&available=true"
```

La réponse contient `items`, `has_more` et `series_counters`. Chaque compteur peut fournir `series_key`, clé opaque de groupe (`tmdb:tv:<id>` pour une série identifiée, `media:<stable_uuid>` pour un groupe non identifié, et le titre seul uniquement lorsqu'il n'existe qu'un groupe correspondant). Ne fusionnez jamais deux groupes sur leur seul titre : les homonymes, comme deux versions de Doctor Who, restent distincts. `Media.path` est seulement un chemin relatif de catalogue : ce n'est ni une URL, ni un droit d'accès au stockage. Pour modifier une fiche, une saga, ou un ordre, suivez exactement les révisions et les corps de l'OpenAPI.

### Synchronisation locale du catalogue (client lourd)

Le client officiel conserve les métadonnées et les miniatures dans son cache privé local ; les flux vidéo restent exclusivement servis par le serveur. Une intégration tierce peut adopter le même comportement avec l'extension additive suivante de l'API datée :

`GET /profiles/{profile}/catalog/revision` répond seulement `{ "revision": "…" }`, sans lire les fiches, créer un checkpoint ou ouvrir un transfert. C'est le même curseur opaque commun/personnel que pour `changes` : comparez-le par égalité, jamais comme un nombre. Une modification d'un autre profil seul ne l'invalide pas. Ce GET convient au navigateur, qui ne possède pas le miroir SQLite du Mac : inchangé, il conserve son affichage ; changé, il recharge les données selon son fonctionnement existant. Ce GET n'est pas un delta ni une capture atomique du catalogue. Sur un ancien serveur, seul `404 ROUTE_NOT_FOUND` permet le repli vers un rechargement classique ; une erreur de profil/réseau ne signifie pas « inchangé ». Ajout compatible à la version2026-10-14, disponible aussi sous l'aliasv1 ; pas encore présent sur le Pi ffcc6349 au contrôle du15octobre.

`POST /profiles/{profile}/catalog/changes` accepte `{ "known_revision": "…" }`. Si rien n'a changé, elle répond `{ "revision": "…", "unchanged": true }` sans matérialiser le catalogue. Sinon le serveur compare le checkpoint durable connu avec une capture cohérente et retourne uniquement les ajouts/modifications dans `items` et les identifiants supprimés dans `deleted_ids`. Une première connexion, un curseur inconnu ou élagué donne `reset: true` et `items` représente alors le catalogue complet. La réponse contient aussi les sagas, playlists, révisions de séries et `season_metadata`, ainsi que `snapshot_id` pour les pages suivantes. Les éléments sont `{ "media": {…}, "revision": n }`.

`GET /profiles/{profile}/catalog/changes/{snapshot}?offset=…` lit les pages immuables (au plus 100 éléments). Le snapshot est lié au serveur et au profil, conservé jusqu'à 120 secondes, avec trois transferts actifs et 32 Mio sérialisés globalement au maximum. Une réponse complète en une page n'occupe aucun créneau ; la dernière page libère le transfert. Si cette dernière réponse est perdue, `404 SYNC_SNAPSHOT_NOT_FOUND` impose de relancer le POST sans publier de résultat partiel. La révision est un curseur opaque durable, qui survit au redémarrage : elle ne contient pas d'identifiant de processus et ne doit pas être comparée comme un nombre. Une restauration officielle change sa génération pour éviter la réutilisation d'une ancienne révision. Les checkpoints ne contiennent que des identifiants et signatures SHA-256, sont supprimés avec leur profil et limités à quatre par profil et 16 Mio globalement. Le client applique le delta en une transaction locale; il conserve l'ancien miroir si le transfert échoue. Les routes historiques `/catalog/snapshot` restent disponibles et inchangées pour les clients existants.

Pour compatibilité, les pages historiques se lisent par `GET /profiles/{profile}/catalog/snapshot/{snapshot_id}?offset=100`. L'instantané historique est immuable pendant deux minutes, lié au serveur et au profil. Le nouveau flux `changes` est préféré pour éviter de retransférer les éléments inchangés. Un quatrième transfert actif renvoie `429 SYNC_BUSY`; `413 SYNC_TOO_LARGE` indique une limite atteinte. Les positions de lecture, vitesses et préférences, absentes du miroir, n'invalident pas le delta; les vus/notes n'invalident que le profil concerné.

Un scan concurrent pendant la capture peut produire `409 SYNC_SNAPSHOT_CHANGED` : reprendre le `POST` de façon bornée (le Mac fait au plus trois tentatives espacées de 100 ms), puis conserver le cache précédent si le conflit persiste. Les epochs sont comparés avant/après toutes les lectures : aucune page incohérente n'est publiée sous une nouvelle révision.

Les documents auxiliaires (`sagas`, `playlists`, `series_revisions`, `season_metadata`) sont complets dans la réponse POST uniquement. Les pages GET suivantes portent des tableaux/objets vides pour ces champs ; ne remplacez pas le document initial avec ces valeurs. Le Mac borne le cumul de toutes les pages JSON, documents compris, à 32 Mio. Les anciens checkpoints complets du prototype non livré sont un cache jetable et sont vidés au profit des signatures bornées ; aucune donnée catalogue/personnelle n'est effacée.

Le client Mac conserve ces métadonnées dans une SQLite privée par serveur/profil, ainsi que les originaux et miniatures JPEG des affiches. Les lectures de grilles/fiches/suggestions/séries/listes sont locales ; les originaux d'images se chargent progressivement, un à la fois, hors du verrou du lecteur. Plafond d'images : 2 Gio par profil. Les changements du catalogue déclenchent une nouvelle copie cohérente ; inchangé = seulement la vérification de révision. Les vidéos restent diffusées par Range HTTPS, jamais dans ce miroir. En cas d'échec d'une actualisation, la dernière copie complète est conservée et signalée comme périmée, sans tentative réseau à chaque navigation ; démarrage/focus/rechargement explicite permettent de retenter. Aucune mutation n'est rejouée hors ligne. Après une liste de profils autoritative valide, seuls les caches des profils supprimés de ce serveur sont purgés ; échec signalé, liens symboliques refusés.

Les archives contenant une affiche de playlist ont un `data.json.format_version` 2 ; le conteneur ZIP/manifeste conserve son format 1. Le nouvel importeur accepte les documents 1 et 2. Un ancien serveur ne peut pas importer un document 2 : remettre son ancienne sauvegarde privée pour un rollback complet. Les documents sans cette extension restent au format 1.

Les chemins restent relatifs et l'instantané n'expose ni chemin NAS absolu, ni base opérationnelle, ni autre profil. Ne réutilisez jamais le cache d'un serveur ou d'un profil pour un autre. Les routes historiques de catalogue restent inchangées.

Une playlist peut également avoir une affiche personnalisée : `PUT /profiles/{profile}/playlists/{playlist}/poster?expected_revision=n` avec une image JPEG, PNG ou WebP de 8 Mio maximum, puis `GET /profiles/{profile}/playlists/{playlist}/artwork`. Le champ optionnel `poster` de la playlist contient cette URL quand elle existe. Sans affiche personnalisée, le client compose sa tuile à partir des neuf premiers membres.

Une série peut être identifiée et éditée au niveau série avec les scopes TMDB `series` et `season`. `SeriesResponse.tmdb_id` identifie la série choisie (deux séries homonymes restent distinctes) et `season_metadata` contient les métadonnées connues par numéro de saison. La réponse de `PUT /profiles/{profile}/series/{series}` contient `episodes`, `revision` et la clé opaque `key` à réutiliser après renommage. La recherche de saison se fait à partir du `tmdb_id` de la série. Les affiches peuvent être importées sans fermer la fiche : `PUT /profiles/{profile}/media/{media}/series/poster` ou `PUT /profiles/{profile}/media/{media}/season/poster` (CAS via `expected_revision`), et l'affiche de saison est lue par `GET /profiles/{profile}/series/{tv}/seasons/{season}/artwork`. Les formats acceptés restent JPEG/PNG/WebP, 8 Mio maximum.

## Lecture : bail, SSE et Range

Un flux n'est possible qu'après acquisition d'un bail du profil. Utilisez un `device_id` UUID persistant par appareil ; le `session_id` est attribué par le serveur. La séquence normale est :

1. `POST /profiles/{profile}/playback/acquire` avec `media_id`, `device_id`, `context`. Le serveur attribue le `session_id` dans le bail retourné.
2. Conservez le bail reçu (session, génération, révision du média, échéance) et ouvrez `GET /media/{media}/stream` avec `X-Chill-Session`, `X-Chill-Generation` et un unique `Range`.
3. Renouvelez avec `heartbeat` environ toutes les 5 secondes; envoyez `progress` avec une séquence strictement croissante et des millisecondes.
4. Écoutez `playback/events` en SSE. Un événement pause impose un arrêt local, y compris si le lecteur possède déjà des octets en tampon.
5. À pause, arrêt ou changement de média, envoyez la progression finale puis `release`.

`Progress.final_elapsed_ms` est du temps réel de lecture dans les derniers 10 % de la vidéo, sans pause, buffering ou seek. À 10 secondes cumulées, le serveur marque vu. Une vitesse ×2 ne double pas ce temps. Les vitesses autorisées sont 0,5; 0,75; 1; 1,25; 1,5; 1,75; 2.

Le transfert du même profil est volontairement confirmé : proposer, confirmer, écouter la pause et ACK final de l'ancien lecteur, puis seulement jouer avec le nouveau bail. Ne tentez jamais de prolonger un ancien bail pendant ce transfert.

Pour la vidéo, ne demandez jamais le fichier complet. Envoyez une seule plage, conservez l'`ETag`, puis transmettez-le dans `If-Match` aux lectures suivantes. `206` est normal; `412` signifie que le média a changé et `416` que la plage est impossible. Le buffer client est local : il ne donne aucun accès direct au NAS.

## Entiers 64 bits et prototype Python sûr

`media_revision` est un entier non signé 64 bits (`uint64`) opaque : il doit être conservé exactement dans `Lease`, `PlaybackControl` et `Progress`. Python et Rust le font nativement. Un client JavaScript ne doit pas passer par `JSON.parse`/`Number` lorsqu’il est susceptible de recevoir une valeur supérieure à `2^53 - 1` : utilisez un parseur JSON sans perte, conservez la valeur en `BigInt` ou en texte, puis réémettez le même **nombre JSON** exactement. Une chaîne JSON n’est pas substituable sans accord du contrat, et `JSON.stringify` ne gère pas directement `BigInt` : employez un sérialiseur sans perte adapté. Une valeur arrondie est refusée avec `LEASE_INVALID`.

Ce prototype Python standard-library n’appelle aucun fournisseur externe. Remplacez les UUID factices et le certificat public approuvé ; il acquiert un bail, demande une plage, renouvelle, puis libère le bail. Il ne désactive jamais TLS.

```python
import json
import ssl
import urllib.request

base = "https://192.168.1.42:8443/api/2026-10-14"
profile = "11111111-1111-4111-8111-111111111111"
media = "22222222-2222-4222-8222-222222222222"
device = "33333333-3333-4333-8333-333333333333"
tls = ssl.create_default_context(cafile="./chill-center-public.pem")

def request(path, method="GET", payload=None, headers=None):
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request_headers = {"Accept": "application/json", **(headers or {})}
    if data is not None:
        request_headers["Content-Type"] = "application/json"
    req = urllib.request.Request(base + path, data=data, method=method, headers=request_headers)
    with urllib.request.urlopen(req, context=tls, timeout=20) as response:
        return response.status, {name.lower(): value for name, value in response.headers.items()}, response.read()

control = None
try:
    _, _, raw = request(f"/profiles/{profile}/playback/acquire", "POST", {
        "media_id": media, "device_id": device, "context": {"kind": "standalone", "id": None}
    })
    grant = json.loads(raw)             # Python conserve les entiers 64 bits.
    lease = grant["lease"]
    control = {key: lease[key] for key in ("session_id", "generation", "media_id", "media_revision")}
    status, headers, chunk = request(f"/media/{media}/stream", headers={
        "X-Chill-Session": control["session_id"],
        "X-Chill-Generation": str(control["generation"]),
        "Range": "bytes=0-65535"
    })
    assert status == 206 and headers["content-range"].startswith("bytes ")
    request(f"/profiles/{profile}/playback/heartbeat", "POST", control)
finally:
    if control is not None:
        request(f"/profiles/{profile}/playback/release", "POST", {"lease": control, "progress": None})
```

## Administration, TMDB, IA et archives

Les endpoints `/admin/*` ont le même périmètre LAN sans auth. Un scan ne peut pas être déclenché via la page catalogue par défaut; l'import/export et la configuration de bibliothèque sont aussi administratifs.

La clé TMDB est envoyée seulement avec `PUT /admin/tmdb`, dans `credential` marqué `writeOnly`. Une clé API v3 fait exactement 32 caractères hexadécimaux; un jeton v4 fait 20 à 2 048 caractères graphiques après l’éventuel préfixe `Bearer `. Les espaces de début et de fin sont acceptés puis retirés; ils ne font pas partie du secret. `profile_id` est requis seulement lorsque `auto_enrich` vaut `true`. Le serveur ne renvoie jamais le secret, l'exclut des exports et le statut ne révèle que `configured`, son type, sa source et l'automatisme. Ne mettez ni clé v3 ni Bearer dans une URL, un journal, une capture, ou une réponse d'erreur.

Les travaux TMDB se suivent par identifiant avec `GET /admin/tmdb/jobs/{id}` et peuvent être annulés/repris. Le serveur accepte au plus 2 travaux actifs, conserve au plus 8 travaux terminés pendant 15 minutes, limite une liste de travail à 3 000 médias et son état persistant à 2 Mio. Un dépassement ou une file pleine renvoie `413` ou `429`; attendez et relisez le statut au lieu de soumettre un doublon. Les clients doivent afficher leurs compteurs sans présumer que la liste interne de médias est publique.

Une annulation n’autorise pas immédiatement un troisième worker : la réservation est rendue à sa fin effective, y compris la fin d’une requête fournisseur bornée à 15 secondes. L’expiration des historiques est réconciliée au prochain accès/mise à jour ou redémarrage, sans timer permanent. Les historiques expirés peuvent renvoyer `404 TMDB_JOB_NOT_FOUND`. Une écriture atomique peut brièvement occuper l’ancien et le nouveau fichier d’état (2 Mio chacun).

L'IA n'est pas exécutée par ce serveur. `GET /profiles/{profile}/ai/context` fournit un snapshot borné au profil (maximum 3 000 fiches, filtres `watched`, `available`, `category` ou `categories` CSV). Envoyez ensuite un plan à `ai/apply` seulement après confirmation humaine et avec les deux révisions fournies par le snapshot.

Un export est asynchrone, puis téléchargeable en ZIP lorsqu'il est prêt. Un import est une transaction en quatre temps : ZIP vers aperçu, aperçu vers plan de mapping profils/racine, plan confirmé vers application, ou annulation. Limites : ZIP compressé 1 Gio, décompressé 2 Gio, 20 000 entrées; les vidéos et secrets ne sont jamais importés ni exportés. Un plan est temporaire et peut expirer; un `409 IMPORT_PLAN_STALE` impose de refaire l'aperçu.

## Compatibilité et limites

Le serveur ne promet pas un codec particulier dans MKV/MP4/AVI : le décodage et les pistes sont gérés par la machine cliente. Les sous-titres externes sont des ressources protégées par le bail, limitées à 16 Mio. Les affiches chargées sont JPEG/PNG/WebP, limitées à 8 Mio. Les appels JSON ordinaires sont limités à 1 Mio.

Avant une mise à jour de client, lisez `GET /server` et n'activez une capacité que si elle est annoncée. Avant une intégration de production domestique, testez les erreurs Range, l'expiration du bail, la reprise après transfert, une modification concurrente, et une connexion TLS avec certificat renouvelé.
