# Déploiement web seul — 17 octobre 2026

**Français** | [English](REL-WEB-PARITY-001-web-deployment.en.md)

Autorisation utilisateur explicite : déployer le nouveau client web après
sauvegarde, sans changer le binaire serveur, la configuration ou le NAS.
Livraison de fichiers statiques uniquement, sans arrêt/redémarrage du service.

## Artefact et autorisation technique

Source a13902ea73aa51fb09977f1e6649e252ff39d984, porteur documentaire a0ba4e2.
Archive `artifacts/web/chillcenter-web-test-a13902e.tar`, SHA256
`32176d8fabb110d0889187a7bcbed7ad19ee1d8a25530043fc6a714b2dc885e4`,
build **1cce6dd6cd9d**. Opérateur `scripts/pi-update-web-assets-operator.sh`, SHA256
`b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996`.
[Revue indépendante APPROVED_SCOPED](../reviews/web-assets-deployment-a13902e.md)
avant toute bascule. Ni reconstruction ni modification de l'archive lors de l'installation.

## Exécution et sauvegarde

Déployeur distinct des auteurs/reviewers : `/root/web_assets_deployer`.
Sortie opérateur **0**, le **17 octobre à 12:56:36 Europe/Dublin**
(`2026-10-17T11:56:36Z`). Ancien build 9091c7934eda → **1cce6dd6cd9d**.
PID **5420** conservé, service actif, redémarrages **0** avant/après.

Sauvegarde privée complète créée et vérifiée avant remplacement :
`/root/chillcenter-rollback/predeploy-complete-20261017T115606Z/web-before.tar`,
1 126 400 octets, dossier 0700, SHA256
`050aead4ce44c2f698e4793893bae90e4d5bd56c017e4c007550d31de05e567e`.
Manifeste trié de 60 entrées, 56 fichiers hachés ; les descriptions de membres et
métadonnées sont conservées à côté. Ancien dossier web intégral conservé dans
`/root/chillcenter-rollback/web-20261017T115634Z`.

Échec préparatoire conservé : une première capture de sommes a quitté avec le
code 123 (chemins relatifs résolus depuis le mauvais répertoire), avant création
du TAR, copie entrante ou remplacement. Le dossier partiel
`/root/chillcenter-rollback/predeploy-20261017T115544Z` reste une preuve d'échec,
**pas une sauvegarde**. La seconde capture utilise explicitement le bon cwd.
Il s'agit d'une erreur de préparation d'outillage, pas d'un bug du lecteur/serveur.

## Invariants contrôlés par l'opérateur

Les empreintes relevées avant la bascule puis à 11:56:45Z sont identiques :

| Élément | SHA256 conservé |
| --- | --- |
| Binaire serveur | `6fecabc69258e372d71730a23451f5ff47565f60cbbc352552c12d8ac39424ea` |
| Configuration | `d6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1` |
| Unité systemd | `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2` |
| Clé publique du certificat (SPKI) | `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df` |

Aucun accès aux fichiers vidéo, scan, import de données, nouvelle clé ni écriture
NAS. Contrôles HTTP/HTTPS de l'opérateur PASS, TLS strict avec le certificat
public local, sans désactivation de validation. API 2026-10-14 et alias v1 non modifiés.
Bundle Mac et release/tag GitHub v0.6 conservés ; aucun push ni changement de visibilité.

## Vérification indépendante et limites

**APPROVED_SCOPED**, `/root/web_assets_post_verifier`, distinct du déployeur,
des auteurs et des reviewers, le 17 octobre de 12:58 à 13:01 Europe/Dublin
(11:58–12:01UTC). Aucun retour arrière recommandé.

- 79 fichiers installés, 78/78 sommes valides, couverture exacte et aucun fichier
  spécial ; propriétaire root, sans écriture groupe/autres.
- Sauvegarde et ancien dossier : liste exacte, 56/56 fichiers identiques, ancien
  build 9091c7934eda, TAR et dossier 0700 conformes. Dossier incomplet confirmé
  au chemin corrigé ci-dessus : manifeste seul et fichier de hashes vide.
- Même PID 5420, 0 redémarrage, binaire/configuration/unité/SPKI identiques.
- HTTPS strict : page, build, API 2026-10-14 et v1 PASS ; capacités attendues présentes.
  WASM 200 `application/wasm`, 652 895 octets ; `bridge.js` servi identique au fichier installé.
- HTTP historique 8080 : page, build et deux API PASS. Cross-site 403 et `.env` 404.
- Aucun avertissement/erreur dans les journaux depuis le début du préflight.
  Montage CIFS courant en lecture seule, sans parcours des fichiers du NAS.

Après approbation, seule la copie temporaire de transfert
`/var/tmp/chillcenter-web.tar` a été retirée, après contrôle de son type et de son
SHA exact. L'archive locale d'origine, les sauvegardes et l'ancien dossier web
sont conservés. Build et PID recontrôlés identiques après ce nettoyage ciblé.

L'empreinte du fichier certificat complet et celle de l'état des montages n'ont
pas été relevées avant l'opération : elles ne sont pas revendiquées inchangées.
La comparaison TLS porte sur le SPKI. L'absence d'opération NAS résulte du
périmètre des commandes exécutées, pas d'un audit de tout le contenu du NAS.

Le parcours lecteur, les écritures de progression et les tests NAS ne sont pas
rejoués dans ce déploiement de fichiers web. Les capacités de décodage et
plein écran restent propres au navigateur. L'IA web reste exclue.

## Retour arrière

Sur échec de contrôle pendant la bascule, l'opérateur rétablit automatiquement
l'ancien dossier. Après livraison, conserver l'ancien dossier et le TAR privé :
replacer l'ancien répertoire web par renommage, en gardant le nouveau à part,
sans remplacer le binaire, modifier la configuration ou redémarrer le serveur.
Ne pas écraser l'ancienne sauvegarde ; sérialiser toute restauration avec une
nouvelle opération de déploiement et recontrôler build, PID et empreintes.
