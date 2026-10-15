# BUG-0067 — Organiser avec l’IA reste désactivé

2026-10-15, auteur `/root`, investigation complémentaire `/root/investigate_ai_organize`. Capture utilisateur : bouton pâle, non utilisable.

Cause reproductible : `run` mémorise l’état désactivé des contrôles avant une commande. Le rendu du nouveau profil autorise le bouton IA, puis `finally` restaure son ancien état désactivé. L’inverse peut réactiver à tort un contrôle sans profil. Un rafraîchissement normal bloquant rendait aussi tous les boutons indisponibles pendant son attente.

Deux tests RED dans `ui/connected/run-availability.dom.test.js` reproduisent les deux transitions. Correction : rendre les disponibilités sémantiques après restauration des désactivations transitoires, jamais avant. La synchro détachée (BUG-0064) ne passe pas par cette désactivation globale.

Diagnostic sans requête IA réelle, clé ou NAS. Les tests mockés IA existants vérifient les propositions/confirmations, mais ne prouvent ni la présence d’un CLI sur le Mac utilisateur ni son authentification. Disponibilité du fournisseur réel : à vérifier séparément si le lancement de « Proposer » retourne ensuite une erreur.
