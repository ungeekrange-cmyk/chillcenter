# Revue code indépendante — macOS 15 et reprises mesurées

- **Type / reviewer** : code / `/root/mac15_progress_code`
- **Base exacte** : `e8d6f53c6b9d2dedf4a89afc4851af2d001830f4`
- **Candidat exact** : `82836674ad1c7d0f12b8744d8e26e71cff14ce30`
- **Branche / mode** : `codex/macos15-build` / candidat local, aucune fusion, publication, installation ou certification sécurité
- **Verdict** : `CHANGES_REQUESTED`

## Constat bloquant

### MEDIUM / P2 — le repli sur un ancien serveur conserve et affiche des reprises obsolètes

`ui/connected/progress.js:29,70` recharge le cache `localStorage` du couple serveur/profil, puis, lorsque le nouveau endpoint répond `404` ou `405`, positionne seulement `unsupported=true`. Les `entries`, la `revision` et la copie persistée restent intactes. Après retour du même serveur à la version publiée 0.7.2, les barres et la rubrique **En cours** continuent donc d'afficher indéfiniment l'ancien état mesuré alors que la fonction est censée être désactivée proprement sur un serveur sans endpoint.

Correction attendue : sur `404/405`, vider `entries` et `revision`, supprimer `cacheKey(key)`, redessiner/vider la vue En cours sans affecter le reste de l'application. Ajouter une régression qui précharge un cache valide, fait répondre `404`, puis vérifie l'absence de barre et de résultat En cours. Les pannes réseau transitoires doivent continuer à conserver le dernier cache valide.

## Delta accéléré et preuves réutilisées

- Le delta `2edaab5b8a40f8808bfeffe10276793ea1bdf611..82836674ad1c7d0f12b8744d8e26e71cff14ce30` est limité à `apps/desktop/src/main.rs` : `None` devient `None::<&Value>` pour lever l'ambiguïté générique du GET. Aucun comportement, contrat, cache, schéma ou chemin de packaging ne change ; l'analyse du parent reste valable sauf le constat ci-dessus.
- Revue statique : persistance transactionnelle après Progress validé, endpoint groupé distinct, séparation du delta catalogue, réseau natif hors mutex, garde de contexte tardif, épisodes non groupés avec contexte d'autoplay série, resolver média isolé et rejet du minimum Mach-O supérieur à 15.0.
- Vérifications reviewer : `git diff --check e8d6f53..8283667` PASS ; 36 tests Node ciblés PASS (`progress`, chargement catalogue DOM, bridge web) ; 15 tests Python ciblés PASS (`macos_media_inputs`, minimum macOS). La première invocation Python depuis la racine a échoué uniquement sur le chemin d'import du harnais ; la même suite exécutée depuis `scripts/helpers` a PASS.
- Preuves valides fournies par le coordinateur et réutilisées : 329 Node PASS avant les derniers ajouts, 15 Node ciblés PASS, Python 20+3 PASS, domaine 75 PASS avant assertion de réouverture, serveur 148 PASS/1 ignoré avec endpoint/bail réel ; smoke Chromium généré `881f9a79be2f` PASS pour seuil 5 %, détail 42 %, épisode, conservation de recherche, mobile Plus→En cours et absence d'appels par carte. Ces preuves ne corrigent pas le scénario cache préchargé + 404 absent des tests.

## Limites et observations d'architecture

- Tests physiques macOS 15/26 : `NOT_RUN`. La sonde Mach-O et le chargement libmpv sur macOS 27 ne prouvent pas ces plateformes.
- Build/release complet du porteur : hors de cette revue au moment du verdict ; aucun Cargo lancé par le reviewer.
- ADR0034 décrit l'extraction de « dylibs/notices only », alors que `macos_media_inputs.py` extrait aussi les recettes `.brew/*.rb` pour provenance sans les exécuter. Non bloquant pour ce verdict, mais le texte ADR ou la sélection devrait être aligné avant acceptation formelle de l'ADR.
- Cette revue ne constitue pas une certification sécurité, une autorisation de publication, de Pi/NAS ou de déploiement.

Une correction source rend ce rapport historique et exige une nouvelle revue exacte du SHA, éventuellement par delta accéléré.
