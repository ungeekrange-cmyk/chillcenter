# Architecture — référence v0.6

**Français** | [English](OVERVIEW.en.md)

Synthèse documentaire de l'état livré, pas une nouvelle décision ni une preuve
de test. Les [ADR](decisions/INDEX.md), l'[architecture historique approuvée](ARCHITECTURE.md)
et les [notes v0.6](../releases/REL-V0.6.md) restent les sources de traçabilité.
Lorsqu'un comportement historique a été remplacé, appliquer l'ADR ultérieur.

## Responsabilités

Le serveur Linux Rust (Axum/Tokio/rustls) lit les bibliothèques configurées,
conserve le catalogue et les données personnelles en SQLite, fournit l'API,
les affiches et les octets vidéo par HTTPS. Le NAS est monté par Linux,
jamais par le client. Aucun transcodage vidéo ne se fait sur le serveur.

Le client Mac Apple Silicon utilise Tauri et une interface HTML/CSS/JavaScript
sans framework supplémentaire. Rust porte le transport HTTPS, la confiance,
le cache, les règles et l'adaptation du lecteur natif libmpv/FFmpeg.
La webview ne doit pas partager un verrou bloquant avec les téléchargements
de sous-titres ou la synchronisation du catalogue.

Le workspace sépare `chill-domain` (règles/contrats), `chill-catalog`
(scan/métadonnées/persistance), `chill-server` (API/TLS/baux/flux),
`chill-client` (transport/cache/réglages/IA), `chill-player` (libmpv)
et `apps/desktop` (intégration Tauri). Le chantier web reste indépendant.

## Synchronisation et identité

Le miroir client est privé par serveur et profil. Les grilles et fiches lisent
ce miroir, pas le NAS ; les vidéos ne sont jamais enregistrées dans ce miroir.
Les curseurs opaques et deltas durables permettent une vérification légère.
Un delta complet se publie atomiquement, sans retirer l'ancien catalogue
pendant son chargement ; un échec conserve la dernière copie complète.
Les téléchargements et animations de statut ne bloquent pas la navigation.

Ne pas identifier une série uniquement par son titre. Une clé opaque permet
de distinguer les homonymes ; `selector=key` est explicite. Le comportement
historique par titre reste supporté pour les anciens clients. Les sagas sont
communes ; playlists, notes, vus et reprises sont propres au profil.

## Lecture et ressources

La lecture exige un bail serveur lié au profil, au média, à sa révision et à
une génération. Le transport Rust fournit des plages HTTP au lecteur natif.
L'interface gère les pistes, la progression et l'enchaînement ; le serveur
arbitre l'exclusivité du profil et persiste la progression vérifiée.

Le client Mac conserve moteur et tampon pendant une pause (ADR0022), avec
renouvellement du bail ; un transfert ou une échéance impose toujours l'arrêt.
Le cache vise 60 s devant et environ 30 s derrière, avec plafonds configurables
en Mio : les durées ne sont pas garanties à tout débit. Les sous-titres
externes sont téléchargés de façon bornée et annulable, sans pause vidéo.

Les mesures RAM et la déduplication d'affiches sont documentées séparément.
Un benchmark synthétique n'est ni une mesure de l'application native complète
ni une garantie de plusieurs flux sur le Wi-Fi du Pi.

## Sécurité et compatibilité

Réseau local de confiance, sans authentification : les profils ne cloisonnent
pas l'accès des membres du foyer. HTTPS et confiance initiale du certificat
restent nécessaires. Le renouvellement garde la clé approuvée ; l'exception
acceptée après expiration de l'ancien certificat comporte un risque d'usurpation
LAN, décrit dans ADR0005. Aucune désactivation globale de TLS.

Seule la base `/api/2026-10-14` est servie depuis le serveur0.7.1 ; l’ancien
alias `/api/v1` est supprimé à la demande explicite de l’utilisateur (ADR0031).
Les ajouts compatibles restent dans cette version ; aucune seconde API parallèle.
Lire le [guide API](../api/INTEGRATION-FR.md) et l'OpenAPI avant une intégration.
Les clés et adresses réelles restent hors Git ; les exports excluent les secrets.

## Documentation et livraison

Langue d'interface locale `auto/en/fr` ; métadonnées utilisateur inchangées.
Les références documentaires sont disponibles dans les deux langues via
l'[index](../README.md). Les décisions et preuves historiques gardent leur langue
et leur statut d'origine. Ce complément ne recompile ni ne redéploie v0.6 et
ne change pas les fichiers du client web.
