# BUG0074 — bandeau supérieur masquant le décor

16 octobre2026, capture utilisateur sur v0.5.0 (source1e16ee2). Halloween clair : l'image s'arrête visuellement sous la recherche/profil ; même mécanisme à Noël et en sombre.

H1 confirmée : `.workspace::before` a déjà `position:fixed; top:0`, mais `.app-topbar` reçoit un fond opaque `var(--bg)` de shell.css. L'image est masquée, pas trop courte. Reproduction ciblée RED : assertion de transparence pour Noël sombre, réel`rgb(21,27,24)` au lieu de transparent. Pas de deuxième hypothèse.

Correction : une règle dans events.css rend le bandeau transparent uniquement pour les deux événements. Le champ de recherche garde sa propre surface lisible ; image/voile/ancrage/navigation/lecteur inchangés. Les thèmes sans événement gardent le fond initial. Test étendu : vraie interface connectée/pont synthétique, recherche visible, transparence des quatre couples Noël/Halloween × clair/sombre, décor top0, retour opaque hors événements, captures. Aucune nouvelle vidéo ni action serveur/NAS.

Version corrective0.5.1/build6 distincte : conserver tag et bundle v0.5. Source/commandes/résultats définitifs dans REL-V0.5.1 ; état FIXED, validation locale en cours à ce checkpoint. Repli par relance du bundle0.5, sans migration.
