# BUG-0150 — Récupération concurrente avec le minuteur

23 novembre 2026 · Déploiement ARM64 du serveur 0.7.10 sur le Pi.

Le helper local a vérifié le manifeste signé de séquence 11, sauvegardé les
données puis démarré le binaire 0.7.10. Le journal du service de mise à jour
a ensuite enregistré `RolledBack`, cinq secondes après le démarrage. Le
processus actif correspondait au SHA-256 du lot signé. Son exécutable apparaissait
supprimé dans `/proc` et le lien `server/current` désignait le slot 0.7.9-10.
Le registre anti-retour signalait la séquence 11 comme validée.

Le minuteur a lancé une récupération pendant la fin de l’installation locale.
Le serveur 0.7.11 livre le même correctif produit avec la séquence signée 12.
L’installation locale se fait pendant une suspension temporaire du minuteur.
Le contrôle compare les octets du processus et du slot de démarrage au lot signé,
puis vérifie de nouveau le slot après un cycle du minuteur réactivé.
