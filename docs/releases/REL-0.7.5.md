# 0.7.5 — Accueil, collections et mises à jour / Home, collections and updates

Candidat en préparation ; aucun paquet de cette version déclaré publié ici.
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
sans notarisation Apple, macOS15+ Apple Silicon. Les paquets finaux et la preuve
publique seront ajoutés après construction et vérification réelle.
