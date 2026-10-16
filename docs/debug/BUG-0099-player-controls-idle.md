# BUG0099 — Contrôles restant visibles après interaction souris

16octobre2026, FIXED_LOCAL, branche `codex/configuration-matching`, base66aef9c. Demande utilisateur : en lecture, masquer automatiquement les contrôles après quelques secondes sans mouvement. Une logique2,5s existe ; le retour humain indique qu'elle ne suffit pas dans son parcours.

Ne pas conclure que le timeout seul manque. Vérifier les garde-fous de focus/survol et la différence souris/clavier/menu ouvert ; préserver l'accessibilité clavier et ne pas cacher les commandes pendant pause/chargement ou un vrai menu actif. Tests ciblés attente/reprise au mouvement et clic souris sans focus artificiellement persistant. [Résultats](../implementation/CONFIGURATION-MATCHING-20261016.md).

H1 confirmée : le focus posé par un clic souris était traité comme une interaction clavier permanente. Le flag distingue désormais l'intention clavier du pointerdown ; focus clavier/menu réellement actif conservés, clic souris suivi d'inactivité autorise le masquage. Délais et fondu existants conservés, tests DOM ciblés PASS ; observation humaine Tauri restante.

Revue681afad : seconde cause confirmée, simple playerHover pinne tout l'overlay indépendamment du focus. Retrait de cette garde générale ; seul clavier ou SELECT actif peut prolonger. Régression pointerenter+focus souris BUTTON puis2,5s immobiles→masqué, pointermove→visible ;34tests DOM PASS. Le candidat initial ne remplissait pas encore cette demande.
