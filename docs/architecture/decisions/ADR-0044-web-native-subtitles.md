# ADR0044 — Sous-titres texte natifs dans le client web

20 novembre 2026. État : IMPLEMENTED_REVIEWED, revue source indépendante APPROVED_SCOPED.
Complète ADR0015/0020 et remplace les stubs de sous-titres du lecteur web.

Les sous-titres intégrés MKV sont démultiplexés chez le client et rendus par
les TextTrack/VTTCue de l’élément vidéo, pour suivre son horloge et sa
présentation native. Le serveur continue à transmettre les octets sans
transcodage. Les tracks texte n’entrent pas dans MediaSource.

Pour la vidéo canvas, le master audio conserve son horloge ; un overlay limité
à `textContent` sert de repli. SRT/VTT/ASS/SSA externes passent exclusivement
par l’API existante sous session/génération. Annulation et epochs isolent les
sélections et fermetures. Les cues externes bornées restent disponibles pour
les sauts ; les cues intégrées anciennes sont purgées sans recréer la piste.

Le texte UTF8 du Block Matroska et sa durée font foi. BCP47 prime sur la langue
legacy. Les tags de formatage connus sont réduits au texte ; caractères de
marquage échappés dans les cues natives, aucun DOM HTML issu des sous-titres.
Les identifiants de pistes et positions restent locaux ; aucune écriture NAS
ou modification d’historique n’est nécessaire pour les tests.

Options écartées : incruster côté serveur (charge et transcodage interdits),
overlay unique en remplacement des pistes vidéo natives (ne suit pas le plein
écran natif mobile), chargement de toutes les langues en mémoire sans borne.
Limites : texte uniquement, effets ASS avancés et bitmap non implémentés ;
les appareils physiques iPhone/Pixel restent à vérifier.

Le bouton Installer l’app est retiré de la construction web à la demande de
l’utilisateur. La coque/worker versionnés existants restent indépendants du
lecteur et ne mettent en cache ni API ni vidéo. Aucun changement DNS, certificat,
confiance système ou exposition réseau n’accompagne ce correctif.

Références : [Matroska Subtitles](https://www.matroska.org/technical/subtitles.html),
[Matroska Elements](https://www.matroska.org/technical/elements.html),
[TextTrack](https://developer.mozilla.org/en-US/docs/Web/API/TextTrack).

Une unique piste est créée via addTextTrack par élément vidéo ; le sélecteur
de l’application choisit la langue. Chaque changement retire les cues détenues,
Off désactive la piste, la fermeture la vide/désactive avant de détruire la vidéo.
Le parcours Chrome complet et400 tests JS/28 docs passent ; tests physiques
Safari/iPhone et Chrome/Pixel toujours non revendiqués.
