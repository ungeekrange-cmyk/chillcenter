# Revue sécurité indépendante — opérateur Pi/web 0.7.2

- **Type / reviewer :** operational delta, sécurité /
  `/root/v071_final_security`
- **Nouvelle source revue :**
  `3d27456a3e2c46fb2dc8937b4e00370f85d54470`
- **Runtime/paquets inchangés :**
  `a38d4b16616ebd6735fa4aab0b518e28d767079e`, déjà approuvé dans
  `docs/reviews/BUG-0127-security.md`
- **Opérateur précédemment approuvé :** commit
  `815b523a383e3d0a61e1b116f27495e1ca2584d0`, SHA-256
  `4dd8451281a6a3037861fab15eccaff8623273b558ae167dadbad51e02ed6cfb`
- **Nouvel opérateur :** SHA-256
  `12a87083be5c18cdbabade89f8a248beb7952defe2f219d25138d0b4d4e56d0c`
- **Date :** `2026-10-21T12:24:20Z`
- **Verdict :** **APPROVED**

Cette approbation porte uniquement sur les octets exacts de l'opérateur et le
delta de compatibilité ci-dessus. Elle ne prétend pas qu'une publication, un
staging, une sauvegarde, un arrêt, un remplacement Pi/web ou une vérification
post-déploiement a déjà eu lieu. Aucun Pi, NAS, GitHub, service ou réglage n'a
été contacté ou modifié par cette revue.

## Delta et confiance

Le seul changement de comportement dans l'opérateur remplace la condition
incorrecte `required_server.min == version_cible` par le contrat inclusif signé
`min <= version_cible <= max`.

- Les bornes et la cible doivent être des chaînes SemVer stables strictes
  `MAJOR.MINOR.PATCH`. La regex refuse notamment composant absent, valeur non
  chaîne, zéro initial non canonique, pré-release, métadonnée et suffixe libre.
- Les trois composantes sont converties en tuples entiers avant comparaison :
  `0.10.0` est donc correctement supérieur à `0.9.0`, sans comparaison lexicale.
- L'accès direct à `min` et `max`, puis `lower <= upper` et les deux comparaisons
  inclusives, échoue fermé sur borne absente, mal formée, inversée, cible sous la
  borne basse ou au-dessus de la borne haute.
- Le contrôle s'exécute sous `python3 -I`, après comparaison des SHA-256 attendus
  de la clé publique, des deux manifestes et des deux signatures, et avant le
  hash des payloads, l'inspection/extraction ZIP, la création du reçu de
  transition ou l'arrêt du service. Une incompatibilité ne peut donc ni muter
  le Pi ni consommer une autorité d'installation.
- Les assertions adjacentes restent inchangées : schéma 1, composants
  `server`/`web`, version cible commune, identifiant de clé commun, API exacte,
  schéma base 3, cibles `linux-aarch64`/`web` et empreintes payload attendues.
  Le manifeste web `0.7.0..0.999.999` admet ainsi correctement le serveur 0.7.2
  sans élargir le dépôt, la clé, la cible ou la version réellement installable.

L'opérateur Pi ne vérifie pas lui-même Minisign, contrainte déjà revue pour le
Pi legacy dépourvu de ce binaire. Cette limite n'est pas élargie : les quatre
fichiers clé/manifeste/signature sont liés à un reçu local par SHA-256, leurs
signatures doivent être vérifiées hors Pi avant l'invocation, puis les hashes de
payload sont comparés au contenu de ces manifestes épinglés. Toute autre paire
d'artefacts ou de reçus sort de cette approbation.

## Gardes préservées

Le diff exact contre `815b523` ne modifie que 8 lignes ajoutées et 2 retirées
dans le bloc de compatibilité. Restent byte-identiques :

- modes fermés et chemins absolus/non-symlink ;
- clé/manifeste/signature/payload épinglés, ZIP sans traversal/lien/spécial ;
- TLS strict, scan/TMDB/sockets/SQLite et second contrôle d'inactivité ;
- staging root privé sur le même système de fichiers ;
- reçu de transition durable et récupération idempotente ;
- sauvegarde cohérente bornée sous l'identité de service, nettoyage de groupe
  de processus et validation exhaustive du backup ;
- remplacement de la paire serveur/web, contrôle santé/version, comparaison
  config/certificat/bibliothèque/token/unité et rollback fail-closed.

Aucun helper, timer, politique, unité/drop-in, compte, TLS, configuration,
donnée, montage ou NAS n'est ajouté ou modifié. Aucun secret, clé privée,
adresse du foyer ou donnée utilisateur n'apparaît dans le delta.

## Preuves exécutées

| Commande / contrôle | Résultat | Portée |
| :--- | :--- | :--- |
| `git rev-parse HEAD`; `git rev-parse HEAD^` | `3d27456…`; parent Git réel `a38d4b1…` | Identité exacte de la candidate. |
| `git diff --check a38d4b1..3d27456` | PASS | Trois fichiers du delta, aucun défaut whitespace. |
| `shasum -a 256 scripts/pi-v07-legacy-server-web-operator.sh` | `12a87083…d0c` | Hash exigé par la porte opérateur. |
| `git show 815b523:scripts/pi-v07-legacy-server-web-operator.sh \| shasum -a 256` | `4dd84512…cfb` | Confirme la base opérateur approuvée historique. |
| `bash -n scripts/pi-v07-legacy-server-web-operator.sh` | PASS | Syntaxe uniquement. |
| `bash scripts/pi-v07-legacy-server-web-operator.sh --self-test` | `pi_v07_legacy_operator_self_test=PASS` | Auto-test non mutateur. |
| `python3 scripts/test-pi-v07-legacy-operator.py` | **8 PASS**, 0 échec, 27,7 s | Flux complet simulé, état préservé, sauvegarde/recovery/interruptions, modes/ZIP et plage SemVer fermée. |
| Inspection du diff `815b523..3d27456` limité au script | 8 ajouts / 2 retraits, bloc SemVer seulement | Prouve l'absence de delta dans les gardes opérationnelles historiques. |
| Scan lexical des trois fichiers modifiés | Aucun secret, fichier de clé privée ou IP privée | Scan de motifs, pas un scan d'entropie ni d'historique. |

Note de topologie : malgré la désignation « parent `258518d` » dans un autre
handoff, Git établit que `3d27456^` est `a38d4b1`. `258518d` est un porteur de
rapports frère, lui aussi enfant de `a38d4b1`; il ne change pas le runtime ni les
octets de l'opérateur. Une intégration future de ces rapports peut être un
porteur documentaire, mais toute modification des octets opérateur exige une
nouvelle revue et un nouveau hash.

## Findings et porte d'exécution

Aucun finding Critical, High, Medium ou Low n'est ouvert dans ce delta. La
correction restaure le contrat de compatibilité sans affaiblir la confiance ou
la récupération. **L'opérateur exact `12a87083…d0c` est approuvé.**

Avant toute mutation Pi, le coordinateur doit encore lier les artefacts serveur
et web 0.7.2 publiés aux manifestes/signatures effectivement vérifiés, à la clé
publique, aux hashes, build web, cible ARM64 et anciennes empreintes attendues,
puis obtenir un preflight TLS/inactivité frais. L'exécution autorisée doit créer
une sauvegarde privée fraîche et conserver NAS, réglages, TLS, helper/timer,
unité/drop-in et exception AF_NETLINK inchangés. Une instance distincte doit
ensuite vérifier la paire déployée, la sauvegarde et ces invariants ; sinon la
livraison ne peut pas être déclarée réussie.
