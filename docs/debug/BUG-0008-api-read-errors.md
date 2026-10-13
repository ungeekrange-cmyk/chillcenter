# BUG-0008 — erreur SQLite présentée comme vidéo supprimée

2026-10-13 ; auteur `/root`, reviewer `/root/ui_code_review`. Source `d361eba6d1b1b24432a4cb793b07538c35b657d0` ; défaut MEDIUM restant de la revue code r2, rapports antérieurs conservés.

H1 unique : `Database::get` convertit tout échec `query_row` en sentinelle « Vidéo introuvable », donc `Store::set_rating → media → catalogue.get` répond404MEDIA_NOT_FOUND même pour une base défaillante. Inspection indépendante confirme ce chemin et la mauvaise récupération UI. Un échec SQLite n'a pas été provoqué sur données utilisateur.

Correction : seule `QueryReturnedNoRows` correspond à vidéoabsente ; autres erreursSQLite et JSONcorrompu donnent l'erreur de base sûre, convertie par enum `StoreFailure` en503DATABASE_OFFLINE. Messages internes, cheminsSQL/NAS ou contenu d'erreur fournisseur ne sont pas envoyés au webview.

Régression : catalogue absent, JSONcorrompu puis tablemediaretirée dans une base temporaire ; HTTPnotes après injection de tableabsente doit retourner503et codeDATABASE_OFFLINE. Ce n'est pas une simulation d'un crash disque physique. Le domaine garde les erreurs héritéesString dans son adaptateur typé, sans refactor complet horsscope.

État FIXED ; résultats/SHA corrigé et revue r3 dans INTERFACE-EVIDENCE. F-05/F-14 et contrats d'erreurs ; rollback checkpoint fe85cea sansmigration. La suppression de table est uniquement dans le testlocalisolé, jamais dans une base du foyer.
