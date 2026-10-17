# BUG-0084 — Rotation de la synchronisation décentrée

## 17 octobre 2026 — FIXED / VERIFIED_LOCAL

Cause : rotation du glyphe texte et de ses métriques plutôt que d'une géométrie
centrée. Un conteneur fixe 24×24 porte un SVG 22×22, animé autour du centre de
son viewBox ; la réduction des animations est respectée.
`scripts/test-ui-recovery-icons.cjs` vérifie boîte et centre stables à plusieurs
angles/tons/largeurs, synchronisation tenue sans blocage de la recherche et
mode reduced-motion : PASS. Pas de nouveau test matériel ou de synchronisation Pi.
Le signalement initial est conservé ci-dessous.

**OPEN — non bloquant, signalé et illustré, non corrigé dans ce lot.** 16 octobre 2026, indicateur de synchronisation en haut de l’interface Mac ; version exacte à relever.

L’utilisateur décrit une icône de recharge qui monte et descend au lieu de tourner autour d’un centre fixe. La capture localise le contrôle ; une image fixe seule ne valide pas la trajectoire de l’animation.

Attendu : boîte et centre stables pendant la rotation, sans déplacement de la barre supérieure ni du contenu voisin. Cause inconnue : boîte du glyphe, métriques de ligne et origine de transformation à vérifier, sans les présenter comme diagnostic acquis.

Test futur : comparer le centre visuel/boîte sur plusieurs angles, fenêtre et tons différents ; respecter la réduction des animations et conserver la synchronisation en arrière-plan non bloquante. Aucun changement produit ni lancement de synchronisation réelle dans ce lot.
