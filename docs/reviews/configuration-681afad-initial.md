# Revues initiales —681afad, conservées défavorables

Source **681afad332704e2b7c56edbbe5b916adf72689d0**, base66aef9c251cc150bd8d0cfe310ac6056d573766c.16octobre2026. Rapports transcrits par `/root` depuis deux instances distinctes, lecture seule ; pas d'approbation rétrospective du candidat.

## Code —/root/configuration_code_review : CHANGES_REQUESTED

- HIGH fonctionnel : retour immédiat sur des candidats TMDB non vides mais non exacts empêchait le secours demandé. Corriger après zéro match exact, pas seulement zéro résultat brut ; deux matchs exacts restent ambigus. Test du vrai loop fournisseur avec premier résultat flou puis secours unique.
- MEDIUM : le simple survol stationnaire de tout l'overlay pinne encore les contrôles. Un clic/Reprendre puis souris immobile doit permettre le masquage ; clavier/vrai menu préservés. Le test initial codifiait à tort le survol permanent.

Reviewer :41Node ciblés PASS, test Rust de requête PASS mais insuffisant pour le premier défaut, diffcheckPASS. Aucun Pi/NAS/fournisseur/vidéo, aucune édition.

## Sécurité —/root/configuration_security_review : CHANGES_REQUESTED

MEDIUM SEC-681-01 : metadata(root/parent) puis traversal séparé ; tout ENOENT de traversal pris pour absence finale. Une racine ou un parent disparaissant dans l'intervalle pouvait déclasser à tort l'entrée. Exiger une distinction dans la traversée par descripteurs, seulement ENOENT du dernier openat, et un test de course déterministe. Autres frontières multi-racines/archives/baux/TLS examinées sans autre défaut relevé.

Preuves propres au SHA :135serveur (une permission loopback sandbox levée pour le test concerné),3Store missing,1fallback,1regroupement,1pause libmpv ; scan Gitleaks8.30.1 expurgé sans fuite. Tests ultérieurs sur arbre modifiable exclus de ce SHA.

## Opérateur privé — APPROVED_SCOPED distinct du produit

V2SHA **2b9b7804cf71a0235a37c2c263bac23d2e7158d4cca3de6f624974676eb1902b**, V3SHA **36588ab01798c019ddbd18ea135a68b2876668a0f32714adb08201e2e8eec6ee**. Revue statique : pin binaire/processus/config/unité/SPKI/web/montage, sauvegarde exacte, refus connexions actives, rollback conditionnel ; V3 valide chemin distant root0700/V2 immuable, refuse état TMDB actif ou invalide et préserve library.json. Bash-nPASS ; pas d'exécution Pi par le reviewer.

INFO non bloquant : library.json changé après réussite deV2 entraîne sortie42 deV3 sans restauration automatique, pour préserver une éventuelle modification utilisateur. Vérifier l'état effectif, ne pas relancer aveuglément.
