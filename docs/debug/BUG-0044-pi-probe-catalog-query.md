# BUG-0044 — Sonde opérateur : query dans le path

2026-10-14, causeconfirméeH1 `/root`. Harnais seulement, serveur/clientproduit non modifiés.

Sondeprepare(sourcef56ecaaf) : HTTPSidentité/profilsPASS, scanNAS2496 en10.14sPASS, puiscatalog404. H1 : appelget_json avec path contenant ?limit=100&available=true ; ServerEndpoint.request_url utilise Url.set_path et queryNone, donc ? est encodé comme partie du path. Productionutilise json_with_options pourquery séparée. Prédiction : curlstrictCArouteavecquery réelleretourne200/catalogue. ExpérienceRO : mêmeprofil/endpoint/certpublicCA, GET routeavecquery retourne100items/has_moretrue/IDsvalides ; pas de disparition de profils ou bugscan.

Correctioncausale sonde : json_with_options GET/pathsansquery/queryfields séparés. Ne pas re-scan/recréerprofils : les3IDsQA déjàcréés et catalogue2496 sont valides. Harnais--play sansquery peut tester les3IDsopaques obtenus ; compte/service/Pi/vidéos ne nécessitent pas rollback. Auteurcorrectif distinct mandaté ; régénérer preuvebuild/clippy du harnais, pas binaireproduit.
