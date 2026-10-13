# Couverture fonctionnelle et preuves

Révision CC-STORIES-001 ; source CC-FONC-002 et CC-DOC-001 ; auteur /root. Tous les IDs ci-dessous renvoient aux entrées acceptées, pas à de nouveaux choix de produit.

| Exigence | Stories fonctionnelles | Travaux techniques |
| --- | --- | --- |
| F-01 | Cible technique ci-contre | [ENABLER-002](../work-items/ENABLER-002.md), [ENABLER-006](../work-items/ENABLER-006.md), [ENABLER-007](../work-items/ENABLER-007.md) |
| F-02 | [US-001](US-001/story.md), [US-016](US-016/story.md) | [ENABLER-001](../work-items/ENABLER-001.md), [ENABLER-003](../work-items/ENABLER-003.md), [ENABLER-007](../work-items/ENABLER-007.md) |
| F-03 | [US-001](US-001/story.md), [US-006](US-006/story.md), [US-016](US-016/story.md) | [ENABLER-001](../work-items/ENABLER-001.md), [ENABLER-002](../work-items/ENABLER-002.md), [ENABLER-003](../work-items/ENABLER-003.md), [ENABLER-006](../work-items/ENABLER-006.md), [ENABLER-007](../work-items/ENABLER-007.md) |
| F-04 | [US-023](US-023/story.md) | [ENABLER-006](../work-items/ENABLER-006.md), [ENABLER-007](../work-items/ENABLER-007.md) |
| F-05 | [US-002](US-002/story.md), [US-003](US-003/story.md), [US-004](US-004/story.md) | [ENABLER-004](../work-items/ENABLER-004.md) |
| F-06 | [US-003](US-003/story.md), [US-004](US-004/story.md), [US-007](US-007/story.md), [US-013](US-013/story.md), [US-014](US-014/story.md), [US-015](US-015/story.md), [US-018](US-018/story.md), [US-020](US-020/story.md), [US-021](US-021/story.md), [US-022](US-022/story.md), [US-023](US-023/story.md), [US-025](US-025/story.md), [US-026](US-026/story.md), [US-030](US-030/story.md) | [ENABLER-004](../work-items/ENABLER-004.md), [ENABLER-005](../work-items/ENABLER-005.md) |
| F-07 | [US-021](US-021/story.md), [US-023](US-023/story.md) | [ENABLER-005](../work-items/ENABLER-005.md) |
| F-08 | [US-022](US-022/story.md) | [ENABLER-005](../work-items/ENABLER-005.md) |
| F-09 | [US-016](US-016/story.md), [US-017](US-017/story.md), [US-019](US-019/story.md), [US-020](US-020/story.md) | [ENABLER-001](../work-items/ENABLER-001.md) |
| F-10 | [US-018](US-018/story.md), [US-019](US-019/story.md), [US-020](US-020/story.md) | — |
| F-11 | [US-018](US-018/story.md), [US-019](US-019/story.md) | — |
| F-12 | [US-008](US-008/story.md), [US-011](US-011/story.md), [US-015](US-015/story.md), [US-024](US-024/story.md), [US-025](US-025/story.md) | [ENABLER-005](../work-items/ENABLER-005.md) |
| F-13 | [US-025](US-025/story.md) | [ENABLER-005](../work-items/ENABLER-005.md) |
| F-14 | [US-006](US-006/story.md), [US-007](US-007/story.md), [US-008](US-008/story.md), [US-009](US-009/story.md), [US-010](US-010/story.md), [US-011](US-011/story.md), [US-012](US-012/story.md), [US-013](US-013/story.md), [US-014](US-014/story.md), [US-015](US-015/story.md), [US-022](US-022/story.md), [US-030](US-030/story.md) | [ENABLER-004](../work-items/ENABLER-004.md) |
| F-15 | [US-030](US-030/story.md) | — |
| F-16 | [US-006](US-006/story.md), [US-012](US-012/story.md), [US-026](US-026/story.md), [US-027](US-027/story.md), [US-028](US-028/story.md), [US-029](US-029/story.md) | [ENABLER-004](../work-items/ENABLER-004.md), [MIG-001](../work-items/MIG-001.md), [MIG-002](../work-items/MIG-002.md) |
| F-17 | [US-027](US-027/story.md), [US-028](US-028/story.md) | [ENABLER-004](../work-items/ENABLER-004.md), [MIG-001](../work-items/MIG-001.md) |
| F-18 | [US-001](US-001/story.md), [US-002](US-002/story.md), [US-004](US-004/story.md), [US-006](US-006/story.md), [US-027](US-027/story.md), [US-028](US-028/story.md), [US-029](US-029/story.md) | [ENABLER-002](../work-items/ENABLER-002.md), [MIG-002](../work-items/MIG-002.md) |
| F-19 | [US-001](US-001/story.md), [US-005](US-005/story.md) | [ENABLER-001](../work-items/ENABLER-001.md), [ENABLER-002](../work-items/ENABLER-002.md), [ENABLER-003](../work-items/ENABLER-003.md) |
| F-20 | [US-031](US-031/story.md), [US-032](US-032/story.md), [US-033](US-033/story.md) | — |
| F-21 | [US-032](US-032/story.md), [US-033](US-033/story.md) | — |
| F-22 | [US-016](US-016/story.md), [US-019](US-019/story.md) | [ENABLER-001](../work-items/ENABLER-001.md) |

Les AC de chaque fichier reprennent les F concernés ; les contraintes serveur Linux/Rust/ressources sont tracées aux enablers, plutôt qu'à de fausses stories « créer un serveur ».

| Vérification approuvée | Stories / work-items à exercer |
| --- | --- |
| V-01 Linux ARM64 | ENABLER-002/006/007 |
| V-02 trois flux et ressources | US-016/017/019/023 ; ENABLER-006/007 |
| V-03 lecteur, formats, FFI | US-016–020 ; ENABLER-001/003/007 |
| V-04 transferts | US-003/004/021/023/025 ; ENABLER-005/007 |
| V-05 statut vu | US-022 ; ENABLER-005/007 |
| V-06 suite | US-024/025 ; ENABLER-005/007 |
| V-07 import/export | US-027/028 ; MIG-001 ; ENABLER-007 |
| V-08 déplacements/indisponibles | US-006/012/026 ; MIG-001 ; ENABLER-004/007 |
| V-09 TLS | US-001/005/019 ; ENABLER-002/003/007 |
| V-10 personnel/concurrence/IA | US-002–004/009/013–015/018/020–023/030 ; ENABLER-004/005/007 |
| V-11 apparence/accessibilité | US-031–033 et contrôles lecteur ; ENABLER-007 |
| V-12 sauvegarde/restart | US-021/023/029 ; MIG-002 ; ENABLER-002/005/007 |

Aucun AC exécuté à cette étape. Le ledger futur indiquera pour chaque AC environnement, commande, SHA et résultat, sans substituer « tests passent » à une trace explicite.

