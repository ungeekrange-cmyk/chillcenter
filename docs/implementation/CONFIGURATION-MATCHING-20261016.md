# Configuration, matching et cycle de lecture —16octobre2026

Branche `codex/configuration-matching`, base **66aef9c251cc150bd8d0cfe310ac6056d573766c**. Travail correctif familial déjà cadré, demandes utilisateur explicites ; mode de livraison DEV_FAST ciblé propre à Chill Center, sans réouverture de découverte/cloud. Déploiement serveur seul autorisé explicitement pendant ce lot, sous réserve des tests/revues, sauvegarde privée et conservation du web/NAS/configuration. Candidat exact, artefacts et vérification à compléter après gel.

## Acceptation observable

BUG0081 : association en cours nommée et visible immédiatement, propositions fermées, animation locale, confirmation/erreur et gardes de fiche ; pas de verrou global. BUG0094 : suffixes de release nettoyés, un secours prudent/provenance réelle, années/ambiguïtés conservées ; saisies manuelles intactes. BUG0095 : variantes typographiques d'une source réunies, vrais homonymes distincts, cache dérivé versionné.

US034 et ADR0021 : panneau Configuration, vue Tout+neuf catégories, recherche globale sans secrets, cartes pleine largeur ; Bibliothèque avec plusieurs emplacements. Protocole2026-10-14/v1 additif et ancien web inchangé. Imports comportant plusieurs espaces de noms explicitement refusés en attendant un mapping dédié ; ancien import principal préservé.

BUG0096 : absence finale confirmée au lancement, persistance ciblée atomique/CAS, delta, statut indisponible/actions et retour sans défaut graphique ni saut de scroll ; panne NAS/parent non confondue. BUG0097 : écran noir natif entre fichiers. BUG0098/ADR0022 : moteur/bail/tampon gardés en pause, anticipation bornée, barre conservée et retour visuel immédiat. BUG0099 : masquage automatique au repos souris sans perdre l'accès clavier/menu.

## Diagnostic réel autorisé, avant déploiement

« Avatar The Last Airbender - Aang »2024 retrouvé dans le miroir en lecture seule puis via GET du serveur existant : disponible=true, révision1. Tentative API acquire autorisée par l'utilisateur : **HTTP503 MEDIA_UNAVAILABLE**, aucun bail, aucune vidéo/progression/transfert. Cela reproduit l'incohérence, sans confirmer la cause physique exacte (fichier final vs montage/parent). Aucun scan, appel TMDB ou déplacement de fichier. Les tests de suppression de fichier utilisent seulement une fixture temporaire locale.

## Preuves progressives — ne pas confondre avec le candidat gelé

- Catalogue81 tests PASS ; client41 (ajout du test de corps HTTP historique) ; serveur133+2CLI au checkpoint bibliothèques, puis tests manquants/CAS/pauses ajoutés à revalider globalement. Test socket initial sous sandbox : PermissionDenied, même test puis suite avec permission loopback PASS ; pas un défaut réseau du Pi.
- Node101 ciblés PASS au checkpoint avant derniers compléments pause/contrôles. Tests de feedback média/série/saison : état avant premier await, double clic, erreur et retour tardif ; protections web bridge/player existantes conservées.
- API76 chemins PASS, OpenAPI/guide enrichis sans retrait de requête/champ existant. Nouveau client omet réellement le champ facultatif pour un ancien serveur ; test Rust du corps JSON exact, pas seulement test JavaScript.
- Premier parcours navigateur Configuration : échec sur aucune commande save_library. Cause confirmée BUG0100, helper global corrigé. Passage suivant : échec scroll1200→0 après lancement manquant ; cause body.player-open/hauteur confirmée BUG0096, garde de restauration ajoutée. Les premiers échecs ne sont pas transformés rétroactivement en PASS. Parcours final à joindre.
- Parcours navigateur Configuration/TMDB/fichier absent **PASS** après correction : dix catégories, recherche normalisée globale, pleine largeur, commande multi-emplacements, état de matching avant réponse, autre fiche protégée, tag/action et scroll conservé. Serveur/fournisseur fictifs et réseauHTTP(S) bloqué.
- Test libmpv ciblé réel **PASS** : source synthétique30s, stream mémoire ralenti, pause du même moteur, lectures croissantes sur2s et cache≤60,1s, reprise sans nouveau loadfile. Pas un parcours vidéo général, aucun contenu utilisateur/Pi/NAS/fournisseur ; warning kIOSurface non fatal. La preuve du même bail vient du chemin bridge de pause sans release, pas du moteur qui ne connaît pas les baux.

## Sécurité, compatibilité et limites

Checkpoint intégré : **81 catalogue +41 client +74 domaine +135 serveur** PASS, puis **25 desktop** PASS et **1 test libmpv pause réel** PASS. Premier passage client40/41 : assertion historique attendait le mot « dossier » dans ROOT_REQUIRED ; message désormais Configuration › Bibliothèque, assertion adaptée au nouveau libellé, pas une erreur de transport masquée. Transaction d'indisponibilité confirmée IMMEDIATE et ciblée, correction sans changement des autres transactions de profil. API76 et parcours navigateur final PASS. Les chiffres préliminaires ci-dessus restent leurs checkpoints, pas des suites supplémentaires indépendantes.

Pas de nouvelle dépendance/runtime, fournisseur, coût, authentification, clé ou règle TLS. Bibliothèques supplémentaires validées et accès indexés cadrés sur leur racine ; pas de purge sur scan partiel. Pause conserve l'exclusivité du profil : un autre appareil confirme un transfert, pas un contournement. Échéance/révocation fermées maintenues.

Configuration `library.json` enrichie : un futur rollback vers un ancien serveur à désérialisation fermée doit restaurer l'ancien fichier de réglages sauvegardé, sans écraser les états de visionnage postérieurs ni les fichiers NAS. La mise à jour autorisée ne modifie pas les réglages existants. Web installé/worktree web et anciens bundles/tags conservés. Pas de push ou publication GitHub.

## Remise

En préparation : source figée, revue distincte, scan de secrets du delta, bundle Mac séparé, binaire ARM64 serveur seul, opérateur sauvegarde/bascule revu et vérification post-bascule distincte. Ne pas utiliser le numéro0.5.1/build6 seul pour identifier un bundle ; nom de fichier et hash exacts gouvernent la remise. Les validations humaines Tauri et de débit réel Pi/Wi-Fi/NAS sont distinctes des tests simulés et ne seront pas déduites de leur succès.
