# G-04 — Remise logicielle avec réserves d'essais réels

14 octobre 2026 · **APPROVED_BY_USER_WITH_RESERVES** · source `126da0af167ee62588d92d90297f2dfc51962676` · périmètre : 33 stories / 133 AC.

Preuve utilisateur : « Oui, validé avec ces réserves. Passe à l’installation de Chill Center sur le Pi et aux tests avec le NAS. » La remise logicielle et le bundle sont acceptés avec les réserves ci-dessous. La demande suivante autorise explicitement l'installation du produit sur le Pi identifié et les tests NAS ; elle ne transforme pas les essais non exécutés en PASS, ne permet aucune suppression de vidéos ou modification implicite de secrets/permissions d'autres services.

Livrable : sources du serveur Linux et du client Tauri Mac Silicon avec moteur libmpv/FFmpeg ; catalogue, profils, lecteur, archives, IA et ambiances raccordés ; architecture, décisions, bugs et preuves documentés. 255 tests Rust, 24 Node et 7 de packaging PASS ; trois revues indépendantes favorables au SHA exact ; bundle autonome arm64 de 75 MiB, signature ad hoc et chargement vérifiés. Aucun push, déploiement Pi/foyer ou « 133 AC tous PASS » revendiqué.

Réserves explicites : Pi/Linux/NAS prêts selon l'utilisateur, Chill Center non installé. La signature SSH de la clé a échoué ; accès en lecture seule en attente du socket de contrôle. Build ARMv7/aarch64, installation, systemd, NAS/Wi-Fi, deux à trois flux, CPU/RAM/température et redémarrage/panne réelle restent NOT_RUN. Son et accessibilité contrôlés par un humain, veille OS, second Mac sans Homebrew, fournisseurs réels/TMDB, notarisation, RustSec et migration du foyer non prouvés. Aucun benchmark inventé.

Preuves : [remise](../implementation/COMPLETION-EVIDENCE.md), [inventaire AC](../implementation/AC-MATRIX.json), [revues finales](../reviews/COMPLETION-FINAL-126da0a.md). Les rapports défavorables antérieurs et les checkpoints restent conservés.

Question historique à la remise : accepter la partie logicielle et le bundle avec ces réserves ? Réponse positive reçue et consignée ci-dessus ; les essais matériels restent ouverts. L'installation Pi est autorisée par la demande explicite suivante, pas par une inférence de validation globale des133 AC.
