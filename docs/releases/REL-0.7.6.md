# 0.7.6 — Mise à jour Mac au clic / One-click Mac updates

Candidat en préparation le 8 novembre 2026, à la demande explicite de fusion,
publication macOS et installation locale. Mac 0.7.6/build 16, séquence 7.
Serveur 0.7.4, web 0.7.5 et thèmes 0.7.1 inchangés ; API et schéma inchangés.
L'utilisateur autorise aussi l'installation du serveur et du web publics sur
son Pi. La publication GitHub et ces installations seront vérifiées séparément.

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

Source fonctionnelle approuvée `a88d94c13a8690a4c5136be45434a570bebca250`.
[Cause et régression](../debug/BUG-0138-mac-update-orphan-check.md),
[suivi des tests](../implementation/MAC-UPDATER-ONE-CLICK.md).
43 tests Mac, 369 JavaScript, parcours Chromium, Clippy strict et formatage PASS.
Trois parcours opt-in réussis : vraie vérification GitHub, paquet altéré refusé
puis remplacement signé réel et lancement, API app.restart via LaunchServices
avec nouveau PID et code de sortie 0. Le paquet 0.7.6 final sera testé à nouveau.

Worktree et caches existants réutilisés ; nouveau dossier de livraison
`/private/tmp/chillcenter-playback-wake.BEMWYx/release-0.7.6` réservé aux artefacts
immuables, sources et preuves de cette version. Anciennes livraisons conservées.
