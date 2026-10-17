# Revue indépendante / Independent review — web mobile content-first

Date : 2026-10-17. Base exacte :
`a7b00824482e016ecec141823dc9ecd86554d785`. Candidat non encore commité,
build reproductible `95ebee4fbe0a`.

Sources principales revues : `mobile-ui.js` `1dcabc74…f276`, `mobile.css`
`84504f2d…a17d`, `mobile-playback.js` `c765f04b…f8c`, `mse-engine.js`
`bcdb9ad8…a17d`, `player-web.js` `71c930d7…c5d3`.

## Français

**APPROVED_SCOPED** — aucun défaut bloquant ou majeur ne reste dans le périmètre
du client web mobile examiné. Le diff reste limité à `web/` et aux preuves web :
aucun fichier du client Mac partagé, du serveur, de l’API, de la configuration ou
du NAS n’est modifié.

Le rendu « contenu d’abord » masque par défaut la recherche et les filtres sur
smartphone, les ouvre avec une commande accessible et les referme lors d’une
navigation sans perdre leurs valeurs. Le sélecteur d’ajout conserve ses propres
filtres. En paysage sur téléphone à pointeur tactile, la navigation devient un
rail gauche ; le bureau retrouve l’ordre et la présentation existants. Les
outils de saison et de série sont déplacés, sans recréer leurs écouteurs, dans
des sections repliables après les épisodes, puis restaurés dans leur ordre
bureau. Les chemins Films, Séries, Docs, Spectacles, Plus, sagas, listes,
configuration et profil restent atteignables en français et en anglais.

La lecture préfère `MediaSource` et n’emploie `ManagedMediaSource` qu’en son
absence. Dans ce repli iPhone, la lecture distante est désactivée avant
l’attachement de la source. Un refus ou une interruption courante de `play()`
conserve le média et le tampon, publie un état récupérable et affiche un bouton
de reprise ; une pause, fermeture ou tentative plus récente neutralise les
rejets tardifs. Le geste de reprise appelle `play()` et, lorsque nécessaire,
`webkitEnterFullscreen()` de manière synchrone avant tout `await`. Le plein
écran, son nettoyage tardif et l’aide à l’orientation restent limités au
téléphone, et l’aide disparaît déjà en paysage.

Les défauts trouvés pendant la revue ont été corrigés avant ce verdict : état de
recherche mobile survivant au retour bureau ; sélecteur masqué par une règle CSS
trop large ; reprise possible pendant une pause ordonnée par le serveur ou après
invalidation du bail ; `AbortError` courant silencieux laissant un écran noir.
Les deux chemins de reprise passent désormais par la même garde
`engine + lease + !serverPaused + !transfer`, appliquée avant l’ACK réseau.

Preuves indépendantes : `git diff --check` PASS ; contrôles syntaxiques
JavaScript PASS ; sous-ensemble de revue **168/168** tests web PASS, dont les courses pause/ACK,
invalidation de bail, tentatives `play()` tardives et repli MMS ; QA Chromium du
build PASS à 320/390/430/736/844 px et retour bureau à 1280 px ; QA lecture
synthétique PASS pour MKV H.264/AAC et AVI Xvid/MP3, reprise sans réouverture du
flux et câblage `ManagedMediaSource`. Le build reproduit bien
`95ebee4fbe0a` (80 fichiers). La preuve d’intégration finale réutilisée est
**304/304** tests PASS et trois QA PASS (mobile R2, parité bureau, clips AVI/MKV).

Limite importante : ces preuves ne remplacent pas un essai sur iPhone réel.
Safari/Vivaldi iOS, `ManagedMediaSource`, le MP3 Safari, le verrouillage paysage
et les deux fichiers utilisateurs signalés n’ont pas été exécutés sur appareil.
Le correctif traite les causes visibles et fournit un repli explicite, mais le
test post-déploiement sur iPhone reste nécessaire avant d’affirmer que les écrans
noirs AVI/MKV sont définitivement résolus. Le paquet et le déploiement Pi ne
sont pas couverts par ce verdict. `BUG0110` doit donc rester
`FIXED_IMPLEMENTATION / VERIFIED_SYNTHETIC / DEVICE_RETEST_PENDING`, et non
fermé ; la prise en charge MP3 dépend notamment du `isTypeSupported` exposé par
Safari sur l’appareil, sans transcodeur MP3 de repli actuel.

## English

**APPROVED_SCOPED** — no blocking or major issue remains in the reviewed mobile
web-client scope. The diff is confined to web sources and web evidence; shared
Mac UI, server, API, configuration, and NAS paths are unchanged.

The content-first layout keeps phone search and filters collapsed behind an
accessible control, closes them on navigation without clearing values, leaves
picker filters intact, uses a phone-only left rail in touch landscape, and
restores the existing desktop order. Season and series tools are moved with
their existing listeners into collapsed details below episodes, then restored
for desktop. French and English navigation retains all catalogue, More,
collection, settings, and profile destinations.

Playback prefers classic `MediaSource`, falls back to `ManagedMediaSource` only
when needed, and disables remote playback before attaching that managed source.
Current blocked/interrupted `play()` attempts preserve buffers and expose a
recoverable retry; stale settlements after pause, retry, or close are ignored.
The fresh touch invokes play and the iPhone video-only fullscreen fallback
synchronously before any await. Resume is guarded against pending transfer,
server-ordered pause, and invalid leases before the network ACK completes.

Independent evidence: `git diff --check` and JavaScript syntax checks PASS; the
review subset has **168/168** web tests PASS; built-client Chromium QA PASS at
320/390/430/736/844 px and 1280 px desktop restoration; synthetic playback QA
PASS for H.264/AAC MKV, Xvid/MP3 AVI, buffered touch retry, and MMS fallback
wiring. The reproduced 80-file build is `95ebee4fbe0a`. Reused final integration
evidence reports **304/304** tests PASS plus three QA runs (mobile R2, desktop
parity, AVI/MKV clips).

Material limitation: no physical iPhone ran this review. Safari/Vivaldi iOS,
real `ManagedMediaSource`, Safari MP3 behavior, real orientation lock, and the
two reported user files remain device-validation items. The implementation is
consistent with the identified causes and now fails visibly, but a post-deploy
iPhone smoke test is required before claiming the AVI/MKV black screens fully
resolved. Package integrity and Pi deployment are outside this verdict.
`BUG0110` must remain `FIXED_IMPLEMENTATION / VERIFIED_SYNTHETIC /
DEVICE_RETEST_PENDING`, not closed: Safari MP3 support still depends on the
device's `isTypeSupported` result, with no current MP3 transcoder fallback.

## Annexe paquet / Package appendix

Le paquet figé au commit
`8b950334a17641aaa1a20e2129ccfe9a449b813a` est **APPROVED_SCOPED** pour
une installation des seuls fichiers web avec sauvegarde. Archive :
`chillcenter-web.tar`, SHA256
`0da5d3962cbccabc966f71f1b9901aa61c4a2ef74508bb82ba1909694a1d7a50`,
6 628 864 octets, build `95ebee4fbe0a`.

Les 88 membres sont exactement 82 fichiers réguliers et 6 répertoires sous
`web/`, sans lien, nœud spécial, doublon, chemin absolu ou remontée. Les 81
entrées de `SHA256SUMS` couvrent exactement tous les fichiers hors manifeste et
toutes les empreintes passent. L’arborescence hors manifeste est octet pour
octet identique à `web/dist`; `build.json` confirme `95ebee4fbe0a`. La taille
reste sous la limite de 16 Mio.

L’opérateur inchangé
`b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996`
passe `bash -n` et `--self-test`. Il vérifie le binaire installé sans le
remplacer, prépare et contrôle les fichiers dans un répertoire privé, déplace
l’ancien client dans `/root/chillcenter-rollback`, remplace uniquement
`/usr/lib/chillcenter/web`, puis contrôle le même PID, les empreintes de la
configuration/unité, la clé publique TLS et les routes HTTPS/HTTP. Son piège de
récupération remet l’ancien répertoire en cas d’échec. Il ne redémarre pas le
service et ne modifie ni binaire, configuration, certificat, NAS ou client Mac.

The frozen package at commit
`8b950334a17641aaa1a20e2129ccfe9a449b813a` is **APPROVED_SCOPED** for a
backed-up web-assets-only installation. `chillcenter-web.tar` has SHA256
`0da5d3962cbccabc966f71f1b9901aa61c4a2ef74508bb82ba1909694a1d7a50`,
size 6,628,864 bytes, and build `95ebee4fbe0a`.

Its 88 members are exactly 82 regular files and 6 directories below `web/`,
with no link, special node, duplicate, absolute path, or traversal. The 81-line
manifest covers every non-manifest file, all hashes validate, and the tree is
byte-identical to `web/dist` except for `SHA256SUMS`. The unchanged operator
hash above passes syntax and self-test, keeps a private rollback copy, swaps only
the static web directory, verifies the live service without changing its PID,
and proves binary, unit, configuration, TLS public key, NAS, and Mac client are
untouched.
