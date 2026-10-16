# Vérification finale indépendante — médias Pi / NAS

- **Serveur** : source `126da0af167ee62588d92d90297f2dfc51962676`, binaire Pi R7 déjà vérifié, service actif PID 3016 / `NRestarts=0`.
- **Client testé** : `624f2447b5c0ede6b4a053ea3fa7b82db85c0121` (les revues source signalées sont amont; elles ne sont pas rejouées ici).
- **Données de test** : uniquement les trois profils QA, ensuite supprimés par le flux HTTPS TLS strict documenté.
- **Verdict fonctionnel/runtime** : `APPROVED_SCOPED`.
- **Verdict performance trois flux** : `FAILED` — le critère ne peut pas être déclaré satisfaisant.
- **Verdict deux flux** : `QUALIFIED_PASS` — preuve réelle de dix minutes, avec buffering observé sur Dune 2, donc pas une validation universelle des NFR.

## Vérification indépendante après nettoyage

| Contrôle | Observation indépendante | Résultat |
| :--- | :--- | :--- |
| Nettoyage QA | `GET /api/v1/profiles` sous TLS CA stricte retourne `[]` | PASS |
| Serveur / bibliothèque | API v1 0.1.0, racine `/srv/volumes/video`, révision 0 | PASS |
| Persistance catalogue | comptage SQLite sous l’UID de service, sans titre lu : `media = 2496` | PASS |
| Données et clé TLS | état `0700`, SQLite `0600`, clé privée seulement inspectée par métadonnée `0600` (jamais lue) | PASS |
| NAS | CIFS `//nas.local/Video`, `ro,nosuid,nodev,noexec` | PASS |
| Transcodage serveur | aucun processus encodeur observé | PASS |

## Revue des preuves de lecture

Les journaux locaux ont été inspectés contre leur méthode : trois processus client headless, profils QA distincts, durée demandée de 1 800 s, progression/lease/reprise enregistrées et métriques Pi périodiques. Ils prouvent les résultats suivants, pas davantage.

| Essai | Preuve | Verdict |
| :--- | :--- | :--- |
| Trois flux / 30 min | Dune 1 : `PLAYBACK_PASS`, 1 830 s, 0 buffering; 720p : `PLAYBACK_PASS`, 1 830 s, 323 buffers; Dune 2 : erreur « Decoder made less than80percent real-time progress », dernier point 1 814 s / position 1 412.995 s / 4 102 buffers | FAILED |
| Métriques trois flux | 61 échantillons, RSS max 24.23 MiB, CPU moyen 20.50 % d’un cœur / max 22.95 %, swap 0, température max 49.388 °C | OBSERVED — ne compense pas l’échec Dune 2 |
| Deux flux / 10 min | deux `PLAYBACK_PASS`, reprise autre appareil et refus de lease relâchée documentés; Dune 1 : 0 buffer; Dune 2 : 184 buffers | QUALIFIED_PASS |
| Métriques deux flux | 21 échantillons / 600.06 s, RSS max 24.36 MiB, CPU moyen 16.16 % / max 17.43 % d’un cœur, swap 0, température max 49.388 °C | OBSERVED |

## Limites explicites

- Aucun verdict « tous les NFR » ni « trois flux 1080p validés » : Dune 2 échoue en lecture triple et les deux flux ont du buffering sur Dune 2.
- Restent `NOT_RUN` : contrôle humain audiovisuel, retour arrière interactif, qualification formelle de seuil RSS maximal, pannes/déconnexion NAS et leur reprise.
- Cet audit n’a effectué aucun restart, écriture, lecture de secret, ni modification NAS/Pi. Le rollback, si un nouveau défaut critique le justifie, relève du déployeur et doit préserver données/configuration/NAS.

## Sources de preuve

- `docs/releases/evidence/2026-10-14-three-{dune1,dune2,third720,dune_metrics}.log`
- `docs/releases/evidence/2026-10-14-pair_{dune1,dune2,metrics}.log`
- `docs/releases/REL-PI-001-media-evidence.md`
