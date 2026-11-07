# ADR-0038 — Accueil personnel en trois rangées

2026-11-07. Implémentation demandée explicitement par l'utilisateur ; confirmation
« 3 rangées de20 éléments maximum ». Complète ADR-0012, ADR-0025 et ADR-0035.

Accueil remplace la destination « Toute la collection ». Recherche et catégories
continuent à ouvrir le navigateur catalogue. Les rangées horizontales sont, dans
cet ordre : Reprendre, Ajouts récents, Pour vous ; les rangées vides sont masquées.
Une série n'apparaît qu'une fois par rangée. Reprendre exige une saison commencée
(par épisode vu ou reprise mesurée) mais non terminée. Une saison terminée suivie
d'une autre jamais commencée est exclue. La carte ouvre la saison choisie.
Les ajouts utilisent la date maximale des membres disponibles d'une série.

Une lecture GET profil/home produit au plus60 fiches. Requête SQLite unique,
matérialisation plafonnée à10000 vidéos, erreur explicite au-delà. Aucun scan NAS,
appel TMDB/IA ou réseau externe pour composer l'Accueil. Suggestions locales sur
les cinq dernières entités vues datées, score genres/distribution/réalisation,
exclusion des sources et regroupement des séries, départage déterministe.

Le modèle n'avait pas de date de visionnage. Ajout nullable last_watched_at, daté
uniquement lors d'une transition explicite non-vu→vu ou d'une progression validée
qui passe à vu, y compris une proposition IA explicitement confirmée. Aucune
ancienne date n'est inventée. Un trigger invalide la date sur changement du statut
vu, y compris par un ancien binaire ; le nouveau code date la transition dans la
même transaction. Les exports/imports nouveaux conservent la date, les anciennes
archives restent lisibles. Les anciens importeurs stricts refusent le nouveau
champ s'il est présent. La base reste lisible par les binaires précédents.

Client Mac : lecture HTTPS préparée sous verrou puis exécutée hors du verrou
client/lecteur. Web : une seule route datée du profil courant. Fermeture/changement
de contexte invalide réponses et cartes ; requêtes coalescées, aucun polling
pendant lecture/page cachée. Un404/405 propose une mise à jour serveur, sans casser
le catalogue. Une erreur réseau conserve uniquement la vue du même profil.

Aucun déploiement ni nouvelle publication autorisé implicitement par cet ADR.
Rollback logiciel : versions précédentes, colonne additive laissée en place ;
aucune suppression de donnée ni migration inverse destructive.
