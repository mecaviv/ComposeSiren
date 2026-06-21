# ComposeSiren — correctifs des fichiers data

Travaux de correction de l'artefact de clic au crossfade (sessions du 25 mai
et du 4 juin 2026), copiés depuis le repo ComposeSiren (branche `feat/2plugins`).

## Contenu

- **`fix_data.py`** — script final (4 juin 2026). Non destructif : lit
  `Resources/`, écrit les fichiers corrigés dans `Resources-fixed/`.
  - Correction : réindexation des partiels par harmonique physique +
    fréquences forcées aux harmoniques exactes `f0×(h+1)`.
  - Grille auto-détectée par sirène : `f0/2` pour S1/S3/S4/S5,
    `f0` pour S7 (sinon transposition d'une octave !).
  - `--reduit` : format packé expérimental (603 Mo → ~28 Mo, dataFreq
    supprimé) — nécessite un loader C++ adapté, non lisible par le code actuel.
  - Dépendance : numpy. Usage : `python3 fix_data.py [--dry-run]`
- **`CORRECTIFS_ARTEFACT_CROSSFADE.md`** — doc de la session du 25 mai 2026 :
  cause racine (désordre d'indexation des harmoniques dans les tables),
  script S7 d'origine, corrections de code associées, pistes rejetées.
- **`CORRECTIFS_SAMPLERATE.md`** — doc des correctifs sample rate dynamique
  (remplacement du 44.1 kHz hardcodé).

## État au 4 juin 2026

- S7 validée à l'oreille (25 mai) ; S1–S6 corrigées par `fix_data.py`,
  validation à l'oreille en cours via SirenOrchestra standalone.
- Les fichiers originaux de `Resources/` restent la source de vérité ;
  aucune modification du code C++.
