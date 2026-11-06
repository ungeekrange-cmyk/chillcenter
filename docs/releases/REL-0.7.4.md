# 0.7.4 — maintien éveillé / playback wake lock

Préparation / preparation : 2026-11-06. Mac **0.7.4/build14**, web **0.7.4**,
séquence **5** pour ces deux composants. Serveur **0.7.3/séquence4** et thèmes
**0.7.1/séquence2** inchangés. API **2026-10-14**, schéma **3**, rendu **2** conservés.

## Français

- Maintien éveillé uniquement pendant la lecture vidéo effective, sur Mac et web.
- Pause, arrêt, fin, erreur, attente de données et transfert libèrent le maintien
  éveillé. Ouvrir simplement l'application ne bloque pas la veille.
- Sur Mac : assertion IOKit liée au lecteur, sans modifier les réglages système ;
  veille volontaire et fermeture du capot restent prioritaires.
- Sur le web : API Wake Lock du navigateur, contexte sécurisé requis et page
  visible. Sur HTTP LAN non sécurisé ou si le navigateur refuse, la lecture
  reste disponible mais le maintien éveillé ne peut pas être garanti.
- Correctif de dépendance ZIP 9.0.5 dans le client Mac, suite à l'alerte
  GHSA-m665-v7pm-7grj ; le lecteur ZIP à accès aléatoire et les signatures
  existantes empêchent déjà le scénario de redirection décrit, régression dédiée.

La publication fournit les paquets pour le mécanisme de mise à jour existant.
Le Mac demande « Installer maintenant » ; le web suit sa politique configurée.
Aucune connexion directe au Pi/NAS ni modification des réglages du foyer.

## English

- Keep the display/system awake only during effective video playback, on Mac and web.
- Pause, stop, end, errors, buffering and transfers release the inhibitor.
  Opening the application alone does not prevent sleep.
- Mac uses a player-owned IOKit assertion without changing system settings;
  explicit sleep and closing the lid remain effective.
- Web uses the browser's Wake Lock API and requires a secure context and a
  visible page. Plain LAN HTTP and browser refusals cannot guarantee inhibition;
  playback remains available.
- Mac updates its ZIP dependency to 9.0.5 for GHSA-m665-v7pm-7grj, with a
  dual-header regression. Existing signed inputs and random-access extraction
  already avoid the advisory's mixed-parser path.

Publication supplies packages for the existing updater. Mac requires “Install now”;
web follows its configured policy. No direct Pi/NAS connection or household
settings change is performed by this publication.

## Vérification / verification

Correctif / functional source : `935b0fa`, base `bacee3c`.
Revue indépendante source/sécurité par `/root/release_review` : **APPROVED**
le 6 novembre 2026 pour `935b0fa` puis `671fafe`. Le contrôle de publication a
ensuite imposé le correctif ZIP9.0.5 documenté dans
[BUG0133](../debug/BUG-0133-release-zip-advisory.md). Aucun changement d'API,
confiance, authentification, autorisation ou journalisation sensible.

- 347 tests Node UI/web PASS, dont 13 régressions Wake Lock.
- 60 tests Rust desktop/lecteur PASS, assertion IOKit et lecture libmpv réelles.
- Clippy strict, formatage, diff, compilation desktop et build web PASS.
- 33 tests documentation/version/signature et 21 tests packaging Mac PASS.

Pas de test humain de veille physique, GUI assemblée, iPhone ou matériel
macOS15/26 revendiqué. Mac signé ad hoc, sans notarisation Apple.
Les contrôles des paquets finaux et leur publication sont consignés après construction.

Construction isolée dans `/private/tmp/chillcenter-playback-wake.BEMWYx` :
sources existantes et cache Cargo `chillcenter-fixes-web-parity/target` réutilisés.
Nouveaux sous-dossiers `release-0.7.4`, `published-0.7.3` et `media-sequoia` :
livraison immuable, récupération de paquets signés inchangés et entrées épinglées
nécessaires car les anciens dossiers médias n'ont plus leur contenu vérifiable.
Ces dossiers sont conservés pour audit/reconstruction ; aucune purge.

Retour arrière : anciennes releases immuables conservées. Une régression exige
une nouvelle version corrective ou la récupération administrateur vérifiée ;
ne jamais désactiver les signatures ni déplacer un tag publié.
