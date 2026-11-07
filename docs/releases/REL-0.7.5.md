# 0.7.5 — Accueil, collections et mises à jour / Home, collections and updates

Publication du 7 novembre 2026 ; composants indépendants signés et versions ci-dessous.
Mac0.7.5/build15 et web0.7.5/séquence6 ; serveur0.7.4/séquence5 ;
ambiances0.7.1/séquence2 inchangées. API2026-10-14, schéma3 et rendu2 conservés.

## Français

Accueil remplace Toute la collection : Reprendre, Ajouts récents et Pour vous,
vingt éléments au maximum par rangée horizontale, séries regroupées. Reprendre
s'appuie sur une saison commencée mais inachevée. Les suggestions locales utilisent
les dernières entités vues datées ; les anciennes lectures restent sans date.
Les rangées vides sont masquées, la recherche et les catégories restent accessibles.

Jaquettes de saga/liste depuis les membres, recherche TMDB explicite ou import ;
appliquer confirme le choix sans changer les métadonnées des films. Gestion et
suppression visibles, confirmation nommée conservant les vidéos. Curseur masqué
avec les commandes pendant la lecture, rétabli au mouvement/pause/fermeture.

Résultat des contrôles de mise à jour affiché même sous focus ; anciennes cartes
retirées après changement de serveur. Le helper corrige le résumé illisible sous
UMask0077 par publication0640. Le nouveau helper est inclus et signé avec le
paquet serveur ; le mécanisme actuel ne remplace pas lui-même un ancien helper.
Une installation existante nécessite son remplacement administrateur vérifié,
sans réutiliser le wrapper réservé à la première installation. Publier n'effectue
aucune installation directe ni modification du Pi/NAS.

## English

Home replaces Entire library with Continue watching, Recently added and For you:
up to20 entries per horizontal shelf, with one card per show. Continue watching
requires a started, unfinished season. Local recommendations use recent dated
watched entities; historical views receive no invented dates. Empty shelves are
hidden; search and categories remain available.

Saga/playlist artwork can use a member, an explicit TMDB search or an import.
Selection requires confirmation and never changes member metadata. Management
and named deletion confirmation are visible and keep video files. Playback hides
the pointer with its controls and restores it on movement, pause or closure.

Update results stay live while a policy field is focused; obsolete server cards
are removed. The helper publishes its availability snapshot0640 despite UMask0077.
The signed server package includes the new helper; the current mechanism does
not replace an older helper itself. Existing installations require verified
administrator replacement; the first-install-only wrapper must not be reused.
Publication performs no direct Pi/NAS installation or settings change.

## Vérifications et limites / checks and limits

Source fonctionnelle approuvée `a0a47ca`, preuve complète :
[Accueil/collections/mises à jour](../implementation/HOME-COLLECTION-UPDATE-EVIDENCE.md).
396 tests Node et397 tests Rust passent (deux tests manuels préexistants ignorés),
Clippy strict et formatage PASS, revue indépendante APPROVED. Rendu partagé
ordinateur/mobile contrôlé sur données synthétiques. Pas de test de nouveau
binaire Mac sur machine propre ni de téléchargement TMDB avec la clé familiale.

Les archives portables ne transportent toujours pas les durées mesurées selon
ADR0035 : une reprise peut disparaître jusqu'à nouvelle lecture. Les sauvegardes
SQLite complètes les conservent. Le nouvel horodatage est nullable et portable ;
les anciens importeurs stricts refusent une archive contenant ce nouveau champ.
Accueil borné à10000 vidéos avec erreur explicite au-delà. Signature Mac ad hoc,
sans notarisation Apple, macOS15+ Apple Silicon. Paquets finaux : Mac0.7.5/build15 signé ad hoc,50 bibliothèques ARM64 vérifiées ; serveur/helper ARM64 glibc 2.28, 14 contrôles TLS/HTTP/Home dans la VM Linux ; web0.7.5. Inventaires et signatures Minisign vérifiés. Source immuable : `52fc3999b312a472efa463c6eb6a286a1026694b`. Le helper nécessite un remplacement administrateur sur une installation existante.


## Publication vérifiée / Verified publication

Source immuable `52fc3999b312a472efa463c6eb6a286a1026694b`, fusionnée/poussée sur main.
Revue indépendante source et inventaires : APPROVED_FOR_PUBLICATION.
Tags Mac/web0.7.5, serveur0.7.4 et récapitulatif v0.7.5 ; thèmes inchangés.
Tous les inventaires publics, digests, téléchargements et signatures concordent ;
v0.7.5 est Latest. Aucun tag ou asset antérieur remplacé.

[Release 0.7.5](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7.5).

CodeQL PASS : https://github.com/ungeekrange-cmyk/chillcenter/actions/runs/42423193938 ;
Shipping dependency security PASS : https://github.com/ungeekrange-cmyk/chillcenter/actions/runs/42421847420 .
Ce scan porte sur `eaab2919d5fc5f26236911796a5241be8388dddc` ; les dépendances
et Cargo.lock sont inchangés dans la source publiée `52fc399`.
Les 36 contrôles documentaires, de signature/version et de manifeste source passent.

| Paquet | SHA-256 |
| --- | --- |
| `Chill-Center-0.7.5-macos-arm64.dmg` | `6fe15f898630898d75fdeca8b8a827ab6e52c579122c8101530085257f3d791b` |
| `Chill-Center-server-0.7.4-linux-aarch64.tar.gz` | `0e685b6bc0902bcb0dff3f7c32dce63310d04f8859c4a78f2070494359ebf38b` |
| `Chill-Center-web-0.7.5-install.zip` | `780deb4e9bc4c782435588289b354d3c5c381088b990a0ff97060072f61858d8` |
| `Chill-Center-0.7.5-corresponding-sources.tar.gz` | `a4a5f4a778b3a75e0c56d179f0082f94262ab6253f0da8906499280bbd6da15a` |

Vérification publique : 35 fichiers uniques, 51 assets dans quatre releases.
La VM Linux de test a été arrêtée. Sources, livraisons et caches conservés ;
aucune purge. Aucun déploiement sur le Pi, NAS ou Mac utilisateur effectué.
Les politiques automatiques existantes peuvent détecter ces publications.
