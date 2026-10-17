# ADR0025 — Interface commune Mac/web, adaptateurs propres à la plateforme

- Statut : ACCEPTED, périmètre demandé et exclusion IA confirmés par l'utilisateur.
- Date : 2026-10-17 ; complète ADR0019 miroir web, sans remplacer le contrat API.
- Décision : réutiliser `ui/connected` pour les vues, filtres, fiches, séries,
  collections, configuration, langues et récupération des conflits. Conserver
  les adaptateurs Rust/libmpv du Mac et navigateur/IndexedDB/WebCodecs du web.

La branche web e6e23db est intégrée par merge à la base Mac 3b2e28b, pas par
écrasement des fichiers récents. L'autre worktree et les versions livrées restent
intacts. L'API `/api/2026-10-14` demeure l'unique cible du navigateur ; `/api/v1`
et les contrats serveur ne sont pas modifiés. Aucun déploiement implicite sur Pi.

Le web préfère les deltas existants `catalog/changes`, avec miroir publié après
validation complète et atomique ; repli snapshot seulement si la route est
réellement absente. Bornes de transfert 10 000 entrées / 32 Mio UTF-8, pages de
100 au plus. Les séries utilisent les clés opaques, jamais le seul titre pour
fusionner des homonymes. Les métadonnées de saisons suivent le document de sync.

Les imports d'affiches passent par le sélecteur navigateur, avec limites MIME et
taille et garde de contexte. Les archives gardent aperçu, plan et confirmation
explicite : l'export est remis au gestionnaire de téléchargement du navigateur,
sans copie ZIP complète en RAM ; l'import est un travail annulable. Un conflit
CAS n'entraîne aucune réémission automatique d'écriture ni écrasement forcé.

L'IA (organisation ET demande de conseil) est exclue du client web sur choix
explicite utilisateur. Les recommandations locales par métadonnées restent
disponibles. Les fonctionnalités Mac ne sont pas retirées. La parité de vues
ne promet pas une équivalence de codecs, d'intégration système, de TLS ou de
plein écran : ces capacités restent propres au navigateur et au Mac.

Retour arrière : ancien bundle Mac et anciens fichiers web conservés, aucun
schéma serveur migré. Revenir à la version précédente ne réécrit pas le NAS.
Preuves et limites dans le rapport bilingue WEB-PARITY-20261017.
