# ADR-0048 — Continuité de lecture des séries

23 novembre 2026 · Demande de continuité commune aux clients web et macOS.

**Lire** affiche l’épisode ciblé sur la fiche série. Les cartes de l’Accueil
et du catalogue démarrent la lecture sans ouvrir la fiche. La sélection reprend
un épisode disponible commencé et encore non vu. À défaut, elle propose le
premier épisode disponible et non vu après le dernier épisode vu, puis cherche
au début de la série. Une série entièrement vue peut être relancée à son
premier épisode disponible.

À la première lecture effective d’un épisode en ligne, le serveur vérifie le
prédécesseur immédiat selon l’ordre de la série. Il le marque comme vu pour le
profil actif si sa position mesurée dépasse strictement 70 % de sa durée. La
progression du nouvel épisode et le changement d’état sont enregistrés dans la
même transaction. Les historiques de position et de visionnage sont conservés.
