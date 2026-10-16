# BUG0103 — succès partiel de la migration d'affiches

16 octobre2026, défaut de revue locale, jamais appliqué au cache utilisateur ni déployé sur Pi. Auteur correction `/root`, reviewer indépendant `/root/configuration_code_review`. Source initiale9fbf706, correction **f9803557a452f59cbbe3cbf5b1c4bf1b48a9aa67**.

Observation : le parcours de migration ignorait une feuille dont inode/taille/date changeait entre lecture et contrôle ; il retournait pourtant Ok et le CLI annonçait OPTIMIZED/rc0 avec verified<references. Hypothèse unique confirmée par inspection du branchement et test déterministe de la condition. Ce n'est pas une corruption constatée du cache utilisateur.

Correction : référence changée → erreur explicite demandant fermeture des clients/retry ; aucune publication de cette feuille. Avant succès, vérifier verified==references, même nombre de références et mêmes octets logiques après parcours. Les liens déjà publiés sur une erreur précédente conservent leurs octets et restent rejouables/idempotents. Le CLI relaie FAILED/non-zéro, sans fausse complétion.

Régression : deux inodes de même taille/contenu ne passent pas le garde d'identité ; compte vérifié partiel, nombre de références ou octets logiques divergents refusés, cas complet accepté. Huit tests du module et relecture hors ligne PASS ; deux emprunts inutiles ayant fait échouer Clippy corrigés dans le même commit. Revues code et sécurité finales **APPROVED_SCOPED** au SHA f980355. Première revue9fb CHANGES_REQUESTED conservée dans le rapport de lot ; bundle9fb intermédiaire conservé mais non remis.

État : **FIXED / VERIFIED_LOCAL**, puis migration réelle complète sur source revue :3926/3926images vérifiées,3930fichiers historiques intacts, bases/états inchangés. [Preuves](../performance/RESOURCE-OPTIMIZATION-20261016.md), [livraison](../releases/REL-RESOURCE-OPTIMIZATION-001.md). Retour ancien bundle compatible avec les mêmes chemins ; aucune restauration de vidéo ou d'historique nécessaire.
