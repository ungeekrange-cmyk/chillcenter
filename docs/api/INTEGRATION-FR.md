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

Les objets partagés qui indiquent `revision` emploient un verrouillage optimiste. Relisez l'objet après `409`, présentez le conflit, puis renvoyez une mutation avec la révision réellement lue. N'inventez pas une règle CAS globale : certaines mutations personnelles (`watched`, `rating`, préférences, suppression de playlist) n'utilisent pas toutes une `expected_revision`. L'OpenAPI indique les champs requis par opération.

## Profils, catalogue et collections

Un profil est toujours explicite dans l'URL. Les sagas sont communes au foyer; les playlists appartiennent au profil de l'URL. `profile`, `media`, `playlist`, export/import et travaux TMDB sont des UUID. En revanche, l'identifiant de `saga` est opaque et le paramètre `series` est un nom URL-encodé : ne les convertissez pas en UUID.

Le catalogue est paginé côté serveur (`offset`, `limit`, maximum 100, défaut 48), même si l'interface officielle charge les pages en continu. Pour une intégration légère, demandez seulement ce qui est visible :

```sh
PROFILE='11111111-1111-4111-8111-111111111111'
curl --fail-with-body --cacert ./chill-center-public.pem \
  "$BASE/profiles/$PROFILE/catalog?offset=0&limit=48&category=film&available=true"
```

La réponse contient `items`, `has_more` et `series_counters`. `Media.path` est seulement un chemin relatif de catalogue : ce n'est ni une URL, ni un droit d'accès au stockage. Pour modifier une fiche, une saga, ou un ordre, suivez exactement les révisions et les corps de l'OpenAPI.

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

## Administration, TMDB, IA et archives

Les endpoints `/admin/*` ont le même périmètre LAN sans auth. Un scan ne peut pas être déclenché via la page catalogue par défaut; l'import/export et la configuration de bibliothèque sont aussi administratifs.

La clé TMDB est envoyée seulement avec `PUT /admin/tmdb`, dans `credential` marqué `writeOnly`. Le serveur ne la renvoie jamais, l'exclut des exports et le statut ne révèle que `configured`, son type, sa source et l'automatisme. Ne mettez ni clé v3 ni Bearer dans une URL, un journal, une capture, ou une réponse d'erreur. Les travaux TMDB se suivent par identifiant avec `GET /admin/tmdb/jobs/{id}` et peuvent être annulés/repris. Les clients doivent afficher leurs compteurs sans présumer que la liste interne de médias est publique.

L'IA n'est pas exécutée par ce serveur. `GET /profiles/{profile}/ai/context` fournit un snapshot borné au profil (maximum 3 000 fiches, filtres `watched`, `available`, `category` ou `categories` CSV). Envoyez ensuite un plan à `ai/apply` seulement après confirmation humaine et avec les deux révisions fournies par le snapshot.

Un export est asynchrone, puis téléchargeable en ZIP lorsqu'il est prêt. Un import est une transaction en quatre temps : ZIP vers aperçu, aperçu vers plan de mapping profils/racine, plan confirmé vers application, ou annulation. Limites : ZIP compressé 1 Gio, décompressé 2 Gio, 20 000 entrées; les vidéos et secrets ne sont jamais importés ni exportés. Un plan est temporaire et peut expirer; un `409 IMPORT_PLAN_STALE` impose de refaire l'aperçu.

## Compatibilité et limites

Le serveur ne promet pas un codec particulier dans MKV/MP4/AVI : le décodage et les pistes sont gérés par la machine cliente. Les sous-titres externes sont des ressources protégées par le bail, limitées à 16 Mio. Les affiches chargées sont JPEG/PNG/WebP, limitées à 8 Mio. Les appels JSON ordinaires sont limités à 1 Mio.

Avant une mise à jour de client, lisez `GET /server` et n'activez une capacité que si elle est annoncée. Avant une intégration de production domestique, testez les erreurs Range, l'expiration du bail, la reprise après transfert, une modification concurrente, et une connexion TLS avec certificat renouvelé.
