# BUG-0069 — Adaptateur web incompatible avec la nouvelle interface commune

2026-10-15, intégration locale `codex/mac-web-compat-20261015`, non déployée.

Symptôme reproductible par contrat : le Mac11aa529 appelle `sync_catalog` et `season_artwork`, absents de l'adaptateur web issu de son ancêtre a59f5e6. Une fusion Git sans conflit ne suffit pas : l'interface commune reçoit `WEB_UNSUPPORTED` au lieu d'une vérification de fraîcheur. L'adaptateur mémorise également les Blob URL d'affiches sans les invalider après mutation ; une réponse tardive peut repeupler ce cache après changement de profil.

Hypothèse confirmée par lecture des commandes et tests ciblés, sans serveur réel ni lecteur. Correction : GET de révision minuscule réutilisant le curseur serveur existant, comparaison par serveur/profil, rejet des réponses hors contexte, repli conservateur uniquement sur `404 ROUTE_NOT_FOUND`. `season_artwork` utilise la route API fixe validée. Les mutations/changements de contexte/révision révoquent les URL ; une génération empêche une ancienne requête d'en recréer après invalidation.

Régression finale :16 tests `web/test/bridge.test.js` PASS, dont révision inchangée/modifiée, ancien serveur, profil changé, image tardive après mutation et changement de profil. Test serveur dédié vérifiant réponse bornée, portée personnelle, alias et absence de checkpoint PASS. Voir [preuve d'intégration](../implementation/MAC-WEB-COMPATIBILITY-20261015.md).

La revue indépendante du candidat1b251cd a trouvé un P1 : révision mémorisée avant réussite du chargement, donc erreur de page suivie d'une fausse réponse « inchangé ». Correction locale : ticket d'application privé au bridge, acquitté par l'interface seulement après publication complète des pages ; échec conserve le catalogue précédent et n'acquitte pas la révision. Un ticket ancien ou invalidé par mutation/contexte est rejeté. Aucune commande supplémentaire n'est envoyée au Mac natif, qui ne produit pas de ticket. Tests ajoutés : échec503 puis réessai de la même révision, ticket ancien, succès/échec du chargement paginé et absence d'acquittement en échec. Ce résultat défavorable initial reste conservé dans la revue.

Limites : le navigateur n'acquiert pas de miroir SQLite ; TMDB/IA/choix de fichier natif non portés restent explicitement non supportés comme avant. Aucun déploiement ni nouveau test vidéo.
