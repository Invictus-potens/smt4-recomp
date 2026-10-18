# Overrides

Hand-written C functions that replace generated ones for this title. See
[3dsrecomp's overrides docs](
https://github.com/fearkov/3dsrecomp/blob/main/docs/overrides.md) for the
format. They're compiled in by `3dsrecomp port ... --overrides overrides/`.

- `multipliers.c`: Macca, hero EXP, demon EXP and App Points multipliers driven by the
  cheat menu. See [docs/MULTIPLIERS.md](../docs/MULTIPLIERS.md).
- `recruit.c`: Easy Recruit, the negotiation checks forced the player's
  way. See [docs/RECRUIT.md](../docs/RECRUIT.md).
- `widescreen.c`: the wider projection behind the widescreen cheats. See
  [docs/WIDESCREEN.md](../docs/WIDESCREEN.md).

Code that isn't at a function entry can't be overridden; those spots are
patched in the generated C by `scripts/patch_generated.py` instead.
