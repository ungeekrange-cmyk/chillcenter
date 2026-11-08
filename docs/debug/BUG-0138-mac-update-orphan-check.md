# BUG0138 — Vérification Mac bloquée après redémarrage

8 novembre 2026. Application installée observée : 0.7.4, build 14 ; candidat
correctif sur `codex/mac-updater-one-click`, base `978a4e9`.

## Preuve et cause

Lecture seule du journal local : dernière opération Mac/check Running,
created_unix = updated_unix = 1794130056, soit 09:27:36 UTC. Le processus Mac
actuel a commencé le 8 novembre à 09:27:52 UTC, après cette opération. L'état
reste identique lors de la seconde lecture après le diagnostic. Aucun fichier de
configuration ou de données de cette installation n'a été modifié par l'agent.

`StateStore::admit_bound` rend une vérification Pending/Running existante sans
en créer une nouvelle. `UpdateHost::check` retourne ensuite l'instantané si
l'opération n'est plus Pending, sans recréer de worker. `UpdateHost::new` ne
récupérait que les transactions d'ambiances. Chaque clic retrouvait donc le
Running orphelin, indéfiniment. Ce mécanisme est reproduit dans le test ciblé.

Hypothèse confirmée dès le premier diagnostic. Les fichiers du paquet Mac
public 0.7.5 passent les contrôles de signature/version signée et de structure
tar : aucun défaut de format identifié. Le délai de 60 secondes du gros
téléchargement était un risque distinct ; il n'explique pas ce Running orphelin.

## Correction

Verrou OS conservé durant chaque vérification locale et installation. Avant une
nouvelle admission, sous ce verrou libre, seules les vérifications locales
Pending/Running abandonnées passent Failed/CHECK_INTERRUPTED ; le délai de
limitation est levé pour autoriser une nouvelle tentative immédiate. Les reçus,
les séquences et les installations ne sont pas réinitialisés. Une autre instance
tenant le verrou interdit toute récupération et garde son état intact.

Au démarrage, le dernier contrôle terminé est également revérifié : sa version
disponible était en mémoire seulement. Les clics suivants gardent le délai
normal de cinq minutes. Le verrou OS est l’autorité ; un booléen mémoire laissé
actif après un panic ne bloque plus une nouvelle tâche.

La vérification Mac démarre en arrière-plan à l'ouverture ; le planning Mac
historique est ignoré. La nouvelle interface et le retrait de la seconde
confirmation répondent à la demande utilisateur : [ADR0039](../architecture/decisions/ADR-0039-mac-one-click-updates.md).

## Vérification et retour arrière

Tests Rust : reproduction de l'orphelin, nouvelle admission après récupération,
conservation de la séquence, refus sans mutation si un worker tient le verrou,
reçus exacts et garde lecture. Tests navigateur : commandes utilisables sans
politique Mac et installation unique au double-clic. La preuve d'exécution
finale figure dans le [suivi du correctif](../implementation/MAC-UPDATER-ONE-CLICK.md).

Aucune migration de schéma, aucune suppression de donnée ni nouvelle clé.
Retour arrière : précédent paquet public conservé. Les anciens clients ne
tiennent pas le verrou de vérification ; fermer une ancienne copie avant
d'ouvrir simultanément une copie de développement utilisant les mêmes réglages.
