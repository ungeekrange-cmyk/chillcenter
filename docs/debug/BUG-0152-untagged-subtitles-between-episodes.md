# BUG-0152 — Choix des sous-titres entre deux épisodes

26 novembre 2026 · macOS 0.7.12 et web 0.7.16.

Après activation des sous-titres sur Avatar Seven Havens S01E12, la piste
restait désactivée en S01E13. Les deux fichiers contiennent chacun une piste
SubRip sans titre ni langue déclarée. En lecture seule, le démultiplexeur du
client web a identifié la piste S_TEXT/UTF8 de l’épisode suivant avec la langue
`und`. La recherche utilisée dans la version publiée échouait sur cette piste.

La correspondance cherche d’abord le titre et la langue. Pour une piste
sans langue, une seule candidate sans langue permet désormais de reprendre
le choix. Si plusieurs pistes correspondent, le lecteur utilise les réglages
du profil. Le choix explicite « Désactivés » est conservé.

La version précédente transportait le choix par la commande « À suivre ».
Le lecteur garde désormais le choix manuel par serveur, profil et série en
mémoire pendant l’ouverture de l’application ou de l’onglet. On le retrouve
en ouvrant un épisode depuis la bibliothèque lorsque l’identité de série
est disponible.

Les tests web reproduisent l’échec sur la piste sans langue et la perte du
choix au retour dans la bibliothèque. Ils vérifient aussi la désactivation
et l’isolation entre profils. La régression macOS vérifie la correspondance
avec la piste sans langue, l’ambiguïté et le respect d’une langue explicite.
