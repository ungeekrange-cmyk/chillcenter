# ADR-0010 — Tampon client : une minute d'avance, retour d'environ 30 secondes

- Statut : ACCEPTED après revue indépendante624f244, détail de l'ADR-0004, aucun remplacement de celui-ci.
- Date : 2026-10-14 ; auteur /root ; propriétaire lecteur Mac.
- Base : c58da7e614b54890e0ef0207f4679d9b02cfca0e, branche codex/pi-install.
- Révision : CC-CACHE-002 ; revue indépendante/root/pi_integration_gate APPROVED au SHA624f2447b5c0ede6b4a053ea3fa7b82db85c0121 le2026-10-14T12:27:05Z. [Rapport](../../reviews/chillcenter-cache-code-test-624f244.md) SHA256bbbe5cc009515c094e645792b620b9f1d1db97cbed0729ccab931fcfbc8bd19c ; [gate](../../gates/G-DEV-CC-CACHE-002.md). Ce changement de statut ne réécrit pas la décision revue.
- Revue initiale27bd5b2 : code/tests APPROVED, ADR CHANGES_REQUESTED ; rapport conservé dans la preuve de livraison.
- Supersedes / superseded_by : NONE / NONE.
- Exigences : F-02/03/04/09, ADR-0003/0004, ENABLER-006.

## Contexte et décision

Demande explicite de l'utilisateur : « vise plutôt 1 minute d'avance. et 30 secondes de back ». Le lecteur est libmpv/FFmpeg sur Mac, alimenté exclusivement par le transport Rust HTTPS et les baux ; le Pi ne transcode pas et le Mac n'accède pas au NAS.

L'ancien réglage était20 s,48 Mio avant/16 Mio arrière. Le schéma personnalisé `chill-media` n'est pas une garantie d'activation par l'heuristique `cache=auto`. Décision : `cache=yes`, `cache-secs=60`, `demuxer-readahead-secs=60`, avant plafonné à128 Mio, arrière à64 Mio par défaut, `demuxer-donate-buffer=no`. Aucun cache disque ni nouvelle dépendance. Les plafonds portent sur les paquets compressés, pas sur la mémoire totale du Mac (décodeurs, rendu, webview, transport). libmpv peut légèrement les dépasser pour des raisons techniques ; ce ne sont pas des limites RSS strictes.

Ajout utilisateur : « met une option de configuration du tampon en Mio dans le client ». Deux champs dans Administration → Tampon vidéo de ce Mac : avant16–512 Mio, arrière0–256 Mio (0désactive l'arrière), valeurs entières seulement, défaut128/64. Réglage du Mac dans client.json privé/atomique, jamais préférence de profil/serveur. Validation Rust à l'entrée UI, au chargement des réglages et dans Player ; maintien des valeurs en mémoire après échec de sauvegarde. Les anciens fichiers sans ce champ utilisent les défauts. Nouveau lancement/reload natif reçoit les valeurs ; sauvegarder ne modifie pas une lecture déjà active. Une session conserve ses valeurs lors d'une reprise/enchaînement. Pas d'URL, chemin ou option mpv arbitraire dans ce nouveau contrat.

Cette décision **remplace uniquement le budget client64 Mio de ARCHITECTURE.md§10 et de la matrice de vérification mémoire client** par défaut192 Mio et maximum configurable768 Mio de paquets (plus overhead). Elle complète ADR-0004 sans remplacer son moteur/transport. Les preuves antérieures de mémoire client sont STALE pour ce budget ; les preuves serveur/Pi restent valides si leur source/cible n'ont pas changé. Le test RSS total doit être refait, aucune réussite ancien64 Mio n'est transférée au nouveau réglage.

Le manuel primaire [libmpv/mpv](https://mpv.io/manual/stable/#cache) consulté le14octobre2026 précise que l'avance est limitée par le premier plafond temps/octets atteint et que l'arrière est limité uniquement en octets. Il n'existe pas de réglage natif de durée arrière. À18 Mbit/s,60/30 s représentent128,75/64,37 Mio : les plafonds128/64 couvrent environ cette cible. À débits supérieurs ou avec variations fortes, les durées diminuent ; à faible débit, l'arrière peut dépasser30 s. **Ce n'est pas une garantie de30 s exactes ni de60 s dès le lancement.** La cible se remplit progressivement si le réseau fournit plus vite que la lecture ; un débit durablement insuffisant ne se corrige pas par un tampon.

## Options et conséquences

- Garder20 s/64 Mio : rejeté, ne satisfait pas la nouvelle demande.
- Tampon illimité : rejeté, consommation client imprévisible.
- Limite temporelle arrière exacte calculée dynamiquement : non retenue pour ce delta ; le débit moyen ne garantirait pas la durée avec VBR, GOP et pistes multiples. L'objectif exprimé est une cible, pas une purge temporelle stricte.
- Cache compressé borné128/64 : retenu, simple et réversible ; budget nominal192 Mio par lecteur, overhead libmpv possible. Charge réseau de prélecture plus importante au remplissage, sans déplacement de mémoire vers le Pi.

Le cache ne prolonge ni le bail30 s ni son échéance locale. Les tests de révocation, pause indépendante du polling et ancien grant refusé restent obligatoires : les octets déjà reçus ne sont pas une autorisation de continuer après expiration. La confiance TLS, les protocoles autorisés et les références de conteneurs bloquées restent inchangés.

## Preuves, migration et rollback

Test réel de création libmpv : options effectives cache/60/128/64/no, puis valeurs non par défaut96/32 et changement48/0. Suite chill-player/client/desktop, persistance entre ouvertures et indépendance de deux dossiers client, refus bornes/négatif/flottant/champ inconnu et rollback sauvegarde échouée, tests DOM du vrai contrôleur et build Mac autonome. La sonde lit `demuxer-cache-duration` comme **estimation indicative très peu fiable selon mpv**, pas mesure exacte ; absence de valeur n'est jamais considérée comme60 s. L'oracle de lecture réelle combine progression/temps mural/buffering, cette estimation, RSS et seek arrière ; la propriété seule ne prouve ni60 s disponibles ni30 s en arrière sans refetch. Essai NAS : Dune2021MP4 et Dune2024MKV, dimensions1920×768/1920×800 vérifiées par FFprobe client HTTPS strict, plus un troisième MKV1280×720. Résultats à publier après exécution, pas inférés du build. Seek arrière sans refetch et tests humains restent NOT_RUN tant que non exécutés.

Aucune migration de données, aucun redéploiement Pi nécessaire (serveur source126da0a conservé). Livrer un nouveau bundle Mac sans écraser l'ancien126da0a. Rollback : fermer le nouveau lecteur, rouvrir cet ancien bundle ; aucun changement NAS, profil ou TLS à restaurer.

Validation utilisateur des changements : deux messages cités ci-dessus. User gate supplémentaire NOT_APPLICABLE pour ce détail local borné explicitement demandé ; pas de nouvelle exception de sécurité ni d'approbation de revue revendiquée avant son rapport exact. Gate de revue du nouveauSHA requis, sans étendre automatiquement la revue27bd5b2. Aucun runtime/lock/dependance mis à jour dans ce delta.
