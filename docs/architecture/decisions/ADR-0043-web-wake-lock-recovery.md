# ADR0043 — Récupération bornée du maintien éveillé web

20 novembre 2026. Correction demandée par l'utilisateur. Complète ADR0036 et
remplace son choix de ne jamais redemander un verrou libéré sans changement de
lecture. Aucun changement de la gestion native macOS.

Le cycle appartient au lecteur web, uniquement si la vidéo est effectivement en
lecture, bail valide et page visible. Le status existant réévalue l'acquisition
même lorsque cet état reste actif. La possession et les générations de demandes
restent uniques. Une autorisation retournée déjà libérée est un échec, et non
une protection active.

Le système peut retirer l'autorisation pendant la lecture. La récupération
attend1s puis5s, trois demandes maximum par rafale. Une autorisation tenue30s
permet un nouveau budget lors d'une révocation ultérieure, pour les vidéos longues.
Un refus persistant ou une succession de révocations immédiates suspend les
essais jusqu'à une nouvelle activation (pause/reprise ou retour de visibilité).
Aucun timer supplémentaire ni téléchargement, service ou dépendance nouvelle.

Une limitation persistante est affichée pendant la lecture, sans l'empêcher.
L'accès HTTPS reconnu et le support du navigateur restent nécessaires ; aucune
page ne peut contourner le refus de l'OS, une veille volontaire ou un capot fermé.
Aucune permission serveur n'est élargie. Les états personnels restent intacts.

Références : [W3C Screen Wake Lock](https://www.w3.org/TR/screen-wake-lock/),
[MDN](https://developer.mozilla.org/en-US/docs/Web/API/Screen_Wake_Lock_API).
Preuves : [BUG0142](../../debug/BUG-0142-web-wake-lock-recovery.md).
Rollback : ancien client web signé, sans migration de données ni modification TLS.
