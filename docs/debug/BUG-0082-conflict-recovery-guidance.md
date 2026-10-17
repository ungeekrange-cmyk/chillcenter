# BUG-0082 — Conflit concurrent sans action de récupération

## 17 octobre 2026 — FIXED / VERIFIED_LOCAL

Cause : le pont remontait bien le rejet CAS, mais l'interface n'offrait aucun
chemin de relecture contextualisé. `recovery.js` distingue les conflits
409 PROFILE_CONFLICT/CONFIG_CONFLICT des opérations occupées et propose une
actualisation explicite de la fiche, série, collection ou configuration concernée.
Les brouillons non secrets sont conservés, y compris les modifications saisies
pendant la relecture. Aucun enregistrement n'est rejoué automatiquement :
l'utilisateur vérifie puis enregistre de nouveau avec la révision actualisée.
Un changement de contexte retire la proposition périmée. La navigation reste libre.

Preuves : six tests `ui/connected/recovery.test.js` ; vrai DOM connecté synthétique
`scripts/test-ui-recovery-icons.cjs`, conflit média/bibliothèque, saisie conservée,
nouvelle révision, fermeture pendant lecture différée et sauvegarde explicite PASS.
Ce n'est pas un essai humain Tauri ni une modification du serveur/Pi.
Le signalement initial ci-dessous est conservé comme historique.

**OPEN — signalé, non corrigé dans ce lot.** 16 octobre 2026, capture utilisateur examinée. Le bandeau dit : « Modification concurrente : rechargez les données avant de réessayer. »

L’utilisateur ne sait ni quelles données recharger ni où agir. Attendu : message contextualisé et action explicite pour actualiser la fiche ou la liste concernée, conserver la saisie si possible, puis permettre une nouvelle tentative. Ne pas proposer un redémarrage, un rescan global ou un écrasement forcé par défaut.

À distinguer : conflit de révision, opération déjà en cours et occupation momentanée du client. Le code d’erreur exact et l’opération à l’origine de la capture ne sont pas connus ; ne pas les assimiler sans trace. Critères de correction : action de récupération adaptée, pas de perte silencieuse de modifications et navigation non bloquée.
