# Serveur 0.7.5 — délai de sauvegarde du Pi / Pi backup budget

Candidat correctif du 8 novembre 2026, nécessaire pour terminer l'installation
serveur/web autorisée. Serveur 0.7.5, séquence 6. Mac 0.7.6/build16, web 0.7.5
et thèmes 0.7.1 inchangés. Aucune nouvelle publication Mac ou web.

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
