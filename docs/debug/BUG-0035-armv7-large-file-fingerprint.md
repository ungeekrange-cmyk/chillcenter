# BUG-0035 — Conversion de longueur avant borne sur ARMv7

2026-10-14, root + audit lecture seule admin_contract_fix àf9838f0. Sur usize32, 4GiB exacts se convertissent en0. read_chunk bornait après cette conversion et le span partiel faisait pareil : échantillon vide, boucle full-hash sans progression pour certaines tailles >4GiB. Reproduction arithmétique exacte u64→u32 ; **pas un build/test ARM exécuté**.

Correction : min en u64 avant la conversion (borne64KiB), soustraction vérifiée des offsets, span min(size,64KiB) en u64. Régression pour4GiB,4GiB+17 etu64::MAX ; petit reste17. Audit des autres casts streaming/archive/subtitles : ils sont précédés de bornes ≤1GiB ou convertis après borne64KiB, aucun autre défaut confirmé.

Test hôte `lengths_are_bounded_before_conversion_for_armv7_large_files` PASS. Pi/ARMv7 réel NOT_RUN. Aucun fichier de4GiB, vidéo du foyer ou transcodage créé pour cette preuve.
