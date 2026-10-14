# ADR-0010 — Tampon client : une minute d'avance, retour d'environ 30 secondes

- Statut : PROPOSED, détail de l'ADR-0004, aucun remplacement de celui-ci.
- Date : 2026-10-14 ; auteur /root ; propriétaire lecteur Mac.
- Base : c58da7e614b54890e0ef0207f4679d9b02cfca0e, branche codex/pi-install.
- Revue indépendante : PENDING, SHA exact dans la preuve de livraison du delta.
- Supersedes / superseded_by : NONE / NONE.
- Exigences : F-02/03/04/09, ADR-0003/0004, ENABLER-006.

## Contexte et décision

Demande explicite de l'utilisateur : « vise plutôt 1 minute d'avance. et 30 secondes de back ». Le lecteur est libmpv/FFmpeg sur Mac, alimenté exclusivement par le transport Rust HTTPS et les baux ; le Pi ne transcode pas et le Mac n'accède pas au NAS.

L'ancien réglage était20 s,48 Mio avant/16 Mio arrière. Le schéma personnalisé `chill-media` n'est pas une garantie d'activation par l'heuristique `cache=auto`. Décision : `cache=yes`, `cache-secs=60`, `demuxer-readahead-secs=60`, avant plafonné à128 Mio, arrière à64 Mio, `demuxer-donate-buffer=no`. Aucun cache disque ni nouvelle dépendance. Les plafonds portent sur les paquets compressés, pas sur la mémoire totale du Mac (décodeurs, rendu, webview, transport).

Le manuel primaire [libmpv/mpv](https://mpv.io/manual/stable/#cache) consulté le14octobre2026 précise que l'avance est limitée par le premier plafond temps/octets atteint et que l'arrière est limité uniquement en octets. Il n'existe pas de réglage natif de durée arrière. À18 Mbit/s,60/30 s représentent128,75/64,37 Mio : les plafonds128/64 couvrent environ cette cible. À débits supérieurs ou avec variations fortes, les durées diminuent ; à faible débit, l'arrière peut dépasser30 s. **Ce n'est pas une garantie de30 s exactes ni de60 s dès le lancement.** La cible se remplit progressivement si le réseau fournit plus vite que la lecture ; un débit durablement insuffisant ne se corrige pas par un tampon.

## Options et conséquences

- Garder20 s/64 Mio : rejeté, ne satisfait pas la nouvelle demande.
- Tampon illimité : rejeté, consommation client imprévisible.
- Limite temporelle arrière exacte calculée dynamiquement : non retenue pour ce delta ; le débit moyen ne garantirait pas la durée avec VBR, GOP et pistes multiples. L'objectif exprimé est une cible, pas une purge temporelle stricte.
- Cache compressé borné128/64 : retenu, simple et réversible ; budget nominal192 Mio par lecteur, overhead libmpv possible. Charge réseau de prélecture plus importante au remplissage, sans déplacement de mémoire vers le Pi.

Le cache ne prolonge ni le bail30 s ni son échéance locale. Les tests de révocation, pause indépendante du polling et ancien grant refusé restent obligatoires : les octets déjà reçus ne sont pas une autorisation de continuer après expiration. La confiance TLS, les protocoles autorisés et les références de conteneurs bloquées restent inchangés.

## Preuves, migration et rollback

Test réel de création libmpv : options effectives cache/60/128/64/no. Suite chill-player, régression de baux/transport, build Mac autonome. La sonde lit `demuxer-cache-duration` pour distinguer l'avance **mesurée** de la cible ; absence de valeur n'est pas considérée comme60 s. Essai NAS : Dune2021MP4 et Dune2024MKV, dimensions1920×768/1920×800 vérifiées par FFprobe client HTTPS strict, plus un troisième MKV1280×720. Résultats à publier après exécution, pas inférés du build.

Aucune migration de données, aucun redéploiement Pi nécessaire (serveur source126da0a conservé). Livrer un nouveau bundle Mac sans écraser l'ancien126da0a. Rollback : fermer le nouveau lecteur, rouvrir cet ancien bundle ; aucun changement NAS, profil ou TLS à restaurer.

Validation utilisateur de la cible temporelle : message cité ci-dessus. Détail mémoire communiqué dans le chat ; pas de nouvelle exception de sécurité ni d'approbation de revue revendiquée avant son rapport exact.
