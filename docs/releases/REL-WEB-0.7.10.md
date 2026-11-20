# Client web0.7.10 — sous-titres natifs

## Français

Candidat web0.7.10/séquence11, non publié et non installé à ce stade.
Compatible avec Mac0.7.8, serveur0.7.7 et thèmes0.7.1.

Le lecteur web affiche les pistes texte MKV UTF8/ASS/SSA/WebVTT et les fichiers
externes SRT/VTT/ASS/SSA accessibles sous le bail existant. Préférence de langue
à l’ouverture, choix manuel, Désactivés, sauts et réponses tardives sont gérés.
Un TextTrack natif unique est réutilisé par vidéo, sans HTML de sous-titre
exécuté. Le seek utilisateur garde la priorité pendant la reconstruction du
tampon ; un cue qui recouvre la reprise n’est pas perdu. Aucun transcodage serveur.

Le bouton « Installer l’app » est retiré de l’interface web. Aucun changement
DNS/certificat, confiance système ou accès réseau. La coque versionnée existante
reste séparée du lecteur ; API et médias ne sont jamais précachés.

400 tests JavaScript,28 docs et parcours Chromium complet des sous-titres,
régressions vidéo native H.264/HEVC, repli mobile et coque sans bouton Installer
PASS. Revue indépendante APPROVED_SCOPED. Projet dernière chance :35 pistes texte,
dont deux françaises ; cue française native active en lecture à5001,97s, vérifiée
via plages SSH bornées dans un contrôleur isolé sans bail ni écriture personnelle.
Safari/iPhone et Pixel/Android physiques non testés. PGS/VobSub, sous-titres
intégrés MP4 tx3g et effets ASS avancés ne sont pas implémentés.

Retour arrière : paquets/slots signés existants conservés ; parcours administrateur
requis par l’anti-rollback. Aucun changement de schéma ou de fichiers NAS.

## English

Web0.7.10/sequence11 candidate, not yet published or installed. Compatible with
Mac0.7.8, server0.7.7 and themes0.7.1.

Native browser captions for MKV UTF8/ASS/SSA/WebVTT text and lease-bound external
SRT/VTT/ASS/SSA files. Initial language preference, selection, Off, seek and stale
responses are handled. One programmatic TextTrack is reused per video; subtitle
text never becomes executable HTML. Explicit user seeks win over native clock
corrections during buffer resets. No server transcoding.

The in-app Install button is removed. No DNS, certificate, OS trust or network
access changes. Existing versioned shell caching stays separate from media/API.
400 JS tests,28 docs and full Chrome subtitle, native H.264/HEVC, synthetic mobile
and shell regressions pass. Independent review approved. Actual French captions
in the reported movie were verified through bounded SSH ranges in an isolated
controller without household leases or personal writes. Physical iPhone/Pixel
validation remains pending; bitmap subtitles, MP4 tx3g and advanced ASS effects
are not provided. Existing signed rollback packages/slots retained, no migration.

[BUG0144](../debug/BUG-0144-web-subtitles.md) · [ADR0044](../architecture/decisions/ADR-0044-web-native-subtitles.md).
