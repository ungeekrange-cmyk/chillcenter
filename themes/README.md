# Theme packs / Packs de thèmes

**Français** | [English](README.en.md)

Statut : pack embarqué déclaratif `themes/default` (format 1, rendu 2, version
0.7.1). Le chargeur valide ce manifeste avant toute application et replie vers
la copie embarquée si le pack externe est indisponible ou invalide.
Le contrat de livraison indépendante est décrit dans
[ADR-0029](../docs/architecture/decisions/ADR-0029-component-releases-updates.md).
Le moteur 2 accepte toujours les anciens packs de rendu 1. Les nouveaux packs
de rendu 2 sont refusés par un ancien client, avant installation.

## Calendrier et priorité

Dates inclusives, réévaluées à minuit selon le fuseau de l'appareil client,
hémisphère Nord. Le fuseau du calendrier visuel est distinct de celui du timer
de mise à jour du serveur.

| Ambiance | Début inclus | Dernier jour inclus | Priorité automatique |
| --- | --- | --- | --- |
| Printemps | 1 mars | 31 mai | Saison |
| Été | 1 juin | 31 août | Saison |
| Automne | 1 septembre | 30 novembre | Saison |
| Hiver | 1 décembre | Dernier jour de février | Saison |
| Halloween | 1 octobre | 31 octobre | Événement, remplace la saison |
| Noël | 25 novembre | 25 décembre | Événement, remplace la saison |
| Saint-Valentin | 31 janvier (J−14) | 14 février | Événement, priorité 200 |
| Pâques occidentales | J−30, recalculé chaque année | Dimanche de Pâques | Événement, priorité 200 |

Pâques est calculé selon le calendrier grégorien, pas fixé à une date du mois.
Exemple 2027 : du 26 février au 28 mars inclus, retour au printemps le 29 mars.
Les saisons ont la priorité 0 ; Noël/Halloween la priorité 100. Une sélection
manuelle ou les ambiances désactivées restent prioritaires sur ces règles.

Ces fenêtres conservent le calendrier déjà approuvé dans F-21. Le lendemain de
l'événement, retour à la saison correspondante. « Désactivée » retire les
ambiances sans modifier le mode clair/sombre ; le choix manuel persistant prend
le pas sur le calendrier. Si de futurs événements se chevauchent, leur priorité
numérique explicite tranche ; une égalité ambiguë rend le pack invalide.

Chaque ambiance fournit les variantes claire et sombre. Le mode système choisit
la variante ; il ne change pas la priorité. Aucun décor ne doit couvrir le
lecteur. Les effets respectent réduction du mouvement, visibilité et petits
écrans ; le sujet des images événementielles reste ancré à droite.

## Format chargé

Un pack versionné contiendra un manifeste JSON déclaratif (version du pack,
version du format/rendu compatible, identifiants, dates, priorités, palettes,
ressources et leurs SHA-256), images et logos. Aucun JavaScript, HTML, chemin
absolu, URL distante, CSS libre ou script d'installation dans le pack.
Les SVG historiques restent dans `ui/connected/branding`; le pack ne contient
que leurs exports PNG approuvés, contrôlés par taille et SHA-256.
Les animations et le moteur de rendu restent dans l'application : dix pétales
CSS et deux silhouettes de lapins au maximum, sans boucle JavaScript par image.
Le déplacement est rigide ; les proportions du lapin approuvé sont conservées.
Le logo de printemps est réutilisé pour Pâques, celui par défaut pour Saint-Valentin.
Les effets s'arrêtent en arrière-plan, pendant une lecture et en mouvement réduit.
Le décor disparaît du panneau vidéo ; aucun transcodage ou effet côté serveur.

La release GitHub `themes-v<version>` livrera le pack signé indépendamment des
binaires. Après vérification, installation dans un nouveau dossier puis bascule
atomique ; pack précédent conservé et pack embarqué utilisé en repli. Un format
incompatible est refusé sans dégrader l'affichage. Mac et serveur web conservent
leurs propres copies installées ; mettre à jour les thèmes n'exige pas de
reconstruire le client ou le binaire serveur. Un nouveau type d'effet hors du
contrat de rendu pourra en revanche nécessiter une mise à jour applicative.

Clés privées et certificats locaux restent hors Git. Seule une clé publique de
vérification des releases pourra être incluse après approbation explicite.
