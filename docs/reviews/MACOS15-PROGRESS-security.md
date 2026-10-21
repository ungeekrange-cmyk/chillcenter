# Revue sécurité indépendante — macOS15 et reprises mesurées

- **Mode :** `DEV_FAST`, candidat local non publié dans les frontières approuvées.
- **Base :** `e8d6f53c6b9d2dedf4a89afc4851af2d001830f4`.
- **Candidat exact :** `82836674ad1c7d0f12b8744d8e26e71cff14ce30`, branche `codex/macos15-build`.
- **Auteur :** `/root` ; **reviewer indépendant :** `/root/mac15_progress_security`.
- **Revu à :** `2026-10-21T19:02:50Z`.
- **Verdict :** **APPROVED**.

Cette approbation porte uniquement sur le diff exact base/candidat. Elle ne vaut
ni approbation des ADR0034/0035 rédigés par l'auteur, ni publication, déploiement,
installation, contact Pi/NAS/GitHub, compatibilité physique macOS15/26 ou
validation d'un bundle final. Le reviewer n'a modifié aucun octet candidat ou
test ; son seul écrit est ce rapport.

## Frontières et contrôles

| Domaine | Conclusion et preuve dans le candidat |
| :--- | :--- |
| Autorité, profils et réseau | Aucune frontière identité/secret/session/cookie/IAM n'est ajoutée. Le produit reste le service familial LAN sans authentification accepté par F-18 ; les profils séparent l'état applicatif mais ne sont pas des tenants confidentiels. La nouvelle route est une lecture `GET` sous `/profiles/{profile}/...`, vérifie l'existence du profil et filtre la jointure SQLite par ce profil. Aucun CORS, redirect, callback, cookie ou CSRF nouveau. |
| Écriture de durée | `duration_ms` n'est écrit qu'après la validation existante du bail, média, génération, séquence et bornes. L'upsert de l'état, la dernière séquence et `cc_resume_measurement` partagent la même transaction. La FK composite suit exactement l'état personnel avec cascade ; une suppression de profil ne laisse pas de mesure orpheline. Une durée n'entre pas dans le snapshot catalogue. |
| Lecture bornée et jetons | `known_revision` est limité à 255 côté serveur, pont natif et pont web (les tokens produits sont ASCII). Le résultat est borné à 30 000 reprises, trié et profile-scoped. Le token inclut un UUID de boot ; un token ancien ou d'un autre profil ne peut pas produire `unchanged=true`. Aucun chemin NAS, nom de profil, jeton de bail ou secret n'est sérialisé. |
| Clients et réponses tardives | Le natif capture seulement transport/profil sous mutex puis effectue HTTPS hors verrou et rejette le résultat après changement d'epoch. Le web utilise la route datée allowlistée. Le cache local est indexé par `server_id:profile_id`, validé avant rendu, supprimé avec le profil, et une réponse tardive ne peut ni peindre ni persister dans le nouveau contexte. `404/405` ne désactive que cette vue. |
| Entrées macOS et provenance | Le verrou contient 43 identités uniques ARM64 Sequoia, URLs HTTPS content-addressed exclusivement sous `ghcr.io/v2/homebrew/core`, SHA-256 exacts et métadonnées licence/homepage/source avec checksum ou révision. Le paquetage utilise exclusivement la racine isolée vérifiée ; aucune résolution Homebrew globale n'est son repli. Le reçu inventorie les octets Cellar et les alias de formules sont vérifiés. Aucun lockfile Cargo/npm, workflow, runtime Rust/Node ou clé de signature n'est modifié. |
| Extraction et bundle | Les chemins absolus, `..`, antislash, mauvaises versions, membres sélectionnés dupliqués, hardlinks/FIFO et liens sortants sont refusés ; nombre, taille d'archive et taille décompressée sont bornés. La fermeture transitive est résolue dans la seule racine isolée, puis les liens package-manager sont réécrits. L'exécutable et toutes les dylibs sont inspectés avant signature et le build refuse tout minimum supérieur à15.0. La vérification et le chargement libmpv portent ensuite sur les octets signés. |
| Secrets et logs | Le scan ciblé du delta n'a trouvé ni clé privée, token, mot de passe ou donnée de profil. Le marqueur `Authorization: Bearer QQ==` est le marqueur public de lecture anonyme du registre, pas un credential utilisateur. Aucun nouveau log runtime n'est ajouté. |

## Finding ouvert

| ID | Sévérité | Scénario, risque et preuve | Blocage | Disposition / propriétaire | Remédiation et retest |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `MAC15-SEC-LOW-001` | LOW | `extract_selected` exige que le nom du lien finisse par `.dylib` et que sa résolution reste sous la formule, mais pas que la cible résolue reste sous `lib/` ni soit elle-même une dylib. Une archive construite avec `lib/libmpv.dylib -> ../LICENSE` est donc acceptée par l'extracteur (`non_library_symlink_target=ACCEPTED`). Préconditions : un futur hash de bouteille malveillante/erronée doit être ajouté au verrou et accepté en revue. L'étape suivante `otool/lipo` échoue sur ce fichier avant signature/publication ; il n'y a ni traversal, ni exécution, ni substitution dans le candidat actuel. Le risque précis est un affaiblissement du fail-fast et un déni de build lors d'un futur renouvellement d'entrées. | Non pour le DEV local exact ; aucun HIGH/CRITICAL. | `OPEN`, propriétaire `/root`, à fermer avant renouvellement du verrou ou publication de ce nouveau flux. | Exiger une cible résolue sous `base/lib`, régulière et `.dylib` (ou Mach-O validée), ajouter le cas `../LICENSE`, puis rejouer les tests d'extraction et la sonde de fermeture complète. |

L'approbation tient compte de ce LOW : son chemin d'abus nécessite une modification
revue du verrou et le gate Mach-O final échoue déjà fermé. Il ne justifie pas de
bloquer ce candidat DEV local, mais il ne doit pas être omis d'une future revue de
publication.

## Preuves reproductibles

| Outil/version | Commande/action exacte | Résultat | Portée / limite |
| :--- | :--- | :--- | :--- |
| Git 2.54.0 | `git rev-parse HEAD`; `git merge-base e8d6f53 8283667`; `git diff --check e8d6f53..8283667` | SHA exact, base exacte, PASS ; 39 fichiers, +1925/-61 | Statique ; le rapport code concurrent non suivi est hors candidat. |
| Revue delta | `git diff --word-diff=porcelain 2edaab5..8283667` | Uniquement `None` → `None::<&Value>` dans le GET natif | Annotation de type sans changement de route, query, corps ou contrôle de contexte. |
| Python 3.9.6 | `python3 -B scripts/helpers/test_macos_media_inputs.py` | 6/6 PASS | Fixtures locales ; aucun téléchargement. |
| Python 3.9.6 | `python3 -B scripts/helpers/test_macos_minimum_version.py` | 9/9 PASS | Sorties `vtool` simulées ; pas un démarrage macOS15/26. |
| Python 3.9.6 | `python3 -B scripts/helpers/test_publish_macos_bundle.py`; `python3 -B scripts/helpers/test_macos_dmg.py` | 3/3 + 5/5 PASS | Publication/DMG simulés ou statiques ; aucun bundle publié. |
| Node 24.13.0 | `node --test ui/connected/progress.test.js web/test/bridge.test.js ui/connected/catalog-loading.dom.test.js` | 36/36 PASS | DOM/ponts simulés ; pas de Safari physique ni d'app native. |
| Validation du verrou | `read_lock(deploy/macos/media-sequoia.lock.json)` et contrôle métadonnées source/licence | PASS : 43/43 uniques, ARM64/15.0 ; SHA-256 verrou `3fde4800…cbd1` | Ne constitue pas un scan d'advisories ni une décision juridique de compatibilité des licences. |
| Scan ciblé | motifs clé privée/token/password sur `git diff e8d6f53..8283667` | Aucun résultat | Heuristique du delta, pas scan d'entropie ou d'historique complet. |
| Expérience adversariale | TAR temporaire `lib/libmpv.dylib -> ../LICENSE` passé à `extract_selected` | `ACCEPTED`, constat LOW ci-dessus | Temporaire local ; aucun fichier candidat écrit. |

Preuves fournies et cohérentes avec les octets de packaging inchangés par le
dernier commit : sonde de 51 Mach-O à minimum global15.0, 50 dylibs relocalisées
et chargement libmpv API2.5 sur macOS27. Le reviewer ne les a pas reconstruites.
Le build release du parent `2edaab5` avait échoué fermé avant publication sur
l'annotation corrigée par `8283667` ; la relance du candidat exact, le bundle/DMG
final, `codesign` final et les essais physiques macOS15/26 restent des gates
séparés.

## Limites de remise

La conformité juridique de distribution des composants GPL/LGPL n'est pas
déduite des seuls identifiants SPDX : le futur paquet doit réellement contenir
les notices prévues et la livraison doit vérifier les obligations de source
correspondant aux 42 formules runtime. Aucune publication n'étant autorisée ici,
ce contrôle reste un gate de release, pas une affirmation de conformité acquise.

Aucun Cargo build/test, lancement de l'app Mac, téléchargement de bouteille,
connexion Pi/NAS, mutation GitHub/cloud ou déploiement n'a été effectué par ce
reviewer. Les résultats Rust fournis (serveur148 PASS/1 NFR ignoré, domaine75
PASS avant l'assertion de réouverture) sont utiles mais ne sont pas requalifiés
en exécution indépendante exacte. Toute modification de source après `8283667`
rend ce verdict obsolète et exige une nouvelle revue exacte.
