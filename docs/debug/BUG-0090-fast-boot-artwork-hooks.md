# BUG-0090 — Cache rapide rendu avant installation du module d'images

16 octobre 2026. **FIXED_LOCAL — RED/GREEN navigateur synthétique.**

Le nouveau test de défilement attendait indéfiniment l'affiche initiale. Diagnostic H1 confirmé :20cartes,0cible d'observation,0appel d'image, fonction d'amélioration présente seulement après leur création. `app.js` pouvait terminer boot/catalogue en microtâches avant l'exécution des scripts `defer` suivants ; l'appel facultatif d'image était alors ignoré.

Correction : attendre DOMContentLoaded lorsque les scripts différés sont encore en cours, puis lancer boot ; pas de délai arbitraire, pas de verrou réseau supplémentaire. Une page déjà complète démarre immédiatement. Les modules sont prêts avant le premier rendu du cache.

`test-catalog-virtual-artwork.cjs` : timeout initial reproductible, puis PASS après le vrai correctif sans forcer de rechargement test. Garde erreurs JS et délai5s par étape. Aucune donnée réelle, aucun accès serveur ou nouvelle lecture vidéo.
