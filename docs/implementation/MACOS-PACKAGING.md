# Packaging macOS Apple Silicon

Paquet courant14 octobre2026, source `126da0af167ee62588d92d90297f2dfc51962676` : bundle75MiB,48dylibs/41formules, signature ad hoc et `--verify-bundle` API2.5 PASS. Destination de remise `artifacts/macos/Chill Center-126da0a.app`. Chemin/hashes/preuves dans [COMPLETION-EVIDENCE](COMPLETION-EVIDENCE.md). Les commandes/hashes plus bas décrivent le premier paquet autonome historique, jamais écrasé.

Status au 2026-10-14 : le script produit un bundle de développement autonome
Apple Silicon, signé ad hoc. Il ne produit ni signature Developer ID, ni
notarisation, ni installateur de distribution.

## Contrat du bundle

Le point d'entrée est `scripts/package-macos.sh`. Il accepte en unique argument
un nouveau chemin absolu se terminant par `.app` et refuse de remplacer un
bundle existant. Il exige un hôte macOS arm64, Cargo/Rust et l'installation
Homebrew Apple Silicon de `mpv` comme entrées de packaging. Homebrew n'est pas
une dépendance d'exécution du bundle généré.

Le résultat contient :

- `Contents/MacOS/chillcenter-desktop`, arm64 uniquement ;
- `Contents/Frameworks/libmpv.dylib` et toute sa fermeture transitive de
  bibliothèques non-système, arm64 uniquement ;
- `Contents/Resources/AppIcon.icns`, dérivée de l'asset approuvé
  `icons/branding/vector/default.png`, et le SVG approuvé conservé comme
  provenance ;
- `Contents/Resources/licenses`, avec inventaire, versions, URL des sources
  publiées, recettes Homebrew, SBOM disponibles et fichiers de licence installés
  pour chaque formule copiée.

Chaque ID Mach-O copié devient `@rpath/<nom>` et chaque lien interne devient
`@loader_path/<nom>`. Les rpaths de gestionnaires de paquets sont supprimés.
Le script valide la fermeture après copie puis après signature, charge réellement
le `libmpv.dylib` signé et exige une API client mpv majeure au moins égale à 2.
La signature ad hoc se fait de l'intérieur vers l'extérieur et est contrôlée avec
`codesign --deep --strict`.

Les identifiants de licence exposés sont les métadonnées des formules Homebrew,
pas une interprétation juridique de Chill Center. L'inventaire généré dans
`THIRD-PARTY-NOTICES.md` fait foi pour un bundle donné. Le composant mpv est
distribué selon GPL-2.0-or-later d'après sa formule ; les textes et sources
précis des dépendances sont inclus ou référencés dans ce même inventaire.

## Commande reproductible exécutée

```sh
env CARGO_HOME=/private/tmp/video-catalog-cargo \
  RUSTUP_HOME=/private/tmp/video-catalog-rustup \
  CARGO_TARGET_DIR=/private/tmp/chillcenter-mac-bundle.Vw7Gev/cargo-target \
  CARGO_NET_OFFLINE=true PYTHONDONTWRITEBYTECODE=1 \
  PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin \
  /bin/bash scripts/package-macos.sh \
  '/private/tmp/chillcenter-mac-bundle.Vw7Gev/artifacts/Chill Center.app'
```

Résultat observé : 48 dylibs arm64 issues de 41 formules ont été copiées et
relocalisées ; le chargeur a ouvert le libmpv embarqué et lu l'API client 2.5.
Le bundle final pèse 74 MiB, dont 61 MiB de frameworks et 1,3 MiB de notices et
preuves de licence. La version mpv relevée est `0.41.0_10` ; la version FFmpeg
réellement résolue dans la fermeture est `9.0.2_1`.

## Vérifications exécutées

- `bash -n scripts/package-macos.sh` : succès ;
- compilation syntaxique des deux helpers Python : succès ;
- build Cargo release verrouillé et hors-ligne : succès, avec deux avertissements
  préexistants de code mort dans `native_player.rs` ;
- `plutil -lint`, `file` et `lipo -archs` : plist valide, exécutable, icône et
  bibliothèques valides, arm64 uniquement ;
- inventaire : 48 dylibs, toutes présentes et résolues ;
- `otool -L` sur l'exécutable et toutes les dylibs : aucun lien `/opt/homebrew`,
  `/usr/local` ou `/opt/local` ;
- `codesign --verify --deep --strict --verbose=2` : succès, signature ad hoc ;
- chargement post-signature de `Contents/Frameworks/libmpv.dylib` : succès,
  API client 2.5 ;
- extraction de l'ICNS final par `iconutil` : succès.

Empreintes du candidat local :

- exécutable : `d2470d3eae80e4be1319d200255c4a285ced2e95b779cdaa0bf3747c3df620a9` ;
- libmpv : `a548453541616ee85970fd31da1aeafd7c7bf501a552e63502f6d69abbac31d7` ;
- AppIcon.icns : `df2685a364690fa7e5f29017a0fcd00481cf830bda74fe76a76b35a607f075b6` ;
- notices tierces : `ca8dfb304d7db7228994d9ffadfcbf24febfd43ffbaed2eb19f0b3afb1efc69f`.

## Non exécuté

- lancement graphique et lecture vidéo de bout en bout sur un Mac propre sans
  Homebrew : **NOT_RUN** ;
- validation visuelle de l'icône dans Finder/Dock : **NOT_RUN** ;
- signature Developer ID payante, notarisation et contrôle Gatekeeper de
  distribution : **NOT_RUN** ;
- création/test d'un DMG ou installateur : **NOT_RUN** ;
- cible Intel ou bundle universal2 : **NOT_RUN**, hors cible arm64 ;
- déploiement distant ou publication : **NOT_RUN**.

Le packaging ne modifie aucune donnée utilisateur, configuration, vidéo ou NAS.
Le rollback consiste à conserver/lancer le bundle précédent ; le script ne
remplace jamais un `.app` existant.
