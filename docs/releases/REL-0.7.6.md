# 0.7.6 — Mise à jour Mac au clic / One-click Mac updates

Publication Mac achevée le 8 novembre 2026 : `mac-v0.7.6`, Mac 0.7.6/build16,
séquence 7, contient 10 assets. La release globale `v0.7.6` reste Latest et
contient 13 assets ; les deux releases totalisent 23 assets, dont 15 uniques
vérifiés publiquement. Le
serveur0.7.4 et le web0.7.5 étaient inchangés lors de cette publication Mac.
Le correctif serveur0.7.5 a ensuite été publié séparément ; thèmes0.7.1 inchangés. API et schéma inchangés. L'installation sur le Pi est une opération
distincte et n'est pas attestée par cette note.

## Français

La vérification Mac démarre en arrière-plan à chaque ouverture. Le formulaire
d'horaire disparaît au profit de la version, Vérifier maintenant et Installer.
Une petite icône près de Configuration permet d'installer et relancer en un
clic, sans second dialogue. Le catalogue reste utilisable pendant les contrôles.
Les erreurs restent visibles et les clics multiples ne lancent qu'une opération.

Les vérifications interrompues par un redémarrage sont récupérées sous verrou
OS. Le premier contrôle de chaque processus recharge le manifeste signé, même
si un contrôle précédent est récent. Un rechargement de l'interface respecte
l'installation déjà active côté Rust. Signatures, expiration, compteur anti-retour
arrière, reçus exacts et gardes de lecture restent appliqués.

La nouvelle version peut devoir être installée une première fois via le DMG
si l'ancien client est déjà bloqué sur une vérification orpheline. Signature
Mac ad hoc, sans notarisation Apple ; Apple Silicon et macOS 15 minimum.

## English

Mac checks for updates in the background at every launch. The schedule form is
replaced by the installed version, Check now and Install. A small icon beside
Settings installs and restarts in one click, without a second confirmation.
Browsing remains usable during checks; errors persist and repeated clicks share
one operation.

Checks abandoned across restarts recover under an OS lease. The first check in
each process reloads the signed manifest even after a recent prior check. A
reloaded WebView honors installation activity owned by Rust. Signature, expiry,
anti-rollback, exact receipt and playback safeguards remain enforced.

An older client stuck on an orphan check may need this first installation from
the DMG. Ad hoc Mac signature, without Apple notarization; Apple Silicon,
macOS 15 or newer. Server/web/theme versions remain unchanged.

## Preuves / Evidence

Source du paquet Mac et des tags : `091785a7d20aad8162f886484bc634001037ca32`.
Source fonctionnelle approuvée `a88d94c13a8690a4c5136be45434a570bebca250`.
[Cause et régression](../debug/BUG-0138-mac-update-orphan-check.md),
[suivi des tests](../implementation/MAC-UPDATER-ONE-CLICK.md).
43 tests Mac, 369 JavaScript, parcours Chromium, Clippy strict et formatage PASS.
Trois parcours opt-in réussis : vraie vérification GitHub, paquet altéré refusé
puis remplacement signé réel et lancement, API app.restart via LaunchServices
avec nouveau PID et code de sortie 0. Le SDK natif est passé de 0.7.5 à 0.7.6 ;
le paquet altéré a été refusé, puis le paquet signé et `AppHandle.restart()` ont
été validés. L'installation locale conserve les sauvegardes privées du bundle et
des données d'application. Le contrôle GUI du Mac installé affiche 0.7.6,
la carte simplifiée sans horaire et « Vous utilisez la dernière version » au
démarrage puis après un clic sur Vérifier maintenant.

Worktree et caches existants réutilisés ; nouveau dossier de livraison
`/private/tmp/chillcenter-playback-wake.BEMWYx/release-0.7.6` réservé aux artefacts
immuables, sources et preuves de cette version. Anciennes livraisons conservées.
