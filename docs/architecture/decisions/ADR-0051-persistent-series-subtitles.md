# ADR-0051 — Choix des sous-titres conservé après fermeture

26 novembre 2026 · Continuité du lecteur sur macOS et sur le web.

Le choix manuel des sous-titres d’une série est enregistré au moment de la
sélection. Le réglage appartient au serveur, au profil actif et à l’identité
de série du lecteur. Une piste est décrite par sa langue, son titre et sa
provenance. « Désactivés » est un choix explicite.

Sur Mac, les réglages occupent une table SQLite locale dans le dossier
privé de l’app. Le navigateur conserve une entrée par série dans le stockage
du site. Une écriture remplace la préférence de la série concernée. On retrouve
chaque choix à la prochaine lecture sur le même client. Ce stockage fonctionne
avec le serveur déjà installé.

Le lecteur privilégie la piste qui correspond au choix conservé. En cas
d’absence de correspondance, il applique les préférences du profil. Le choix
explicite « Désactivés » laisse les sous-titres désactivés. Si le stockage
est indisponible, la sélection reste utilisable dans le lecteur courant.

Voir [BUG-0153](../../debug/BUG-0153-subtitles-lost-after-close.md).
