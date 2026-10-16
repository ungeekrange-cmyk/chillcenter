# BUG-0091 — Contraste insuffisant du bouton « Ne pas lancer »

16 octobre 2026. **OPEN — référencement uniquement, traitement explicitement différé par l'utilisateur.**

## Signalement et preuve

Client Mac, proposition de l'épisode suivant d'une série. Version du processus non relevée ; dernière remise dans ce chat : candidat df2870f, interne0.5.1/build6. Capture utilisateur `codex-clipboard-f4225cca-5306-46a0-bcc1-20d5ce250691.png` : carte sombre « À suivre », épisode004 d'« Avez-vous déjà vu », décompte5s ; libellé « Ne pas lancer » très sombre sur fond sombre. La capture documente le défaut visuel, pas sa cause CSS ni le comportement du bouton.

## Attendu

Le libellé doit être clairement lisible et cohérent avec les autres contrôles, dans les variantes claire/sombre et les ambiances. Garder une distinction visuelle entre lancer et annuler, sans rendre l'action secondaire illisible.

## Suivi

Cause non investiguée, aucune hypothèse testée, aucune correction ou modification de palette appliquée. Validation future : contraste du texte et des états normal/survol/focus, inspection réelle dans Tauri. Aucun test exécuté pour ce signalement ; aucun bundle ou déploiement. Liés : BUG0092/0093, distinct du défaut d'ordre API BUG0088.
