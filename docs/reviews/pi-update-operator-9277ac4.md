# Revue securite independante — operateur Pi V4

- **Mode** : `DEV_STANDARD`, delta operateur de livraison
- **Auteur/deployeur** : `/root/ux_api_delivery`
- **Reviewer** : `/root/pi_operator_review` (distinct, aucun octet candidat ecrit)
- **Candidat exact** : `/private/tmp/chillcenter-pi-update-operator-v4.sh`
- **SHA-256** : `55c6be484d50a8065115ae4b1bf867377c24b255640c033e5fc2faf6f3289587`
- **Taille** : `6933` octets
- **Source/artefact lie** : `9277ac4d51281b3e6d72475294b1e7822cb8e913` / binaire `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`
- **Ancien binaire attendu** : `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`
- **Revue UTC** : `2026-10-14T19:10:41Z`
- **Verdict** : **APPROVED** pour ces octets exacts et l'autorite operatoire bornee donnee par l'utilisateur.

Toute modification, meme documentaire dans le script, annule ce verdict. V2
`ebd92f00...b83d0` et V3 `d27d7d09...e36c61` restent historiques et ne sont
pas approuvees.

## Controles de securite

- Autorite bornee : arret de `chillcenter`, sauvegarde privee locale, remplacement
  du seul binaire, redemarrage et test de connexion. Aucun changement NAS,
  configuration, unite systemd, TLS, identite, IAM, cloud ou exposition reseau.
- Privileges : execution root exigee, UID service fixe `997`, chemins systeme et
  stage entrant refuses s'ils sont des symlinks, hashes ancien/nouveau imposes.
  Le transport prevu par stdin SSH vers `sudo -n /bin/bash -s --` n'evalue aucune
  donnee distante avec `eval` ou `sed`.
- Sauvegarde : commande hors ligne sous `chillcenter`, cwd
  `/usr/lib/chillcenter`, sortie acceptee uniquement si elle contient le prefixe
  exact et un UUID sous la racine privee. Proprietaire/modes et publication du
  manifeste sont controles. La source approuvee copie catalogue/config/TLS dans
  la sauvegarde privee et ne parcourt pas le NAS; le script ne lit, ne hache et
  n'affiche aucune cle privee.
- Rollback : parent et cible symlink/preexistants sont refuses avant creation;
  copie R7 dans un tempfile root, hash verifie, publication no-clobber par lien
  dur. Le remplacement du binaire est un rename dans le meme repertoire.
- Signaux/atomicite : `HUP`, `INT`, `TERM` rejoignent `EXIT`. L'intention
  `stop_attempted` est armee avant l'arret. Avant `completed=1`, la recuperation
  utilise les hashes durables : ancien binaire + service arrete => redemarrage;
  nouveau binaire, service actif ou non => arret si necessaire, restauration R7
  via un nouveau stage verifie, puis redemarrage. Une precondition echouee avant
  l'intention d'arret ne change pas l'etat du service.
- Invariants : hashes config/unite et SPKI du certificat public sont compares
  avant/apres. Les sorties finales se limitent a UTC, PID, hashes et chemin UUID
  de sauvegarde; la detection de connexion ne journalise pas les sockets.
- Session/CSRF/redirect/tenant : non applicables a ce script local; aucun contrat
  HTTP, authentification ou frontiere de foyer n'est modifie.

## Preuves reproductibles

| Outil/action | Commande/scope | Resultat | Limites |
| :--- | :--- | :--- | :--- |
| `sha256sum (Darwin) 1.0` | `sha256sum /private/tmp/chillcenter-pi-update-operator-v4.sh` avant et apres les checks | PASS, hash exact ci-dessus | Verifie les octets locaux, pas la copie recue par le Pi |
| GNU Bash `3.2.57` | `/bin/bash -n ...v4.sh` | PASS | Le Pi utilisera son Bash GNU; syntaxe employee compatible |
| Auto-test candidat | `env -u CHILLCENTER_TLS_CERT_PATH /bin/bash ...v4.sh --self-test` | PASS : chemin UUID valide accepte, deux formes invalides refusees | Teste le parseur; ne simule pas systemd |
| Scan secrets borne | recherche PEM prive/password/secret assigne dans le candidat | PASS, aucun motif | Heuristique, completee par revue manuelle |
| Primitive no-clobber | `ln source target` avec cible deja presente dans un repertoire temporaire | PASS : rc `1`, cible inchangee | Simulation locale de la primitive, pas du filesystem Pi |
| Provenance artefact | hash archive puis `tar -xOzf ... chill-server | sha256sum` | archive `a202c94e...552a`; binaire `1eb24e97...181` PASS | Ne prouve pas encore le stage distant |
| Delta source | `git diff --quiet 9277ac4..HEAD -- apps crates ui scripts deploy Cargo* rust-toolchain.toml` | PASS, aucun delta source | Le checkout courant porte des carriers documentaires ulterieurs |
| ShellCheck | disponibilite locale | `NOT_AVAILABLE` | Compense par `bash -n`, auto-test et revue ligne a ligne |

## Findings et corrections

| ID | Severite | Etat | Preuve et disposition |
| :--- | :--- | :--- | :--- |
| `OP-V2-001` | HIGH | REMEDIATED en V4 | V2 armait les indicateurs apres `systemctl stop` et `mv`, permettant une interruption sans restart/rollback. V4 arme l'intention avant stop et decide le remplacement d'apres les hashes reels. |
| `OP-V2-002` | MEDIUM | REMEDIATED en V4 | V2 executait `install -d` avant de refuser un parent symlink; reproduction locale : rc 0 et chmod de la cible. V4 refuse parent/cible avant `mkdir` et publie le rollback sans clobber. |
| `OP-V2-003` | MEDIUM | REMEDIATED en V4 | V2 n'avait que `EXIT`; V4 route explicitement HUP/INT/TERM et nettoie uniquement ses tempfiles bornes. |
| `OP-V3-001` | HIGH | REMEDIATED en V4 | V3 ne rollbackait pas un nouveau service deja actif apres echec config/unite/SPKI et pouvait demarrer un service initialement inactif. V4 couvre le service actif et rend la recuperation inerte avant `stop_attempted`. |

Aucun finding ouvert HIGH/CRITICAL, aucun secret trouve, aucune nouvelle
dependance ou frontiere de confiance. Aucun acces SSH ni aucune action mutante
sur le Pi n'a ete realise pendant cette revue.

## Sonde post-deploiement bornee

La sonde optionnelle `/private/tmp/chillcenter-connection-probe.F7F5yV/src/main.rs`
est figee a `977335e0a9ba6393ba480783a7e6c5bd4d319a4f69557e80edb96035ec287e11`.
Elle utilise un repertoire client isole, exige le SPKI connu
`3018fb4d...40df`, puis n'emet que les commandes de connexion, selection locale
de profil et catalogue en lecture. Le code `UiClient` sous-jacent effectue des
GET serveur pour identite/profils/bibliotheque/preferences/catalogue; aucune
creation, preference foyer, scan, lecture media ou mutation NAS. Sa sortie ne
revele que version API, nombre de profils, nombre d'items et etat de reconnexion.
Le caller doit lui fournir le repertoire `client-*` prive nouvellement cree;
elle n'est pas une preuve avant son execution reelle apres deploiement.

## Limites et suite autorisee

- `SIGKILL`, panne d'alimentation, panne disque/systemd et concurrence d'un autre
  administrateur ne peuvent pas etre rendues atomiques par Bash; les stages et
  hashes durables rendent l'etat inspectable et la restauration R7 disponible.
- Le controle `ss` borne les connexions TCP etablies visibles au moment du stop;
  il ne prouve pas l'absence de toute session logique. L'utilisateur a
  explicitement autorise cet arret borne.
- L'approbation autorise le deployeur distinct a executer **uniquement** les
  octets V4 ci-dessus via le master SSH deja approuve, puis exige une verification
  distincte : hash/PID/restarts/confinement/TLS strict/API datee, invariants
  config/unite/SPKI et sonde client bornee. Elle ne constitue pas cette preuve.
