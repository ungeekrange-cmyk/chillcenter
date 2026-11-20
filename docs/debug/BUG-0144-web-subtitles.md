# BUG0144 — Sous-titres absents dans le lecteur web

20 novembre 2026. Signalement : Projet dernière chance ne propose/affiche pas
ses sous-titres dans le client web. Le contrôle SSH en lecture seule identifie
35 pistes S_TEXT/UTF8, dont deux françaises. Aucun état personnel modifié.

Cause : le démuxeur MKV supprimait TrackType17 ; le moteur web n’exposait que
l’audio et `selectSubtitle` était vide. Les fichiers externes avaient une API
serveur sous bail, mais aucun branchement dans le lecteur web.

Correction : pistes texte UTF8/ASS/SSA/WebVTT conservées avec la langue BCP47
prioritaire. Uniquement les paquets sélectionnés sont copiés vers le renderer.
Les horodatages et durées du conteneur alimentent un TextTrack/VTTCue natif du
`<video>` ; un overlay texte est réservé au master audio du décodage canvas.
Le texte reste du texte, jamais du HTML exécuté. Les WebVTT Matroska conservent
le Block entier ; les métadonnées de position ne sont pas interprétées.

Au seek, la cue précédente de la piste sélectionnée peut apporter un texte
qui recouvre la position demandée. Un seul cluster indexé est sondé ; le
curseur vidéo reste sur son point d’accès aléatoire, y compris après une longue
période sans dialogue. Changement de langue/saut sérialisés ; ancienne sélection
et réponses externes tardives ne rétablissent jamais une piste désactivée.
Les fichiers SRT/VTT/ASS/SSA externes restent bornés à16Mio/20000cues et sous
le bail existant. Ils sont conservés pour les sauts, sans nouvelle requête.

Le test navigateur a aussi révélé une résolution du seek contre un ancien
tampon audio pendant sa suppression : horloge restée vers8,7s après une demande
à1s. Ré-ancrer la position après purge/rewind corrige ce cas. L’amorce reste
courte et le décodeur vidéo reste celui du navigateur.

Preuve réelle : Chrome, copie logique du lecteur utilisant uniquement des
plages SSH bornées du fichier NAS, sans bail du foyer. À5001,97s, piste
française sélectionnée, TextTrack `showing`, cue active5001,042–5002,458s,
texte non vide et aucune erreur native ; aucune réplique enregistrée.
Tests ciblés : parseurs, langue contradictoire, cue multiligne, cue longue,
réponse tardive ignorant AbortSignal, relecture à la position demandée.
400 tests JS,28 docs, parcours Chrome complet des sous-titres intégrés et
externes, langues/Off/sauts, limites/échappement et réponses tardives PASS.
Régressions vidéo native H.264/HEVC, mobile synthétique et coque web sans
bouton Installer PASS. Aucune validation physique Safari/iPhone ou Chrome/Pixel.

Les pistes image (PGS/VobSub), effets typographiques ASS avancés et sous-titres
intégrés MP4 tx3g restent hors du décodage texte ajouté ici.
[ADR0044](../architecture/decisions/ADR-0044-web-native-subtitles.md).

La piste native utilise `addTextTrack`, réutilisée et vidée à chaque changement.
Une balise track sans source pouvait contenir des cues sans les activer sur le
chemin externe ; le test navigateur complet a reproduit puis validé le correctif.
Le reset ignore les corrections d’horloge natives intermédiaires ; les requêtes
explicites de seek restent prioritaires. Une régression rouge/verte prouve
qu’une correction8,7s ne remplace plus une demande1s.
