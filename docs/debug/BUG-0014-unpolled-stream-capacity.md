# BUG-0014 — Flux non consommés retenant la capacité

2026-10-13 · VERIFIED · auteur /root · source initiale761496fa41d994002cf57d0a176bafebac92fdd3 · PLAY-SEC-001 MEDIUM, revue indépendante r1.

H1 confirmée : huit corps Range non repollés gardent les huit permis ; la vérification de révocation/timeout dans le générateur n'est jamais appelée sous backpressure. Test router `release_reclaims_all_unpolled_stream_slots` RED après release :0 permis disponibles au lieu de8. Correction : garde indépendant, inspection toutes les100ms, restitution des permis après révocation/expiration ou5s sans demande de bloc. Un corps annulé ne peut plus produire de nouveaux octets ; vérifications avant/après I/O. Le test devient GREEN sans consommer/détruire les corps anciens, qui échouent s'ils sont repris ensuite.

Régressions : huit slots/nouvièmeMEDIA_BUSY, déconnexion ; release, modification racine et suppression profil avec corps retenus ; idle5s récupère capacité pour un autre profil même avec un bail encore actif. Le timeout ne peut pas interrompre un syscall noyau : quatre vrais jobs bloqués conservent leurs slots après timeout5s, puis les rendent après retour, testé séparément. Les corps réseau anciens peuvent encore détenir des descripteurs jusqu'à clôture HTTP ; les plafonds de connexions/streams existants restent en place. Pas de contournement auth/TLS ni acceptation de risque implicite. Rollback par nouvelle branche source r1, défaut alors réintroduit.

Au SHA `b4100a37f985943fa1932bd98a2ec99d70b6c044` : tests de capacité/révocation/idle PASS,146 tests PASS, revues indépendantes r2 APPROVED ; PLAY-SEC-001 RESOLVED. Limites résiduelles explicites acceptées pour le slice de développement par la revue sécurité, pas d'approbation produit/Pi.
