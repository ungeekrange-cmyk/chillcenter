# Revues — ressources, source f980355

Base **228c4c6e2a02c9f6ff7520b3d2e0fb77bc5104db**, source exacte **f9803557a452f59cbbe3cbf5b1c4bf1b48a9aa67**, branche `codex/resource-optimizations`,16octobre2026. Transcription par `/root` depuis les instances distinctes ci-dessous. DEV_FAST familial ciblé, demande explicite mesures/builds/déploiement ; pas de nouvelle frontière identité/IAM/TLS/dépendance ou exposition.

## Code — `/root/configuration_code_review`

Source9fbf706 : **CHANGES_REQUESTED**. MEDIUM CODE-F-001 : référence changeante ignorée mais migration annoncée réussie ; exiger échec et couverture intégrale des compteurs. LOW CODE-F-002 : deux emprunts inutiles font échouer Clippy-Dwarnings. Aucun autre défaut fonctionnel relevé. Rapport défavorable conservé, pas réécrit en succès rétroactif.

Source **f980355 : APPROVED_SCOPED**, delta exact relu. Identité changée →Err avant publication ; verified==refs et nombre/octets logiques après parcours exigés ; CLI non-zéro sur erreur. Test déterministe d'inode changé et de chaque mismatch, migration normale/idempotente conservée. Clippy tous targets client PASS,8tests artwork PASS, diffcheck/arbre propre.

Conclusions source inchangées : reset partage le snapshot complet Arc avec checkpoint durable ; delta conserve le checkpoint **complet** courant et pages de ses changements. Sérialisation d'admission/budget/429/TTL avant checkpoint préservés. Publication client Option sans clone intégral, CAS/contexte/mutations et fast-path unchanged sans DB conservés. CAS images privé, chemins anciens et budget logique2Gio inchangés, copie atomique si hardlink impossible ; pas d'écriture en place, pas de SQLite ou sidecars, compromis des objets orphelins explicite. API/UI/web/desktop/Cargo.lock sans diff.

Preuves indépendantes initiales :50client/1ignored,7artwork,4sync/1ignored ; un test socket de la suite serveur refusé par le sandbox, pas transformé en échec applicatif ; suite exacte hors sandbox137PASS/1ignored du coordinateur réutilisée. Nouvelle régression8artwork/Clippy réexécutés sur f980. Les mesures synthétiques sont distinguées de la consommation de l'app Tauri et du Pi.

ADR0024 : **APPROVED**, proposition hash **98c8a30c82fea4fa6534d7bae98534d4c060773745d850ec37053355528b65ef**, auteur/root≠reviewer. Options, fallback, compatibilité, migration, retour arrière et rétention interne crédibles. Nouveau gate utilisateur NOT_APPLICABLE : détail interne réversible couvert par sa demande explicite ; garde opérationnel client fermé et preuves intégrales toujours requis. Statut ACCEPTED ultérieur est administratif, pas une réécriture de la décision.

## Sécurité — `/root/configuration_security_review`

Source9fb puis delta **f980355 : APPROVED_SCOPED**, aucun HIGH/MEDIUM/LOW restant. Gitleaks8.30.1 expurgé base→9fb puis9fb→f980 sans fuite. Huit tests de cache sur f980 PASS ; snapshot/admission4tests, owned stale et annulation1chacun sur9fb réutilisables car delta uniquement cache-migration.

Répertoires réels et privés0700, serveur/profilUUID, feuilles64hex/type/taille, O_NOFOLLOW et lecture bornée/exacte. Stages0600/create_new/fsync, blobs créés par hardlink sans écraser une création concurrente ; existing hash vérifié, références rename atomique, fallback copie. Aucune purge/GC concurrente, aucun blob modifié en place. Maintenance explicitement client fermé, identité contrôlée et succès intégral exigé ; comparaisons de chemins dépendantes de cette précondition documentée. Pool correctement par serveur : la note temporaire contraire a été retirée après vérification du calcul des deux parents depuis images ; aucune correction erronée appliquée.

Pas d'expansion API/web/TLS/SQL/dépendance. Aucun cache réel, Pi, fournisseur ou vidéo opéré par ces reviewers. Leurs verdicts ne remplacent pas la vérification opérationnelle distincte.

## Vérification d'exécution

[Mesures](../performance/RESOURCE-OPTIMIZATION-20261016.md), [livraison](../releases/REL-RESOURCE-OPTIMIZATION-001.md) et [vérification indépendante](../releases/REL-RESOURCE-OPTIMIZATION-001-verification.md) gouvernent les résultats effectivement exécutés. Aucun succès déduit d'un test planifié.

Complément après revue, sans modification du produit : `/root/resource_audit` exécute trois comparaisons par version en **release**, en plus des relevés debug cités dans la revue. Pic médian client90,98→65,80Mio, serveur238,91→211,98Mio ; mêmes fixtures, même checksum normalisé sur les six runs serveur. Les caches de compilation initialement réutilisés à tort sont exclus puis reconstruits par nettoyage ciblé des seuls paquets release ; les exécutables baseline/final sont figés et leurs hashes distincts. Ce rôle de mesure n'est ni auteur du produit ni certificateur de l'interface Tauri ou de la RAMPi.
