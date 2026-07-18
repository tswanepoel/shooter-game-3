# Feature 011 - Blaster lineup

Developers need a debug **blaster lineup**: a row of Kenney blasters, each held by a blocky character in a right-hand hold pose, so import scale, facing, paint, and **hand–grip attachment** can be checked together against the metre grid. This evolves the character-only lineup (008) into an armed composition check. Presentation is client-only.

**Pairing:** same letter — `blaster-{letter}` held by `character-{letter}` (stable across loads).

## Acceptance criteria

- Gated with the debug tools surface (003). Dev builds own the implementation; release builds strip debug tools. Show/hide goes through the command and cvar registry (and host bridge). The existing lineup entry point remains the toggle; root README lists one line with other debug tools; model facts live in the kit READMEs.
- **Row subject is the blaster kit.** Slots are `blaster-a` … `blaster-r` in letter order. Each slot shows one blaster at a character’s right-hand grip. Characters complete the grip check.
- **Hold pose:** each character uses the kit clip **`holding-right`** on **`arm-right`** (rigid part TRS). The rest of the body stays at bind pose for this presentation.
- **Attachment contract** (authoritative detail in the blaster kit README):
  - **Grip point:** image of the per-blaster offset under `arm-right` after `holding-right` (offsets in character-kit / arm-local units)
  - **Orientation:** in character space (face +Z, up +Y), **180° about Y** so mesh muzzle (−Z) aims character-forward (+Z) with top up
  - **Scale:** character body **×1/1.5** kit→metres; blaster mesh **×1** authored→metres (relative **×1.5** on the gun when the position chain already carries the character scale)
- **Cook / load (010):** character and blaster assets for this row ship in the Kenney core pack (or same-cadence sibling). Loaders use pack and asset ids.
- **World placement (002 / 008):** characters stand with **feet on y = 0**, face **+Z**, character root scale as in the character kit README. Row spacing leaves room for held weapons.
- **Presentation:** flat albedo path used for characters (008 / 009)—unlit-style colour, atlas wrap, mips as for similar albedos. Blaster materials follow the blaster kit README.
- Feature 008 character load and paint rules remain the body path; this feature adds blasters, hold pose, and the grip contract above.

## Kit documentation

- Characters: `assets/source/characters/README.md`
- Blasters and grip table: `assets/source/blasters/README.md`
