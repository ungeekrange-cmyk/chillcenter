# Archives administratives — contrat candidat

2026-10-14 · US-027/028 partiellement implémentées, pas de livraison complète revendiquée.

- POST `/api/v1/admin/exports` : prépare l'archive et renvoie `{export_id,bytes,entries,state:"ready"}` ; GET `/exports/{id}` : téléchargement ZIP. Un export inachevé n'est jamais téléchargeable.
- POST `/api/v1/admin/imports` : corps ZIP binaire streamé vers staging privé ; aperçu `{preview_id,export_id,counts,profiles,roots:["library"],apply_supported:true}`. Aucune donnée de destination modifiée.
- POST `/imports/{preview_id}/plan` : `{profile_map:{source_uuid:{kind:"existing",id:destination_uuid,expected_revision:1}|{kind:"new",name:"Nom"}},root_prefix:"",conflict_policy:"keep_destination"|"replace_conflicting"}`. Racine bibliothèque configurée obligatoire. Réponse `{plan_id,epoch,root_revision,profiles,conflicts,conflict_count,already_imported,apply_supported:true}` ; aperçu des conflits limité100 lignes, total distinct fourni. Plan complet privé, jamais accepté du client.
- POST `/imports/{preview_id}/apply` : `{plan_id,confirmed:true}`. Retour `{already_imported,media,profiles,epoch}`. Plan périmé HTTP409 `IMPORT_PLAN_STALE` ; pas d'import partiel. Correspondance complète/injective, répétition idempotente, aucun chemin libre/vidéo/source touché.
- DELETE `/imports/{preview_id}` : annulation204, nettoyage ZIP/plan exacts. Opération simultanée409 `IMPORT_BUSY`. Durée15min, plafond6 aperçus. Redémarrage invalide les poignées.

Une politique de remplacement est une décision affichée avant confirmation, pas une fusion personnelle automatique. Les vidéos importées nouvelles restent indisponibles jusqu'à scan de la racine correspondante. Les champs présents de la destination sont conservés avec keep_destination. Les affiches sont recopiées dans un cache immuable validé et leurs références validées avant transaction ; erreurs SQL/epoch nettoient uniquement les nouvelles copies non référencées. Le résultat du commit décide de leur conservation même si le client annule sa requête.

Preuves locales :3 tests domaine (injection erreur SQL, rollback/epoch, profils distincts/idempotence) et route export→ZIP→aperçu→plan→application→répétition→annulation avec vraie image PNG. Ni migration Cinéfolio réelle, ni restauration foyer, ni ressources Pi mesurées. Limites détaillées et dépendances dans ADR-0009. Raccordement interface restant.
