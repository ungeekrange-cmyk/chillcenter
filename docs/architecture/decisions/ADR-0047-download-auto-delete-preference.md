# ADR-0047 — Réglage global de suppression après visionnage

21 novembre 2026 · Amendement demandé lors de la validation
de [l’architecture hors connexion](ADR-0046-offline-downloads.md).
Le même réglage s’affiche dans **Téléchargements** et dans
**Configuration → Téléchargements**. Cette décision précise la portée du
réglage décrit dans la section « Progression, conflits et suppression »
de l’ADR-0046.

Le réglage **Supprimer après visionnage** est activé au premier usage.
Il est global pour les téléchargements du profil sur le Mac ou dans le
navigateur utilisé. Il est conservé localement pour rester modifiable hors
connexion. La modification est visible aux deux emplacements et, sur le web,
dans les autres onglets du même profil. Configuration reste accessible
quand la rubrique Téléchargements est masquée faute de copie.

Le réglage s’applique aux téléchargements déjà présents et aux suivants.
Un bouton **Conserver cette vidéo** maintient l’exception par copie prévue
dans l’architecture. Lorsque le réglage global est désactivé, toutes les
copies sont conservées. Le réactiver ne lance aucune purge rétrospective
et préserve les exceptions.

Le client relit le réglage et l’exception avant d’effacer. Une désactivation
intervenue avant l’effacement annule la suppression automatique en attente.
Un échec d’enregistrement conserve le réglage précédent et s’affiche à
l’écran. Le déclenchement, la reprise et la suppression restent limités à
la copie locale selon les règles de l’ADR-0046.

[Scénarios d’utilisation](../../stories/OFFLINE-DOWNLOADS.md).
