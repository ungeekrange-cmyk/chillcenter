# BUG-0086 — Le lecteur reste ouvert à la fin du dernier élément

16 octobre 2026. **CORRECTED_LOCAL**, restitution native Tauri à confirmer.

Cause confirmée : `renderNext()` masquait la proposition lorsque `next.media` était nul, sans fermer le lecteur ni revenir à l'origine. De plus, « Lire » dans la fiche d'un membre ne transmettait pas le contexte saga/liste, contrairement à sa carte.

Correction : un EOF sans erreur avec réponse `next` résolue et `media:null` ferme le lecteur puis revient à la série, saga, liste ou fiche du film. Une vidéo suivante présente conserve le contrôleur existant (vignette, nouvelle saison, disponibilité, compte à rebours et lancement immédiat). Annuler ferme et revient également. Le contexte de la fiche est conservé si le média appartient au panneau courant.

Une réponse `next` absente n'est pas une fin de collection : elle peut indiquer une synchronisation ou résolution échouée. Aucun lancement/fermeture automatique sur erreur de décodage ; vidéo suivante indisponible laissée visible sans lancement. Stop et retours différés sont gardés contre changement de serveur/profil, nouvelle navigation ou lecture.

Preuves : tests synthétiques `player.dom.test.js` sur les quatre contextes terminaux, délais 0/5/10/15/20/30/off, annulation, indisponibilité, erreur, stop échoué et retours périmés ; `detail-lifecycle.test.js` sur les contextes et catalogue sans héritage. Parcours EOF DOM connecté dans `test-series-layout.cjs`. Aucun décodeur ou serveur réel sollicité, API inchangée.
