# BUG0140 — Reprise existante absente de l’Accueil

8 novembre 2026. Demande utilisateur : retirer le raccourci « En cours », déjà
présent dans l’Accueil, et corriger un film commencé qui n’apparaît pas.
Statut : corrigé et testé localement, publication et installation non réalisées.

## Cause confirmée

Lecture seule de l’API du Pi0.7.5, via le certificat déjà approuvé par le Mac :
film disponible, non vu, position persistée de 3 025 917 ms ; aucune durée mesurée.
Le GET Home omet le film. La requête de progression mesurée ne contient aucune
observation pour ce profil. Aucun besoin de modifier l’historique ou de sonder
le NAS pour établir la cause. L’essai SSH a échoué dans l’agent1Password ; le
transport HTTPS vérifié a permis les lectures sans contournement TLS.

H1 confirmée : le filtre Home des films exigeait à la fois position>0 et durée
mesurée>0. Une reprise héritée d’un ancien serveur/import est valide pour
reprendre la lecture, même sans pourcentage disponible. Les séries n’avaient
pas cette exclusion. Reproduction synthétique : film hérité à 3 025 917 ms et
film mesuré ; avant correction un seul apparaît, au lieu de deux.

## Correction

Home accepte un élément disponible, non vu, avec position>0 ; `progress` reste
null tant que la durée n’a pas été mesurée. Aucune durée TMDB, aucun pourcentage
inventé, aucune modification des baux ou du marquage vu. Plafond20 et regroupement
par série/saison conservés. Pas de changement de schéma ni de forme d’API.

Retrait du raccourci « En cours » sur Mac/web et dans le menu mobile, ainsi que
son filtre local distinct à5%. Barres de progression et pourcentages mesurés
restent disponibles dans les fiches et vignettes. Voir [ADR0041](../architecture/decisions/ADR-0041-home-resume-without-duration.md).

## Vérifications

- Régression domaine rouge avant correctif, verte après ; réouverture de base,
  séparation des profils, durée mesurée facultative, vus/indisponibles exclus.
- 83 tests domaine, test HTTP Home avec reprise sans durée et chemin local masqué,
  368 tests JavaScript partagés/web, Clippy domaine/serveur strict PASS.
- Chromium du client web reconstruit PASS : reprise héritée sans pourcentage,
  raccourci absent, navigation Accueil/catégories et variantes mobile/bureau.
  Les fixtures de smoke antérieures supposaient encore « Toute la collection »
  au démarrage ; elles utilisent désormais l’Accueil puis les catégories visibles.

Revue indépendante du candidat figé APPROVED, aucun point bloquant.
Cache de compilation temporaire retiré après les contrôles, journaux conservés.

Aucune donnée de production changée. Pas de nouvelle version publiée ou
installée. Rollback logiciel : revenir au commit précédent ; aucune migration
inverse ni suppression de données nécessaire.
