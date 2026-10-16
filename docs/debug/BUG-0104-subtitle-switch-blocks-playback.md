# BUG-0104 — sélection d’un sous-titre et interruption de lecture

16 octobre 2026, auteur `/root`, base `0c49a8c`, branche `codex/playback-locale`.
Statut : FIXED / VERIFIED_LOCAL, essai humain avec le NAS restant.

## Symptôme et mécanisme

L’utilisateur signale des pauses au changement de sous-titre. La sélection manuelle
d’un fichier externe attendait `download_subtitle` dans `PlayerHost`, sous le mutex
utilisé par les ticks, statuts et renouvellements. Un téléchargement lent peut donc
retarder le renouvellement jusqu’à la garde native d’expiration. Aucun ralentissement
NAS réel n’est reproduit par le banc local ; le chemin bloquant est confirmé dans le code.

## Correction

Téléchargement optionnel détaché, une seule tâche par session. Le tick consomme
uniquement un résultat terminé. Un choix plus récent, une piste intégrée, la fermeture
ou un transfert annulent l’ancienne tâche. Ni pause, ni rechargement du média, ni
vidage du tampon. Les limites de taille/cache et le transport TLS existants restent.
La piste précédente peut rester affichée pendant l’attente ; aucune dépendance vidéo
au chargement du nouveau sous-titre.

## Preuves

- `ui/connected/i18n.test.js` : contrat de tâche détachée, annulation avant réutilisation
  du cache, absence de pause/reload dans le chemin de sélection.
- `cargo test --release --locked -p chill-player subtitle_changes_keep_the_current_playing_or_paused_state` :
  vrai libmpv, vidéo et sous-titres synthétiques locaux ; progression pendant trois
  changements de piste et un ajout externe, pause intentionnelle conservée : PASS.
- Vérifier humainement le téléchargement d’un sous-titre lent depuis le serveur réel.

Rollback : ancien bundle conservé. Aucun serveur/API/SQL/web/NAS modifié.
