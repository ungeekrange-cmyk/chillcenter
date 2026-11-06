# 0.7.4 — maintien éveillé / playback wake lock

Publication : 2026-11-06. Mac **0.7.4/build14**, web **0.7.4**,
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
Les contrôles des paquets finaux sont détaillés ci-dessous. La publication GitHub
est distincte de leur installation sur les appareils.

Construction isolée dans `/private/tmp/chillcenter-playback-wake.BEMWYx` :
sources existantes et cache Cargo `chillcenter-fixes-web-parity/target` réutilisés.
Nouveaux sous-dossiers `release-0.7.4`, `published-0.7.3` et `media-sequoia` :
livraison immuable, récupération de paquets signés inchangés et entrées épinglées
nécessaires car les anciens dossiers médias n'ont plus leur contenu vérifiable.
Ces dossiers sont conservés pour audit/reconstruction ; aucune purge.

Retour arrière : anciennes releases immuables conservées. Une régression exige
une nouvelle version corrective ou la récupération administrateur vérifiée ;
ne jamais désactiver les signatures ni déplacer un tag publié.

## Paquets finaux / final packages

Source immuable : `d1d663e5a5d2f713c0983a28ebfb3ce617196080`. Tags
`mac-v0.7.4`, `web-v0.7.4`, `v0.7.4` ; aucun tag serveur0.7.4 ou thème créé.
Revue indépendante finale source et inventaires scellés par `/root/release_review` :
**APPROVED_FOR_PUBLICATION**, Mac/web uniquement. Les premiers candidats
`671fafe` sont exclus de tous les inventaires publiés.

- Bundle final Apple Silicon0.7.4/build14 : signature stricte, minima des51
  Mach-O<=15.0,50 bibliothèques relocalisées et chargement libmpv API2.5 PASS.
- DMG vérifié/monté en lecture seule, identité et lien Applications PASS.
- Archive de mise à jour TAR avec racine standard, sans AppleDouble ; manifeste
  et signature Tauri/Minisign, séquence5/source/digest/tailles PASS.
- Web build `ebdafd2f63a3`,102 fichiers ; ZIP installer avec wrapper inchangé
  vérifié contre le manifeste serveur0.7.3 et signature existante. Guides FR/EN
  de distribution générés avec les versions Mac/web0.7.4, exemples serveur0.7.3 conservés.
- Sources correspondantes :42 formules runtime,248 empreintes internes vérifiées,
  sources applicatives exactes avec ZIP9.0.5. Entrées média inchangées.
- Tests après correctif ZIP :27 updater (dont dual-header hostile) et40 desktop
  PASS, Clippy strict PASS. Les20 tests lecteur et347Node du correctif restent valides.
- Inventaires : Mac10, web4, umbrella17 assets ; pas de paquet serveur/Windows
  republié. Signatures et SHA256SUMS vérifiés avant publication.

[Release0.7.4](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7.4) ·
[DMG Mac](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.4/Chill-Center-0.7.4-macos-arm64.dmg) ·
[Web installer](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.4/Chill-Center-web-0.7.4-install.zip).
Serveur0.7.3 et thèmes0.7.1 restent dans les releases précédentes. Les anciennes
alertes de dépendances ne sont pas toutes clôturées ; voir BUG0133 pour le périmètre.

| Paquet | SHA-256 |
| --- | --- |
| `Chill-Center-0.7.4-corresponding-sources.tar.gz` | `2807eb92c47762cadee261d247a879fc6123307d40bb82e2857ee053ed6c1fb5` |
| `Chill-Center-0.7.4-macos-arm64.dmg` | `51d89031ddc1ef0184f769e5590aec2b27e788da9f03715e372d258fd96557c8` |
| `Chill-Center-web-0.7.4-install.zip` | `a1f24459b1ab1368597caa103ff6684ac99b4c8cb47a7e72fd18d82b45056484` |

## Vérification publique finale / final public verification

Les trois releases sont publiques, non-prerelease ; `v0.7.4` est Latest.
Les31 assets des trois inventaires correspondent aux noms, tailles et digests
SHA-256 scellés. Les20 fichiers uniques ont été téléchargés depuis les releases
publiques : chaque empreinte correspond aux octets locaux et toutes les signatures
Minisign ont été vérifiées avec la clé épinglée. Aucun fichier ni tag ancien remplacé.

GitHub au SHA source final :
[Shipping dependency security PASS](https://github.com/ungeekrange-cmyk/chillcenter/actions/runs/42304145672) ;
[CodeQL PASS](https://github.com/ungeekrange-cmyk/chillcenter/actions/runs/42304145968).
Les README FR/EN et guides courants sont synchronisés après publication avec les
versions indépendantes et les liens réels ;33 tests doc/version/release PASS.
Le commit documentaire est distinct des tags immuables du binaire.

Rapport local de vérification :
`/private/tmp/chillcenter-playback-wake.BEMWYx/release-0.7.4/public-verification.json`.
Dossier de livraison et premiers candidats exclus conservés (environ1,4Gio avant
téléchargements), entrées médias75Mio, archives historiques308Mio et téléchargements
de contrôle conservés pour audit. Le cache Cargo existant reste réutilisé ;
aucun environnement dupliqué ni purge. Les intermédiaires exclus peuvent être
nettoyés sur autorisation ciblée après l'audit, sans toucher aux sources/releases.
