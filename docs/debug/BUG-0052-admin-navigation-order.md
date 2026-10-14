# BUG-0052 — ouverture Administration répartie entre callbacks concurrents

2026-10-14, candidat local `codex/ui-panels-player` depuis `0d0da0e`, avant remise. Régression découverte pendant le lot, pas déclarée présente dans le bundle9277 du foyer. Aucun Pi touché.

Attendu : arrêter proprement le lecteur puis ouvrir Administration et charger ses informations. Observé : le premier parcours Tauri atteint les nouvelles pages/sélecteurs puis échoue `TMDB config form: timeout`. Le panneau peut rester fermé.

H1 : l'attente asynchrone de l'arrêt laisse le listener secondaire commencer `unavailable` avant `refresh`, qui partage un slot UI exclusif. Le test `admin-navigation.dom.test.js` rejoue cet ordre avec un tour d'événement d'IO : RED `Administration never opened`. Cause confirmée, sans requête fournisseur réelle.

H2, seconde manifestation du même partage de responsabilité : avec une réponse immédiate, le callback principal peut se terminer avant le listener qui sélectionne la section. Snapshot du navigateur synthétique après échec : connected, même contexte/baseline, section finaleadmin, busyfalse, derniers appelsrefresh/resolve_appearance, aucunscan_status ; le contrôle `current()` a donc rejeté la suite avant que l'autre listener n'ait choisiadmin. Le timing d'une IPC réelle plus lente masquait ce second cas dans le parcours Tauri GREEN intermédiaire.

Correction : un seul gestionnaire choisit explicitement `shellShow("admin")` après l'arrêt et avant `refresh`, puis séquence scan, indisponibles et TMDB. Suppression des listeners de chargement concurrents ; garde de contexte entre étapes. Le fournisseur reste derrière son API de lecture/retry existante. Pas de retry de mutation, de temporisation arbitraire, de suppression d'assertion ou d'augmentation de timeout.

Régression locale RED→GREEN, navigateur isolé complet GREEN après correction. Le parcours Tauri final et le SHA remis sont consignés dans REL-MAC-UI-PANELS-001. Échecs intermédiaires conservés, aucune validation TMDB réelle revendiquée.
