# Revue de sécurité indépendante — correctif QA de visibilité native

- Date : 2026-10-14
- Candidat exact : `9277ac4d51281b3e6d72475294b1e7822cb8e913`
- Base examinée : `db9455cf202a59be7ec17079ab3ad75cbcb07d44`
- Verdict : **APPROVED**
- Constatations HIGH/CRITICAL : **0**
- Autres constatations de sécurité : **0**

## Portée et conclusion

La revue couvre uniquement le delta exact ci-dessus. Les ajouts préparent la fenêtre native avant le démarrage du scénario QA lecteur, attendent le retour réel de visibilité avant les temporisations et ajoutent des traces bornées. Ils ne modifient ni le comportement produit du lecteur, ni l'API, ni TLS, ni TMDB, ni les dépendances.

Le correctif est acceptable pour la sécurité : la surface QA et l'appel AppKit sont absents des builds release, l'appel Objective-C est exécuté sur le thread principal et ne conserve aucun pointeur externe, et les nouvelles traces ne contiennent que des libellés de phase constants, un temps écoulé et un booléen de visibilité.

## Éléments vérifiés

### Frontière debug/release et entrées

- `apps/desktop/src/main.rs:548-607` place le gestionnaire de chargement, `qa_deadline_probe` et `qa_result` derrière `#[cfg(debug_assertions)]` ; `apps/desktop/src/main.rs:608-615` confirme que le handler release n'expose que les commandes produit.
- La préparation de visibilité n'est appelée que si `CHILL_UI_QA_URL` et `CHILL_UI_QA_ROOT` existent et si `CHILL_UI_QA_MODE == "player"` (`main.rs:563-571`). Ces variables proviennent du processus local de test déjà approuvé ; le delta n'ajoute aucune donnée distante ni redirection.
- `apps/desktop/src/native_player.rs:307-330` protège la fonction entière par `#[cfg(debug_assertions)]`. L'import `AnyObject` est lui aussi limité au debug (`native_player.rs:13-14`), sans avertissement ni symbole QA supplémentaire en release.
- Aucun nouveau `#[tauri::command]`, handler IPC, accès fichier, requête réseau, permission ou secret n'est ajouté.

### AppKit, thread et cycle de vie

- Les messages `sharedApplication`, `unhide:` et `activateIgnoringOtherApps:` sont regroupés dans `run_on_main_thread` (`native_player.rs:311-323`). La classe et le pointeur retourné sont contrôlés avant usage ; aucun pointeur n'est conservé au-delà du callback.
- L'effet se limite à rendre visible et active l'application de test courante, puis à afficher/focaliser sa fenêtre Tauri (`native_player.rs:324-329`). Il ne cible aucune autre application ni ressource système.
- L'ordre asynchrone éventuel entre l'activation AppKit et la visibilité effective ne devient pas une course de sécurité : `ui/connected/player-qa.js:7-12` attend `visibilitychange`, retire son écouteur après succès et ne démarre les étapes sensibles aux timers qu'ensuite. Il n'y a ni boucle ni minuterie nouvelle pendant l'état masqué.

### Journalisation et données sensibles

- Les nouvelles traces contiennent uniquement `phase`, `elapsed_ms` et `document_hidden` (`player-qa.js:6-7`). Elles n'incluent ni URL, racine NAS, identifiant de profil, média, jeton, certificat ou clé TMDB.
- L'échec de préparation expose une chaîne fixe (`qa visible preparation failed`) et pas l'erreur AppKit interne (`main.rs:568-570`).
- Le collecteur QA existant borne chaque détail à 300 caractères et traite `TRACE` comme non terminal (`main.rs:654-670`).
- Le scan du commit exact n'a trouvé aucun motif de clé privée, jeton GitHub/AWS ou clé de type `sk-`.

### Dépendances et étendue du delta

- Aucun changement de `Cargo.toml`, `Cargo.lock`, `package.json` ou `package-lock.json` entre la base et le candidat.
- Aucun changement serveur, API datée, TLS, fournisseur TMDB, stockage, profil ou logique produit du lecteur dans ce delta.
- Les rapports historiques ajoutés sous `docs/reviews/` sont des preuves archivées, pas une approbation transitive du présent candidat.

## Vérifications indépendantes

| Vérification | Résultat |
|---|---|
| `git rev-parse HEAD` et état du dépôt | `9277ac4d51281b3e6d72475294b1e7822cb8e913`, propre |
| `git diff --check db9455c..9277ac4` | PASS |
| `cargo check -p chillcenter-desktop --locked --offline` avec caches isolés | PASS, 4,41 s lors de la relance sur le SHA exact |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` | PASS, 47/47, 0 échec, 0 ignoré |
| `node --check ui/connected/player-qa.js` | PASS |
| `node --check ui/connected/player-qa.test.js` | PASS |
| Scan de secrets sur le commit exact | PASS, aucune correspondance |
| Changement de dépendances/verrous | Aucun |

## Limites et conditions du verdict

- Je n'ai pas exécuté le scénario natif A/B sur une application Tauri fraîche. Ces essais sont encore en attente ; cette approbation de sécurité ne constitue pas leur réussite et ne remplace pas la porte de validation native/test.
- La compilation vérifiée est le profil `dev`, pas un bundle release. L'exclusion release est établie ici par inspection des gardes `cfg(debug_assertions)` et du handler release distinct.
- Aucun accès au Pi, au NAS, aux identifiants, aux certificats réels, au fournisseur TMDB ou au déploiement n'a été effectué.
- Le verdict vaut exclusivement pour `9277ac4d51281b3e6d72475294b1e7822cb8e913`. Tout changement ultérieur de source, de configuration de build ou de permissions doit être réévalué.
