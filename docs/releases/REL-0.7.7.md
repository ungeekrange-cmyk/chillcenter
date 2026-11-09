# 0.7.7 — Reprises héritées dans l’Accueil / Legacy Home resumes

## Français

Publié le 9 novembre 2026 : Mac0.7.7/build17/séquence8 (`mac-v0.7.7`),
serveur0.7.6/séquence7 (`server-v0.7.6`) et web0.7.6/séquence7 (`web-v0.7.6`).
La release récapitulative `v0.7.7` est Latest. Les thèmes restent0.7.1,
API2026-10-14 et schéma3 inchangés. Les quatre releases contiennent51 assets,
dont35 fichiers uniques retéléchargés et vérifiés publiquement, avec leurs
signatures. Les tags et paquets sont liés à la source immuable
`5d0aa974dc7b626991fc3f177b7e3ff03b66f77f` ; les corrections documentaires
postérieures ne déplacent aucun tag.

BUG0140 rend visible dans Reprendre une vidéo disponible, non vue et ayant une
position positive, même sans durée mesurée historique. `progress` reste `null`
dans ce cas : aucun pourcentage ou temps total inventé. Le regroupement des
séries par saison et le plafond de20 cartes par rangée restent conservés.
Le raccourci redondant En cours disparaît des clients Mac, web et mobile.
Les pourcentages fondés sur une durée réellement mesurée restent disponibles.

Installations réelles terminées : serveur et web0.7.6 sur le Pi, séquence7 ;
Mac0.7.7 dans Applications, installé depuis l’icône de mise à jour du client
0.7.6 et relancé automatiquement, sans seconde confirmation. La GUI montre les
trois versions, un catalogue à jour et le film hérité vérifié dans Reprendre.
La reprise de3 025 917 ms est conservée, avec `progress:null`. Aucun lancement
de vidéo, changement d’historique, migration ni modification du NAS.

Le helper, les configurations et les unités systemd du Pi sont conservés.
Sauvegarde complète avant remplacement du serveur :1 926 fichiers,
92 658 842 octets, reçu `9b7a31cf-4eef-459e-96cd-0887f6126f09`.
Santé HTTPS avec CA épinglée, identité serveur, fichiers HTTP web conformes au
paquet signé, lecture0640 du résumé de disponibilité et service/timer actifs
vérifiés. Les anciens slots et sauvegardes du Pi sont conservés pour récupération.
Une récupération administrateur doit respecter les reçus anti-retour arrière ;
aucune désactivation des signatures n’est autorisée.

## English

Published on 9 November 2026: Mac0.7.7/build17/sequence8 (`mac-v0.7.7`),
server0.7.6/sequence7 (`server-v0.7.6`) and web0.7.6/sequence7 (`web-v0.7.6`).
The summary release `v0.7.7` is Latest. Themes remain0.7.1, with unchanged
API2026-10-14 and schema3. All51 assets across four releases, representing35
unique files, were downloaded publicly and verified with their signatures.
Immutable package/tag source: `5d0aa974dc7b626991fc3f177b7e3ff03b66f77f`.
Later documentation commits do not move these tags.

BUG0140 shows available, unwatched videos with a positive saved position on
Continue watching, even without a historical measured duration. `progress`
remains `null`; no duration or percentage is invented. Season grouping and
the20-card shelf limit remain unchanged. The redundant In progress shortcut
is removed from Mac, web and mobile clients; measured percentages remain.

Real installations are complete: server/web0.7.6 sequence7 on the Pi and
Mac0.7.7 in Applications. The Mac0.7.6 update icon installed the new app and
restarted it automatically without another confirmation. The GUI shows all
three versions, an up-to-date catalogue and the verified legacy film on Continue
watching. Its3,025,917 ms resume position is unchanged, with `progress:null`.
No video was launched and no history, schema or NAS data was changed.

The Pi helper, configurations and systemd units are unchanged. The complete
pre-install server backup contains1,926 files and92,658,842 bytes, receipt
`9b7a31cf-4eef-459e-96cd-0887f6126f09`. Pinned-CA HTTPS health, server identity,
served web bytes, readable0640 availability state and active service/timer
were verified. Pi rollback slots and backups remain available; administrator
recovery must honor anti-rollback receipts and signature validation.

## Vérifications / Verification

Exact source5d0aa97:283 Rust tests (83domain,155server,2server integration,
43desktop),3 native SDK install/tamper/restart tests,368 JavaScript tests,
27 bilingual documentation tests, desktop/mobile Chromium PASS. Strict
domain/server Clippy passed on functional sourcef38be58. The initial sandbox
TCP-bind EPERM disappeared when the identical suite ran with local-network
permission. Independent source and artifact reviews: APPROVED_SCOPED.
CodeQL42609140841 and shipping dependency security42609141969 passed on5d0aa97.

Apple Silicon Mac requires macOS15 minimum:51 Mach-O minima,50 relocated
libraries, signature and DMG checks passed. The signature is ad hoc, without
Apple notarization; physical macOS15/26 execution remains untested. Linux ARM64
binaries require glibc2.28 minimum; real host verification used Debian13.
GNU corresponding sources include the exact1159-file Git source manifest,
54 upstream inputs and42 runtime formulas; all248 internal hashes verified.

[BUG0140](../debug/BUG-0140-home-legacy-resume.md) ·
[ADR0041](../architecture/decisions/ADR-0041-home-resume-without-duration.md) ·
[Installation FR](../updates/GUIDE-FR.md) · [Installation EN](../updates/GUIDE-EN.md)
