# Dépendances et avis de sécurité

**Français** | [English](DEPENDENCIES.en.md)

## GHSA-wrw7-89jp-8q8g — glib,21octobre2026

Avis réel de niveau **moyen**, alerte Dependabot1 laissée ouverte. `glib0.18.5`
reste dans le lockfile parce que Tauri/GTK3 contraignent la série0.18. Forcer
glib0.20 ne satisfait pas ces contraintes et ne constitue pas une correction.
[Avis officiel](https://github.com/advisories/GHSA-wrw7-89jp-8q8g),
[contrainte Tauri amont vérifiée](https://github.com/tauri-apps/tauri/blob/0007a43ae74551655357aced98a000965b759ff3/crates/tauri/Cargo.toml).

L'analyse indépendante prouve l'absence de glib des graphes réellement livrés :
client Mac Apple Silicon et serveur Linux ARM64. Le client lourd Linux n'est pas
un produit distribué ; sa future livraison reste bloquée tant que la chaîne GTK
ne fournit pas une version corrigée. Web/thèmes n'embarquent aucun binaire Rust.

Améliorations concrètes, sans changement de fonctionnement ou lockfile forcé :

- Dependabot vérifie chaque semaine Cargo et les actions GitHub ; aucune liste
  d'exclusion de glib, aucun auto-merge.
- Le garde-fou `scripts/security-shipping-dependencies.mjs` résout avec
  `--locked` les deux graphes précis, hors dépendances de test. Il échoue si
  la version vulnérable devient une dépendance compilée de l'un des paquets.
- Le workflow en lecture seule exécute tests et garde-fou sur les PR, main et
  chaque semaine. Actions officielles épinglées par SHA, pas de secrets,
  d'installation utilisateur ou de déploiement.
- CodeQL et les alertes Dependabot restent les contrôles généraux. Ce garde
  ciblé n'est pas un scanner exhaustif d'avis futurs et ne masque aucune alerte.

Responsable : mainteneur du projet. Réévaluation à la prochaine proposition
Dependabot/Tauri et obligatoirement avant tout client lourd Linux. Ne pas
présenter cet avis comme corrigé ou comme une vulnérabilité du serveur Pi.
