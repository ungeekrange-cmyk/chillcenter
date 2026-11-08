# Serveur 0.7.5 — délai de sauvegarde du Pi / Pi backup budget

Publication `server-v0.7.5` du 8 novembre 2026 : serveur0.7.5, séquence6,
source `956e155ff32497493848e6b1477fa1f9e61d9a2f`. Les 16 assets publics,
leurs sommes et leurs signatures ont été vérifiés dans
`/private/tmp/chillcenter-playback-wake.BEMWYx/server-0.7.5/public-verification.json`.
Mac0.7.6/build16 et web0.7.5 sont des publications séparées ; thèmes0.7.1
inchangés. Cette publication ne prouve pas une installation sur le Pi.

## Français

Sur un Pi à stockage lent, la sauvegarde durable de milliers d'affiches peut
dépasser deux minutes. Le helper la refusait alors correctement et rétablissait
l'ancien service, empêchant l'installation d'une mise à jour saine.
Le délai maximal de cette sauvegarde est porté à 15 minutes ; l'unité dispose
de 30 minutes pour l'opération complète. Les vérifications, reçus, sauvegardes,
quiescence, limites de sortie et mécanismes de retour arrière restent appliqués.

Le paquet signé contient le nouveau serveur et le nouveau helper, ainsi que
l'unité adaptée. Une installation existante nécessite le remplacement vérifié
du helper et de son unité par l'administrateur. L'installateur initial ne doit
jamais écraser une installation existante. API 2026-10-14 et schéma 3 inchangés.

## English

A slow Pi SD card can take more than two minutes to durably back up thousands
of poster files. The helper correctly rejected that unfinished backup and
restored the old service, preventing an otherwise valid update.
The offline backup deadline is now 15 minutes; the update unit allows 30 minutes
for the full operation. Signature, receipt, quiescence, output bounds and rollback
checks are preserved. Existing installations require verified administrator
replacement of the helper and its unit; do not rerun the first-install wrapper.

Linux ARM64, glibc 2.28 minimum. API and schema unchanged.
See [BUG0139](../debug/BUG-0139-pi-offline-backup-budget.md).

## Installation et contrôles réels / Actual installation

Serveur0.7.5 et web0.7.5 installés sur le Pi le 8 novembre, séquence6 chacun.
Le helper/unité ont été remplacés par un opérateur administrateur revu,
avec clé publique épinglée, manifeste signé, sauvegarde durable et rollback.
Le changement d'unité est strictement le délai 10→30 minutes ; configuration
serveur, configuration d'updates, unité serveur et identité TLS conservées.

`chill-update check server` puis `apply server`, sans force ni altération du
registre de confiance : installation `releases/0.7.5-6`, opération
`69c067f4-6cc3-4290-85a6-18c5d6c67a71` réussie. L'ancienne opération différée
0.7.4 a été invalidée normalement lors du changement de manifeste, puis le
nouveau candidat a été admis. Sauvegarde complète
`708d9f19-8e12-4b16-9568-32fa3e289892` : 1 926 fichiers, 92 658 842 octets,
environ 231 secondes de progression des fichiers ; manifeste final présent.

Le bouton Vérifier maintenant de la carte web dans le Mac installé a créé
`a5134007-b6ef-4cf7-931e-bf34ca03e52f`, achevé avec version disponible0.7.5.
Installer maintenant a créé `842dc1d5-0578-4d2f-ba53-775bf1b5b831`, achevé
`installed`. L'interface affiche web0.7.5 et Mise à jour installée. Catalogue
reconnecté. Service et timer actifs ; `available.json`0640 lisible par le service.
Santé HTTPS vérifiée avec CA/résolution configurées, sans désactiver TLS ; les
réponses HTTP index.html, build.json, home.js et updates.js correspondent par
SHA256 au ZIP public0.7.5. API et schéma3 inchangés.

Preuves privées : `server-0.7.5/pi-post-checks.json`, `pi-apply-server-current.log`,
`pi-helper-unit-replacement.log`, `public-verification.json` et
`review-and-checks.json` sous `/private/tmp/chillcenter-playback-wake.BEMWYx/`.
CI CodeQL42529956247 et Shipping dependency security42529956483 PASS ; 28 tests
helper, Clippy strict, 28 contrôles Node helper/docs PASS.

Retour arrière : anciens slots0.7.2 conservés, sauvegarde complète et backups
administrateur privés conservés sur le Pi. Utiliser la procédure locale vérifiée
de récupération avec son reçu exact ; ne pas déplacer manuellement les liens ni
abaisser les séquences. Aucun média ou montage NAS modifié.

Both server and web0.7.5 are installed and healthy on the real Pi. The complete
backup exceeded the former two-minute budget and succeeded with the new bounded
budget. Web discovery and installation succeeded through the installed Mac UI;
served resources match the published payload, with the original TLS trust and
configuration preserved. Rollback slots and private backups remain available.
