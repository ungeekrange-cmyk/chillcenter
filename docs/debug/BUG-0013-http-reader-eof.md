# BUG-0013 — Fin du média HTTP convertie en erreur libmpv

2026-10-13 · VERIFIED · auteur /root · source initiale761496fa41d994002cf57d0a176bafebac92fdd3 · finding revue code r1 MEDIUM.

H1 confirmée : le serveur refuse correctement une plage commençant à la taille du fichier par416 ; l'adaptateur HTTP ne reconnaît pas localement la fin connue et remonte une erreur au callback mpv au lieu de0. Le test `known_http_eof_and_empty_buffers_do_not_issue_a_range_request` est RED sur le candidat r1 puis GREEN après le retour0 local pour taille connue ou buffer vide. Annulation vérifiée avant ce retour ; une erreur sur taille inconnue n'est pas transformée arbitrairement en EOF. Aucune modification du transport/TLS ni du serveur416.

Smoke réel complété par seek près de la fin, lecture et `PlayerEvent::Ended{reason:0,error:0}` attendu ; résultat exact dans PLAYBACK-EVIDENCE. Les anciens résultats load/seek seuls ne prouvaient pas EOF. Pas de migration ; rollback par nouvelle branche à la source r1.

Au SHA `b4100a37f985943fa1932bd98a2ec99d70b6c044` : smoke EOF réel PASS,146 tests PASS, revues indépendantes code/test/sécurité r2 APPROVED ; finding CLOSED. Les preuves r1 restent conservées.
