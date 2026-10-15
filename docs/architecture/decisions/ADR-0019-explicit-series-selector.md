# ADR-0019 — Sélecteur de série explicite, compatibilité des titres historiques

Date2026-10-15 ; propriétaire/auteur `/root` ; IMPLEMENTED_DETAIL, correctif de compatibilité demandé explicitement par l'utilisateur (« bonne version de l'API […] sans rendre obsolète celle de la version web »). Remplace uniquement l'interprétation implicite des titres de l'ADR-0015-series-season-identity, qui reste accepté pour l'isolation des identités.

L'API historique2026-10-14/v1 accepte un titre quelconque non vide, sans réserver les préfixes `media:`/`tmdb:tv:`. Les interpréter comme des clés par défaut est une rupture et peut modifier le mauvais groupe. La conservation des chemins et schémas JSON seule ne prouve pas la compatibilité.

Décision : GET/PUT `series/{series}` conserve le mode titre historique par défaut (`selector=title`). Le paramètre facultatif `selector=key` active explicitement le sélecteur opaque. Un préfixe seul n'a aucun effet sur l'API. Aucun ancien paramètre/requête n'est rendu invalide ; les autres query parameters historiques restent ignorés comme auparavant. La version reste2026-10-14, aliasv1 inchangé : c'est une extension opt-in, pas un changement de sens par défaut.

Le mode titre conserve vue et édition de métadonnées communes historiques par lot, sans changer les identifiants TMDB, fichiers ou saisons. Le mode clé isole une série identifiée ou un groupe source. Les usages internes du domaine restent stricts et les nouveaux clients utilisent explicitement les clés du catalogue. Le bundle Mac doit être ajusté ; l'ancien11aa529 n'est pas annoncé pleinement compatible pour ces éditions. Le web déployé et le web en développement ne sont pas modifiés par cette livraison serveur.

Preuves requises : collisions entre titre littéral et vraie clé pour les deux préfixes, GET/PUT et les deux alias ; CAS/conflict ; scope profil ; groupe non sélectionné inchangé ; test du client natif. Les revues source be57/02b défavorables sont conservées. Les limites perf/lecteur et les règles LAN/TLS restent inchangées.
