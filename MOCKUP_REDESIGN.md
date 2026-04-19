# Battle UI Redesign — Reference

Companion notes for `mockups/battle-ui.html`. Intended as a checklist for porting
the mockup into Bevy 0.18.

---

## Screen layout

A single **16:9-locked viewport**. In HTML this is done with
`width: min(100vw, 100vh * 16/9)` + `aspect-ratio: 16/9` + `container-type: size`
so everything inside scales by `cqh`/`cqw` units. The Bevy equivalent is to
render all HUD nodes inside a parent `Node` whose size is pinned to a 16:9 box
and to express child sizes in `Val::Percent` (or `Val::VMin`-style custom
scaling) rather than `Val::Px`.

```
┌──────────────────────────────────────────────────────────┐
│  Player portrait + HP  │  PHASE banner  │  HP + portrait │  combat bar  (top, full width)
├──────────────┬─────────────────────────┬─────────────────┤
│ INSCRIBED    │                         │ BOOK OF         │
│ ATTEMPTS     │     BATTLE ARENA        │ ACTING          │
│              │  (backdrop + NPC)       │  4 spells       │
│ ──────────   │                         │                 │
│ RUNE         │                         │                 │
│ KEYBOARD     │                         │                 │
├──────────────┴─────────────────────────┴─────────────────┤
│  BINDING STRAIN  ⛓⛓◐○○  ·  3 / 5 hold                   │  binding bar (bottom, full width)
└──────────────────────────────────────────────────────────┘
```

Three-column middle row (22fr / 50fr / 22fr). Left column is a vertical flex
of **Inscribed Attempts** (flex:1) and **Rune Keyboard** (flex:auto).

---

## Visual language

### Palette (CSS variables in `:root`)

| Name                 | Hex       | Role                               |
| -------------------- | --------- | ---------------------------------- |
| `--parchment`        | `#ead9b4` | Aged paper ground for the book     |
| `--parchment-warm`   | `#f0e1bf` | Page highlight                     |
| `--parchment-shadow` | `#c9b383` | Page shadow                        |
| `--parchment-dark`   | `#a88f5f` | Muted body text on dark ground     |
| `--ink`              | `#231510` | Deep text on parchment             |
| `--gold`             | `#c9a24b` | Primary frames, pips, trim         |
| `--gold-dark`        | `#8b6d2a` | Recessed gold                      |
| `--gold-light`       | `#f0d48a` | Headings, glints                   |
| `--blood`            | `#8b1e2e` | Enemy HP, active attempt border    |
| `--blood-bright`     | `#c13546` | Hot accents, broken chain          |
| `--ember`            | `#d47a3a` | Partial-match warnings, torchlight |
| `--mana`             | `#3e6d93` | Player HP base                     |
| `--mana-bright`      | `#6a9ec4` | Player HP highlight                |
| `--verdant`          | `#8ba74a` | Successful attempts (✓ BOUND)      |
| `--night`            | `#0f0a07` | Ground background                  |

Dominant surfaces: ink-black ground with gold-trimmed leather panels; parchment
only appears **inside** the Book of Acting. Crimson and ember are reserved for
hostile / strain signals, never for decoration.

### Typography

| Family                                   | Use                                                                 |
| ---------------------------------------- | ------------------------------------------------------------------- |
| **Cormorant Unicase**                    | Headings, combatant names, spell words, phase name, numeric counter |
| **Cormorant Garamond**                   | Body italics, narrative text                                        |
| **IM Fell DW Pica SC**                   | Small-caps labels, status pills, legends                            |
| **UnifrakturMaguntia**                   | Blackletter dropcaps inside the book                                |
| **Segoe UI Historic** (fallback `serif`) | Elder Futhark runes (use the sprites for the real version)          |

In Bevy these all need to be shipped as `.ttf` in `assets/fonts/` and loaded via
`bevy_asset_loader`. All are OFL-licensed. Runes can rely on the font already
used by `FutharkKeyRuneVisual`.

### Textures / paper effect

The parchment is **not** a bitmap — it's layered gradients:

```css
background:
  radial-gradient(ellipse at 50% 105%, rgba(0,0,0,0.4), transparent 55%),
  linear-gradient(180deg, #f0e1bf 0%, #c9b383 100%);
```

Plus a woven-fiber pattern via two `repeating-linear-gradient`s crossing at
172° / 88°, mix-blend-mode multiply, ~7% opacity.

Plus a soft inner vignette via `inset box-shadow: 0 0 4cqh rgba(139,109,42,0.28)`.

Plus leather binding as two stacked `0 0 0 Ncqh` outer box-shadows (#3a2414
then #1a0f08).

**Bevy implementation options**:
- Simplest: bake a parchment texture PNG (256×256 tileable) and use
  `ImageNode` with `ImageScaleMode::Tiled`. Recommended — the gradient stack
  is not cheap to reproduce in UI shaders.
- For the leather binding frame: a 9-slice border texture.

Global screen effects (on `body::before/after`):
- **Grain overlay** — two overlapping dotted radial-gradients at 3px and 7px
  spacing, mix-blend-mode overlay.
- **Vignette** — radial gradient from transparent at 35% → `rgba(0,0,0,0.8)`
  at edges.

In Bevy these can be a fullscreen `ImageNode` with a tiled noise texture at low
alpha on top of the UI root.

---

## Components

### Combat bar (top)

- Two `combatant` blocks flanking a central `phase` block.
- **Portrait**: 5.8cqh circle, radial gradient fill, 3-ring frame
  (gold border / black hairline / dark gold outer), inner shadow, and a
  `conic-gradient` shimmer ring that rotates every 10s (`@keyframes shimmer`).
- **HP bar**: 1.9cqh tall, clipped to a chevron with `clip-path: polygon(...)`
  — both ends taper. Fill uses a vertical gradient, a top white sheen
  (`::after`), and 10 vertical tick marks as `<span>` flex children.
- Enemy HP **drains rightward** — the fill anchors to `right: 1px` rather
  than `left`. Watch this in Bevy; a simple percentage-width node naturally
  anchors left, so the enemy bar needs `justify_content: End` or equivalent.
- **Phase banner**: Cormorant Unicase 2.8cqh with a glowing ember text-shadow
  + three pip indicators (ReactingActingBinding), active one filled gold.

### Inscribed Attempts (left)

Mirrors the rune-word lane. Structure:

1. **Active composition card** — crimson-bordered, glowing, has a floating
   `INSCRIBING` label pinned to the top edge (`::before` with negative top).
   Shows the in-progress rune string + a blinking crimson caret + target
   word on the right.
2. **Divider** — em-rule flanked by faint gold lines (`flex: 1; height: 1px`).
3. **Ledger** — up to 4 prior attempts with:
   - Roman numeral index
   - Rune string (line-through in crimson if rejected)
   - Italic word + one-line kenning (`"Flem" · misread kenning`)
   - Mark column: `✓` verdant, `~` ember, `✗` crimson
   - Left border stripe in the corresponding colour
   - Oldest entry at opacity 0.55

### Book of Acting (right)

A single illuminated page (not a two-page spread — too narrow in the column).

- Parchment background (see *paper effect* above).
- Red ribbon `.bookmark` clipped via `polygon` peeking above the top edge.
- Italic `pagehead` ("⸺ grimoire · folio xxiv ⸺") with a faint bottom rule.
- **4 `.spell` rows**, each a three-column grid:
  `[dropcap 4cqh] [content 1fr] [sigil 3cqh]`.
  - Dropcap: UnifrakturMaguntia blackletter in `--blood`.
  - Content: UPPERCASE word (Cormorant Unicase) / runes / italic kenning.
  - Sigil: 40px circle with double-ring border, alchemical glyph inside
    (🜂 🜄 ⚔ 🜛). These can be either unicode (simplest) or 24×24 sprites
    if Bevy's font doesn't cover the alchemical block.
- **Active spell** gets an ember-tinted background, crimson left border,
  inner glow, and a pulsing `☛` pointing-finger to its left.

### Binding Strain (bottom, full width)

Three-column grid:
`[title + copy] [chain] [count]`.

- **Chain** — five `.link` ovals, each 5.2×2.8cqh with 0.38cqh gold border.
  `:nth-child(even)` rotates 90° and `+ .link` gives `margin-left: -1.5cqh`
  so they interlock.
- Link states:
  - `.link` — intact, gold border, warm radial highlight.
  - `.link.strained` — animated `filter: brightness()` pulse + hairline
    `::after` crack.
  - `.link.broken` — crimson border, dark red fill, clip-path splits the
    oval in two halves with a glowing hot-centre `::before`.
- The counter on the right is a big 3.4cqh crimson Cormorant Unicase
  `3` followed by a muted ` / 5` small, plus a spaced "✦ strain rising ✦"
  tag.

### Rune Keyboard (left column, beneath attempts)

Own panel with its own header row (`Rune Keyboard · tab · legend`).

Three staggered rows, matching `src/futhark.rs:38-42`:

| Row | Keys                    | Padding-left | Ratio (vs. 48px + 8px gap) |
| --- | ----------------------- | ------------ | -------------------------- |
| r1  | **Tab** + 10 runes      | `0`          | 0                          |
| r2  | 9 runes                 | `5.3cqh`     | 1.71 key-widths (≈96px)    |
| r3  | 7 runes + gap + **Del** | `7.1cqh`     | 2.29 key-widths (≈128px)   |

- Regular key: 2.8cqh square, dark leather gradient, gold-dark border,
  IM Fell runic glyph in gold-light.
- `.key.pressed` — crimson gradient + ember text glow (the **currently held**
  key).
- `.key.glyph` — slightly lighter leather + full-gold border, used for
  keys whose rune is already in the active word.
- `.key.action` — 4.67cqh wide (the 80/48 ratio from the real code),
  IM Fell small-caps label ("Tab", "Del") instead of a rune.
- `.gap` — invisible 2.8cqh square preserving the `usize::MAX` hole in
  the bottom row.

### Battle Arena (centre)

- `background: url('assets/images/backdrop.png') center/cover` — the
  square source crops naturally to whatever shape the grid cell takes.
- Gold corner brackets (`.bracket.tl/tr/bl/br`) with a diamond pip on
  each outer corner.
- NPC sprite centred: 22cqh square, `background-size: 44cqh 44cqh` with
  `background-position: 0 -22cqh` to select the **acting-phase** frame
  (the bottom-left of the 2×2 sheet). `image-rendering: pixelated`.
- Breathing ground-shadow, bob keyframes, drifting ember motes, torchlight
  flicker radial gradient.
- Floating overlays: top-left "ACTING PHASE" pill with a pulsing ember dot,
  centred name tag (`⸻ the quarry ⸻ / HOARDLING VRASK / threat · iii of v`),
  bottom-left italic zone caption.

---

## Motion catalog

| Name      | Target                               | Keyframes / duration             |
| --------- | ------------------------------------ | -------------------------------- |
| `shimmer` | portrait conic ring                  | 360° rotate / 10s linear         |
| `blink`   | active-attempt caret                 | opacity 0↔1 steps(2) / 0.9s      |
| `pulse`   | phase pill dot, active spell pointer | opacity + slight shift / 1.4s    |
| `flicker` | arena torchlight gradient            | opacity 0.7↔1 / 3.6s             |
| `drift`   | arena motes                          | translate + scale + fade / 9–14s |
| `bob`     | NPC sprite                           | translateY 0↔-0.7cqh / 2.4s      |
| `breathe` | NPC ground shadow                    | scaleX 1↔0.88 / 2.4s             |
| `strain`  | strained chain link                  | brightness 1↔1.35 / 2.2s         |

In Bevy, a lightweight approach: add a `BattleUiClock` resource counting a
wrapping `f32`, and drive each of these via a small system that reads the
clock and writes back transforms / colours / alphas per-entity. Avoid
bevy_tweening for this — the motions are all periodic, not one-shot.

---

## Sprites / assets to add

Existing assets (already in `assets/images/`): `backdrop.png`, `futhark.png`,
`goblin.png`, `robed.png`. The mockup references backdrop + goblin directly.

**New sprites needed for faithful implementation**:

1. **Binding chain links**, three states — *intact*, *strained*, *broken*.
   Each ≈52×28px, with alternating rotation handled at layout time. Broken
   variant includes the crimson glow and split-half clip. These can be a
   single 3-column sprite-sheet.
2. **Parchment texture** — 256×256 tileable, subtle fibre grain. Needed for
   the Book of Acting page background.
3. **Leather / gold frame** — 9-slice texture (~16px border) for the panels
   (Inscribed, Keyboard, Binding, Combat bar). Alternative: draw rectangles
   with `BorderColor` and accept a simpler look.
4. **Portrait cameo frame** — optional. The mockup builds it entirely from
   box-shadows; in Bevy a single PNG ring of ~80×80 with transparent centre
   is easier.
5. **Spell sigils** — 4 tiny 32×32 icons (fire / frost / blade / ward) to
   replace the unicode alchemical glyphs if font coverage is a problem.
6. **Corner bracket + pip** — can be a single ~16×16 PNG reused at each
   arena corner.
7. **Grain / vignette overlay** — 1 fullscreen PNG at low alpha, or a
   procedural noise shader.
8. **HP-bar frame** — the chevron-ended bar. Can be done with two 9-slice
   segments (end cap + mid fill) or as a composite drawn via `Node` chevrons;
   the clip-path trick doesn't translate directly to Bevy UI, so an atlas is
   the pragmatic route.

Note: the pixel-art NPC + backdrop intentionally contrast with the
vector-feeling HUD. Keep that distinction — don't pixelate the UI.

---

## Numeric constants worth preserving

- Keyboard row offsets in game px: `[0, 96, 128]` — already in
  `src/futhark.rs:38`. Don't change these; the mockup derives from them.
- Regular key: 48×48px; action key (Tab/Del): 80×48px; column gap 8px.
- Row key counts: 10 / 9 / 7 (with one `usize::MAX` gap at bottom-row idx 3).
- NPC sprite sheet: 128×128, 2×2 grid of 64×64 frames.
  Phase → frame index: Idle=0, Reacting=1, Acting=2, Binding=3.
  (Matches `AGENTS.md`.)
- Combat bar height: ~8cqh. Middle row: `1fr`. Binding: auto.
  Keyboard (within left column): ~20cqh.

---

## Implementation checklist (Bevy 0.18)

- [ ] Introduce a `BattleHudRoot` node pinned to a 16:9 region of the window;
      use `ComputedNode` percentages for everything underneath.
- [ ] Extend `GameAssets` with: parchment tile, leather-frame 9-slice,
      binding-link atlas (3 frames), spell-sigil atlas (4), portrait ring,
      grain overlay, HP-bar chevron atlas.
- [ ] Load the new fonts in `src/loading.rs` via `bevy_asset_loader` (crate
      0.26). Keep UnifrakturMaguntia optional — fall back to Cormorant Unicase
      if the asset is missing.
- [ ] Replace `configure_*` functions per panel: `configure_inscribed_lane`,
      `configure_keyboard_panel` (keep the existing spawner; just wrap in a
      bordered frame), `configure_book_of_acting`, `configure_binding_strain`,
      `configure_arena`, `configure_combat_bar`.
- [ ] Drive animations from a single `BattleUiClock` resource — periodic
      functions, not tween trees.
- [ ] Enemy HP fill must be right-anchored. Player HP fill is left-anchored.
      Both should share a shared `HpBarState` component but render mirrored.
- [ ] Keep pixel-art rendering (`ImageSamplerDescriptor::nearest`) for the
      backdrop and NPC, and **default (linear)** for UI frames and fonts.
- [ ] The `.gap` slot in the bottom keyboard row is a real empty `Node` — do
      not collapse it; layout stagger depends on it.
- [ ] The crimson "INSCRIBING" pill floats above the active-attempt card
      (negative top). In Bevy: use `PositionType::Absolute` + negative `top`.

---

## Open questions / deferred decisions

- Should the book swap to a 2-page spread (as in the earlier mockup draft)
  when the column is wide enough? Probably yes at full 16:9, only if the
  right column ends up > ~400px rendered width. Current single-page layout
  is safer for narrow displays.
- The spell sigils (🜂 🜄 ⚔ 🜛) rely on OS emoji/symbol coverage. Decide
  between unicode (cheap, may render inconsistently) or custom 32×32 sprites
  (one asset each) before shipping.
- The combat bar's portrait cameos are decorative in the mockup (unicode
  glyphs). The final version should either use cropped NPC sprites or
  a dedicated portrait atlas.
