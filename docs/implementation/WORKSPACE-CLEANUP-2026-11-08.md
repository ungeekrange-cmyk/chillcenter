# Nettoyage local Chill Center — 8 novembre 2026

Demande explicite : nettoyer `chillcenter-fixes-web-parity` et les espaces des
projets Chill Center. Audit lecture seule puis contrôle frais des processus,
fichiers ouverts, propriétaires et chemins réels avant suppression. Aucun scan
ou nettoyage global de `/private/tmp`, des caches partagés ou des sessions Codex.

## Résultat mesuré

- Espace disponible avant : 64 215 826 432 octets.
- Après nettoyage et synchronisation : 85 543 698 432 octets.
- Augmentation réelle observée : **21 327 872 000 octets**, soit 21,3 Go / 19,86 Gio.
- Blocs estimés des candidats au moment de l'action : 22 146 019 328 octets.
  Cette estimation diffère du gain réel, notamment avec les extents APFS.

Supprimés : les sorties Cargo de `/private/tmp/chillcenter-fixes-web-parity/target`,
le cache Cargo de `.claude/worktrees/web-pwa/target`, l'outillage cross temporaire
`/private/tmp/chillcenter-linux-cross.1M5r5u`, douze bundles de test et les trois
copies de téléchargement `public-verify` des releases 0.7.4, 0.7.5-final et 0.7.6.
La coquille vide `chillcenter-fixes-web-parity` a ensuite été retirée.

Cargo a refusé le premier cache car son ancien `CACHEDIR.TAG` était absent.
Aucun marqueur n'a été fabriqué : ses seuls enfants générés ont été vérifiés
par liste fermée, métadonnées Rust, profils Cargo et identité du binaire ARM64
copié/scellé avant leur suppression. Le cache web-pwa a été nettoyé avec
`cargo clean --target-dir <chemin exact>`. Les autres copies ont été retirées
uniquement selon l'inventaire exact, sans suivre de lien symbolique.

## Conservé

Les quatre worktrees, leurs HEAD et états Git sont inchangés par le nettoyage.
Les branches et commits locaux, sources, paquets canoniques signés, sources
correspondantes, sauvegarde privée de l'installation Mac, données SQLite/WAL/SHM,
images et petites preuves JSON/logs restent conservés. Aucun nettoyage des
médias ou sauvegardes du Pi/NAS. Les onze autres coquilles temporaires inventoriées
ne comportaient aucun fichier régulier ni bloc récupérable ; leurs liens n'ont
pas été suivis ni supprimés.

Preuves privées conservées :
`/private/tmp/chillcenter-playback-wake.BEMWYx/cleanup-audit/summary.json`,
`exact-paths.json`, `cleanup-before.json`, `cleanup-result.json`, `cleanup.log`
et opérateur `clean-confirmed.py`. Les futurs builds devront recréer leurs caches ;
les recettes et dépendances figées restent dans les sources et livrables.
