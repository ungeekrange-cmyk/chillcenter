# BUG0101 — Course entre vérification parent et détection du fichier absent

16octobre2026, FIXED_LOCAL, détecté par revue de681afad avant installation. SEC-681-01 MEDIUM ; pas un incident NAS réel démontré.

Cause statique confirmée : contrôle metadata de racine/parent par chemin, puis réouverture séparée et toute erreur NotFound classée MissingLeaf. Une disparition intermédiaire pouvait être confondue avec celle du seul fichier. La garde CAS de publication ne corrige pas cette mauvaise classification physique.

Correction : la traversée sécurisée existante conserve les descripteurs racine/parents et expose une observation MissingLeaf seulement pour ENOENT de l'ultime openat. Les anciens appelants open_indexed conservent leur signature/erreur, pas de duplication moins sûre des ouvertures. Le chemin playback ne fait plus de contrôles metadata racy.

Test déterministe : retrait d'un parent avant traversal = erreur temporaire ; retrait du seul fichier après ouverture du parent = MissingLeaf. Clippy et tests ciblés PASS ; révision/source finale et revue de correction dans [le lot](../implementation/CONFIGURATION-MATCHING-20261016.md). Aucun redéploiement de681afad, aucun fichier utilisateur touché.
