# BUG-0082 — Conflit concurrent sans action de récupération

**OPEN — signalé, non corrigé dans ce lot.** 16 octobre 2026, capture utilisateur examinée. Le bandeau dit : « Modification concurrente : rechargez les données avant de réessayer. »

L’utilisateur ne sait ni quelles données recharger ni où agir. Attendu : message contextualisé et action explicite pour actualiser la fiche ou la liste concernée, conserver la saisie si possible, puis permettre une nouvelle tentative. Ne pas proposer un redémarrage, un rescan global ou un écrasement forcé par défaut.

À distinguer : conflit de révision, opération déjà en cours et occupation momentanée du client. Le code d’erreur exact et l’opération à l’origine de la capture ne sont pas connus ; ne pas les assimiler sans trace. Critères de correction : action de récupération adaptée, pas de perte silencieuse de modifications et navigation non bloquée.
