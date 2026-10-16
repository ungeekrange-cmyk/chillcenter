# ADR-0022 — Pause conservant le moteur et le tampon

16octobre2026, amendement utilisateur explicite. Auteur `/root`. **Supersedes** : uniquement la libération du bail à chaque pause dans ARCHITECTURE§5 et les preuves qui l'imposaient. Les règles de fermeture, échéance, transfert et isolation des profils restent inchangées ; budget ADR0010 conservé.

Le problème ne se limite pas au dessin de la barre : `PlayerCommand::Pause` libérait le bail, et `Resume` rechargeait le moteur sous un nouveau bail. Le cache libmpv était donc abandonné. L'utilisateur demande désormais une vraie pause du moteur existant, remplissage du tampon maintenu et reprise sans vider le cache.

Décision : conserver moteur/transport/bail pendant une pause locale ; renouveler le bail et enregistrer la position, sans crédit de visionnage pendant la pause. La lecture anticipée peut continuer jusqu'aux limites existantes de durée et de Mio, jamais télécharger sans borne ni relever les plafonds. Reprendre le même moteur n'exige pas un nouveau chargement ; une perte effective de bail ou un transfert reste une nouvelle acquisition sécurisée, pas une réutilisation d'octets sous bail révoqué.

Conséquence visible annoncée : le profil reste occupé pendant la pause, jusqu'à fermeture/fin/échéance ou transfert explicitement confirmé. Un autre profil reste indépendant. Les notifications de transfert et la protection à l'échéance continuent de mettre en pause/retirer l'accès ; aucun transfert forcé ou relâchement TLS. La barre conserve la dernière position pendant une reprise en attente, le bouton affiche immédiatement pause et une roue de chargement ; un échec doit laisser un état cohérent et réessayable.

Validation ciblée exigée : même instance/bail et cache conservé sur pause/reprise, anticipation bornée pendant pause, aucun crédit à l'arrêt, transfert/échéance toujours effectifs, position et icône pendant réponse retardée/erreur. Pas une promesse de reprise immédiate lorsque le tampon était déjà vide ou le NAS inaccessible. Résultats : [lot Configuration](../../implementation/CONFIGURATION-MATCHING-20261016.md).
