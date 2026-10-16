# Correctif décor / vidéo native — remise de test

Source produit : **bdce3a463fd19b05ffcc4de37dcb082884f3b276**, branche `codex/fix-player-event-overlay`, parent `fd2b7a9`. Seul changement produit par rapport au parent : deux lignes de `ui/connected/events.css`. Aucun moteur, protocole, API, serveur ou code web en cours de développement modifié.

[BUG-0078](../debug/BUG-0078-event-artwork-covers-native-video.md) : décor Noël/Halloween conservé au-dessus de la vue vidéo native par WKWebView. Suppression du pseudo-fond uniquement pendant `body.player-open`, restauration automatique en quittant le lecteur. Diagnostic H1 unique confirmé, correction par un builder distinct.

## Preuves

- Test `scripts/test-event-player-cutout.cjs` : **RED** sur parent, **GREEN** après suppression temporaire du seul décor en mémoire, puis **GREEN** sur le vrai correctif figé. Deux ambiances × deux tons × panneau droit/plein écran ; décor restauré, découpe transparente.
- Régression des ambiances existante : PASS. Suite ciblée catalogue/filtres/synchronisation et site facultatif : **26/26 PASS**. Aucun parcours complet de lecture vidéo rejoué, aucun appel Pi/NAS.
- Revue indépendante source `bdce3a4` : **APPROVED_SCOPED**, règle CSS conforme à l’ordre natif des vues, quatre défauts connexes correctement gardés OPEN. Le reviewer n’a pas pu rejouer Playwright dans son environnement ; exécution par le builder et le coordinateur consignée séparément, pas transformée en preuve de lecture Tauri.
- Gitleaks historique de cette source : six alertes historiques, mêmes fixtures TMDB déjà classées indépendamment, aucune nouvelle alerte. Aucun secret réel identifié ; aucun accord de publication de l’historique privé.

## Bundle Mac de test

Chemin relatif au projet : `artifacts/macos/Chill Center-test-bdce3a4.app`.

- Apple Silicon, **81 Mio**, **48 dylibs arm64**, libmpv API **2.5**.
- SHA-256 exécutable : `6aa696eeff5dcdafbdd9fc9c456f699c1e56ff24e4336fdd974163c72c1f5a5f`.
- Signature ad hoc profonde et stricte vérifiée après publication locale du bundle ; `--verify-bundle` PASS sans ouvrir l’interface ou le réseau.
- Identité interne conservée **0.5.1 / build 6** : c’est un candidat de test nommé par sa source, **pas une nouvelle release ni un remplacement du tag/bundle 0.5.1**. Ne pas le confondre avec le bundle figé portant le nom `Chill Center-v0.5.1.app`.
- L’ancien exécutable 0.5.1 conserve son SHA `815c2a0c76e70c533fa909fcd6fc9d23fe72e8eb0ca4cd1b5e978ff783e35799` ; tags 0.5 et 0.5.1 inchangés.

Commande de construction : `bash scripts/package-macos.sh <destination-neuve-test-bdce3a4.app>`, caches Cargo existants explicitement réutilisés. Compilation release terminée, deux avertissements `dead_code` dans les sondes natives (`nonblank`, `glReadPixels`) conservés ; pas déclarée sans avertissement. Aucun bundle écrasé, aucune installation automatique ni ouverture d’application.

**À vérifier humainement** : dans ce bundle de test, ouvrir une vidéo avec Halloween puis Noël, en panneau droit et plein écran, puis revenir au catalogue. La validation automatique prouve l’absence du décor dans la découpe DOM, pas le rendu natif final ni l’écoute audio.

## Autres défauts conservés ouverts

BUG-0079 ancienne affiche sans match (réapparition de BUG-0065), BUG-0080 clipping au scroll, BUG-0081 confirmation/traitement TMDB, BUG-0082 action de récupération des conflits ; puis BUG-0083 loupe et BUG-0084 rotation de synchronisation. L’utilisateur a demandé de tracer le reste : aucune correction de ces six défauts n’est revendiquée. Voir [l’index](../debug/INDEX.md).

Retour arrière : utiliser le bundle figé précédent, sans restaurer de base ni modifier la configuration. Serveur, NAS, autre chantier web et branche active du dépôt canonique intacts. Les branches ont été enregistrées localement dans le dépôt principal, sans push GitHub ; aucune fusion forcée dans une branche active.
