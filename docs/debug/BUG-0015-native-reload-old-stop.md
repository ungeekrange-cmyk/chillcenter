# BUG-0015 — Arrêt de l'ancien fichier pris pour une erreur du rechargement

2026-10-14 · ROOT_CAUSE_CONFIRMED · /root diagnostic · source `64ce09e3b3e17efd821ba44ee72e4f454edd9e06`, branche codex/player-ui, Darwin arm64/Tauri2.12.3/libmpv0.41.0. Fixture HTTPS privée18546, pas de NAS.

Attendu : pause libère le bail, seek0,5s puis reprise réacquiert et recharge la vidéo. Réel : UI_QA_RESULT false near-zero seek, « lecture interrompue (2/0) ». Aucun correctif de production avant diagnostic.

H1 unique CONFIRMED : événement Ended STOP de l'ancien fichier encore dans la queue pendant le nouveau load, interprété comme erreur du nouveau. Sonde debug uniquement codes/flags (pas noms/paths/URL/session), trace : Started loaded=false pending=true ; FileLoaded ; Ended reason2/error0 loaded=false pending=true ; Started ; FileLoaded. Header installé mpv/client.h confirme EOF=0 et STOP=2. Le même parcours GUI échoue à chaque reprise, sans échec réseau/TLS.

Remédiation autorisée : état du cycle de chargement permettant d'ignorer les fins antérieures avant le nouveau Started, sans masquer une erreur réelle du nouveau média. Writer distinct /root/native_tauri_surface ; pas de correction/approbation par l'instance de diagnostic. Retirer la sonde temporaire, ajouter test d'événements et relancer le vrai GUI jusqu'à pause/seek/reprise/EOF. Résultat et SHA de correction à ajouter, l'échec n'est pas effacé.
