# 0.7.7 — Reprises héritées dans l’Accueil / Legacy Home resumes

## Français

Candidat de release préparé le 9 novembre 2026, non publié et non installé.
Versions réservées : Mac0.7.7/build17/séquence8, serveur0.7.6/séquence7 et
web0.7.6/séquence7. Les thèmes restent0.7.1/séquence2. Les paquets actuellement
publics restent Mac0.7.6, serveur0.7.5, web0.7.5 et thèmes0.7.1.

BUG0140 corrige une reprise héritée valide : une vidéo disponible, non vue et
avec une position strictement positive apparaît dans « Reprendre » même si son
ancienne importation ne contient pas de durée mesurée. `progress` reste `null`
dans ce cas ; aucune durée TMDB, aucun pourcentage et aucune donnée personnelle
ne sont inventés ou modifiés. Les séries conservent leur regroupement par saison
et les plafonds de l’Accueil restent inchangés.

Le raccourci redondant « En cours » est retiré des clients Mac, web et mobile.
Les barres et pourcentages fondés sur une durée réellement mesurée restent dans
les fiches et vignettes ; seuls le raccourci et son filtre distinct disparaissent.

Contrôles déjà réussis sur le candidat source `f38be58` : 83 tests domaine,
1 test HTTP, 368 tests JavaScript, 27 tests documentaires, Clippy strict et
Chromium bureau/mobile. La revue indépendante est APPROVED. Ces contrôles ne
constituent ni une publication ni une installation. Aucun changement de schéma,
de migration, de NAS ou d’historique de production n’est requis.

## English

Release candidate prepared on 9 November 2026; it is neither published nor
installed. Reserved versions are Mac0.7.7/build17/sequence8,
server0.7.6/sequence7 and web0.7.6/sequence7. Themes remain0.7.1/sequence2.
Currently published packages remain Mac0.7.6, server0.7.5, web0.7.5 and
themes0.7.1.

BUG0140 fixes a valid legacy resume: an available, unwatched video with a
strictly positive position appears in Continue watching even when its historical
import has no measured duration. `progress` remains `null` in that case; no TMDB
duration, percentage or personal data is invented or changed. Shows keep their
season grouping and Home limits are unchanged.

The redundant In progress shortcut is removed from the Mac, web and mobile
clients. Progress bars and percentages based on a genuinely measured duration
remain on cards and detail views; only the shortcut and its separate filter go
away.

Checks already passed on source candidate `f38be58`: 83 domain tests, 1 HTTP
test, 368 JavaScript tests, 27 documentation tests, strict Clippy and desktop/
mobile Chromium. Independent review is APPROVED. These checks are not a
publication or installation. No schema change, migration, NAS access or
production-history change is required.
