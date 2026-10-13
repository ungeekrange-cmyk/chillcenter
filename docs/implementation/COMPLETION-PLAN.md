# Plan de complétion — demande « Vas jusqu'au bout des US »

2026-10-13 · étape4 IN_PROGRESS · base2a085486f4579d4cf27c98b254bd29fea9d73547 · lane DEV_STANDARD (périmètre, contrats, architecture et stories déjà validés). Pas de discovery répétée, de nouvelles fonctions ou de gate intermédiaire artificiel. L'utilisateur demande maintenant la totalité des33 stories, pas seulement un checkpoint lecteur.

1. Lecteur Tauri visible : surface native, flux épinglé, controls/langues/vitesse/plein écran, reprise et crédit, réseau/perte/suspension. Tests vrais GUI avec fixtures MKV/MP4/AVI et sous-titres. US016–022.
2. Catalogue complet, séries/sagas, notes de groupe et playlists ordonnées, métadonnées/affiches/TMDB et indisponibilité. Conserver toutes portées et révisions. US007–015/020 ; prototypes legacy seuls ne comptent pas comme livrés.
3. Transfert confirmé et échéance/ACK, puis résolution de suite/enchaînement avec annulation. US023–025.
4. Identité/réassociation explicitement confirmée, archives JSON+affiches staging/ZIP limitées, aperçu/import idempotent/restauration avec sauvegarde préalable. Fixtures uniquement, jamais mutation Cinéfolio/NAS. US026–029/MIG001–002.
5. IA client avec preview/révisions/profil, petits boutons ; ambiances calendrier/locales et assets validés. US030/032–033.
6. Matrice AC complète, revues indépendantes exactSHA, artefacts/bundle arm64/dépendances/licences, service Linux/systemd/rollback. US001–033 vérifiées selon preuves réellement exécutées. Tests Pi/NAS/Wi-Fi et migration foyer/IA réelle nécessitent configuration réelle ; NOT_RUN jusqu'à disponibilité, pas une excuse pour arrêter les fonctions testables localement.

Un auteur par chemin/clone, au plus deux writers simultanés ; root possède contrats et merge queue. Les reviewers restent distincts des auteurs. Candidats figés puis revues code/test/sécurité, rapports anciens conservés. Pas de push/deploiement du foyer/production implicite. La preuve finale G-04 sera demandée seulement avec remise complète et limites explicites, jamais avec toutes stories déclarées DONE sans leurs AC.
