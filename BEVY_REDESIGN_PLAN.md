# Bevy Battle UI Redesign — Implementation Plan

Companion to `MOCKUP_REDESIGN.md` and `mockups/battle-ui.html`.
This document translates the mockup into a concrete Bevy 0.18 task list,
organised by dependency order. Sections marked **DEFERRED** should not be
started until the prerequisite design decision is made.

---

## 0. Pre-requisites

### 0.1 Assets to create

| Asset                  | Path                               | Notes                                                                                   |
| ---------------------- | ---------------------------------- | --------------------------------------------------------------------------------------- |
| Parchment tile         | `assets/images/parchment_tile.png` | 256×256 tileable, subtle fibre grain. Replaces the CSS gradient stack on the Book page. |
| Corner bracket + pip   | `assets/images/corner_bracket.png` | ~32×32 with transparent surround. One PNG; rotate/flip at spawn time for each corner.   |
| Grain/vignette overlay | `assets/images/vignette.png`       | Full-screen (or 1×1 repeat) at low alpha over all UI.                                   |
| HP bar chevron end-cap | `assets/images/hp_end.png`         | ~24×32, 9-sliceable L/R pair. Optional — can use `clip_path` if a UI shader is added.   |
| Portrait ring          | `assets/images/portrait_ring.png`  | ~80×80 transparent-centre ring. Optional — can approximate with nested `Node` borders.  |
| Spell sigils atlas     | `assets/images/sigils.png`         | 4 icons (fire/frost/blade/ward), 32×32 each, one row → 128×32.                          |
| **Binding link atlas** | _DEFERRED_                         | 3 states × 52×28 px. Do not create until Binding rules are defined.                     |

### 0.2 Fonts to add to `assets/fonts/`

All OFL-licensed, downloadable from Google Fonts:

| Filename                        | Family                    | Use                                               |
| ------------------------------- | ------------------------- | ------------------------------------------------- |
| `CormorantUnicase-SemiBold.ttf` | Cormorant Unicase 600     | Headings, spell words, phase name, numeric labels |
| `CormorantUnicase-Bold.ttf`     | Cormorant Unicase 700     | Large headings, phase banner                      |
| `CormorantGaramond-Italic.ttf`  | Cormorant Garamond italic | Body text, ledger word subtitles, flavor copy     |
| `IMFellDWPicaSC-Regular.ttf`    | IM Fell DW Pica SC        | Small-caps labels, status pills, keyboard header  |
| `UnifrakturMaguntia-Book.ttf`   | UnifrakturMaguntia        | Blackletter dropcaps in Book of Acting            |

The existing futhark sprite font (`.png` atlas) already handles rune display.
Register all fonts in `GameAssets` via `bevy_asset_loader` with `#[asset(path = "fonts/...")]`.

### 0.3 Color constants module

Create `src/ui/palette.rs` (or inline in the HUD root module) with Bevy
`Color` constants matching the CSS variables exactly:

```rust
pub const PARCHMENT:        Color = Color::srgb_u8(0xea, 0xd9, 0xb4);
pub const PARCHMENT_WARM:   Color = Color::srgb_u8(0xf0, 0xe1, 0xbf);
pub const PARCHMENT_SHADOW: Color = Color::srgb_u8(0xc9, 0xb3, 0x83);
pub const PARCHMENT_DARK:   Color = Color::srgb_u8(0xa8, 0x8f, 0x5f);
pub const INK:              Color = Color::srgb_u8(0x23, 0x15, 0x10);
pub const GOLD:             Color = Color::srgb_u8(0xc9, 0xa2, 0x4b);
pub const GOLD_DARK:        Color = Color::srgb_u8(0x8b, 0x6d, 0x2a);
pub const GOLD_LIGHT:       Color = Color::srgb_u8(0xf0, 0xd4, 0x8a);
pub const BLOOD:            Color = Color::srgb_u8(0x8b, 0x1e, 0x2e);
pub const BLOOD_BRIGHT:     Color = Color::srgb_u8(0xc1, 0x35, 0x46);
pub const EMBER:            Color = Color::srgb_u8(0xd4, 0x7a, 0x3a);
pub const MANA:             Color = Color::srgb_u8(0x3e, 0x6d, 0x93);
pub const MANA_BRIGHT:      Color = Color::srgb_u8(0x6a, 0x9e, 0xc4);
pub const VERDANT:          Color = Color::srgb_u8(0x8b, 0xa7, 0x4a);
pub const NIGHT:            Color = Color::srgb_u8(0x0f, 0x0a, 0x07);
```

---

## 1. Root Layout — `BattleHudRoot`

**New module**: `src/ui/mod.rs`  
**Entry point**: `pub fn configure_hud(app: &mut App)`

### 1.1 The 16:9 root node

Replace the current ad-hoc absolute-positioned layout with a single parent
node that locks the aspect ratio:

```rust
// Outer centering wrapper (fills window)
Node {
    width: Val::Percent(100.0),
    height: Val::Percent(100.0),
    justify_content: JustifyContent::Center,
    align_items: AlignItems::Center,
    ..default()
}

// Inner 16:9 box (BattleHudRoot)
Node {
    width: Val::Vw(100.0),   // or constrained; see note below
    aspect_ratio: Some(16.0 / 9.0),
    display: Display::Grid,
    grid_template_columns: vec![
        RepeatedGridTrack::fr(1, 22.0),
        RepeatedGridTrack::fr(1, 50.0),
        RepeatedGridTrack::fr(1, 22.0),
    ],
    grid_template_rows: vec![
        RepeatedGridTrack::auto(1),      // combat bar
        RepeatedGridTrack::fr(1, 1.0),  // middle row
        RepeatedGridTrack::auto(1),      // binding bar
    ],
    column_gap: Val::Percent(1.0),
    row_gap: Val::Percent(1.0),
    padding: UiRect::all(Val::Percent(1.4)),
    ..default()
}
```

> **Note on sizing**: Bevy 0.18 `Val::Vw`/`Val::Vh` exists. The HTML uses
> `min(100vw, 100vh * 16/9)` — the equivalent in Bevy is to set
> `width: Val::Percent(100.0)` and `aspect_ratio: Some(16.0/9.0)` on the
> inner node; the engine will letterbox automatically.

**Component to tag the root**: `#[derive(Component)] pub struct BattleHudRoot;`

### 1.2 Column children mapping

| Grid area      | Component tag  | Section |
| -------------- | -------------- | ------- |
| row 1, col 1–3 | `CombatBar`    | §2      |
| row 2, col 1   | `LeftColumn`   | §3–4    |
| row 2, col 2   | `ArenaPanel`   | §5      |
| row 2, col 3   | `BookPanel`    | §6      |
| row 3, col 1–3 | `BindingPanel` | §7      |

The `LeftColumn` is a flex-column child that itself contains:
- `InscribedPanel` (flex: 1 — takes remaining height)
- `KeyboardPanel` (flex: 0, auto height)

### 1.3 ClearColor and background

Set `ClearColor` to `NIGHT` (`#0f0a07`). The current blue should be replaced.

The dark radial-gradient body background in the HTML is a cosmetic polish;
approximate it with the clear color + optional fullscreen `ImageNode` vignette
(§1.4).

### 1.4 Grain and vignette overlay

Spawn a fullscreen `Node` (100% × 100%, `PositionType::Absolute`, high
`ZIndex`) with an `ImageNode` showing `vignette.png` at ~40% alpha.
To tint the image's alpha, set the `color` field on `ImageNode` directly —
**not** `BackgroundColor`, which only draws behind the image:

```rust
ImageNode {
    image: game_assets.vignette.clone(),
    color: Color::srgba(1.0, 1.0, 1.0, 0.4),
    ..default()
}
```

`BackgroundColor` on a node that also has `ImageNode` draws the background
colour in the area behind/around the image, not as a tint over it.

---

## 2. Combat Bar

**Component**: `CombatBar`  
**Grid position**: row 1, columns 1–3 (`grid_column: GridPlacement::span(3)`)

### 2.1 Layout

Three-column grid inside the bar: `[1fr, auto, 1fr]`, align-items center.

Background: `BackgroundColor(Color::srgba(0.07, 0.04, 0.02, 0.85))` plus
a `BorderColor { top: GOLD_DARK, right: GOLD_DARK, bottom: GOLD_DARK, left: GOLD_DARK }`
+ `border: UiRect::all(Val::Px(1.0))`.
(`BorderColor` is a named-field struct in Bevy 0.18; there is no tuple-struct
constructor. All four sides must be set explicitly, or use `..default()` to
fill the rest with black.)

### 2.2 Player combatant block (`CombatantBlock { side: Player }`)

Left side, row direction:

1. **Portrait** (`PortraitNode`) — circular appearance via `border_radius: BorderRadius::all(Val::Percent(50.0))`, `BackgroundColor(MANA dark)`, `border: GOLD`. Contains a `Text` with the UnifrakturMaguntia font for a placeholder glyph (or leave blank until portrait sprites exist).
2. **Stats** (flex-col):
   - `CombatantName` — Cormorant Unicase SemiBold, `GOLD_LIGHT`, ~`font_size: 20.0` (tune to feel right in percent layout).
   - `HpBarNode { side: Player }` — see §2.4.

### 2.3 Enemy combatant block (`CombatantBlock { side: Enemy }`)

Mirror of §2.2: flex `flex_direction: FlexDirection::RowReverse`. HP fill anchors from the
right — spawn the fill node with `position_type: Absolute, right: Val::Px(1.0)`
and drive its `width`.

### 2.4 HP bars (`HpBarNode`)

```
Outer node (relative position):
  height: Val::Percent(2.0)  (≈1.9cqh)
  border: 1px GOLD_DARK
  background: near-black gradient (approximated with flat BackgroundColor)
  overflow: Overflow::clip()

  Fill node (absolute, inset 1px):
    Player: width: Val::Percent(hp_pct), left: 0
    Enemy:  width: Val::Percent(hp_pct), right: 0, left: auto
    BackgroundColor → mana gradient (player) or blood gradient (enemy)

  Ticks overlay (absolute, flex-row, 10 children):
    Each child: flex:1, border_right: 1px rgba(0,0,0,0.5)

  HP text label (absolute, centered):
    Text "78 / 100 · VITÆ", IM Fell SC font, PARCHMENT color
```

The chevron `clip_path` from CSS cannot be reproduced with a plain `Node`.
**Recommended approach**: use the HP bar end-cap PNG (§0.1) as a 9-slice
`ImageNode` overlay. If the asset isn't ready, use flat rectangle bars.

Health state is split by ownership:
- **`PlayerHealthState { hp: u32, max: u32 }`** — `Resource`. The player is a
  singleton; a resource is the natural home for it.
- **`NpcHealthState { hp: u32, max: u32 }`** — `Component` attached to the
  spawned NPC entity in `combat.rs`. Attaching to the entity means HP is
  created/despawned with the NPC automatically and scales to multi-NPC
  encounters without a separate registry.

System `sync_hp_bars` reads the player resource for the left fill and queries
the active NPC entity for the right fill; it sets both node widths each frame.

### 2.5 Phase banner (`PhaseBannerNode`)

Center column of the combat bar:

```
flex-col, align-items: center

  Text "⸻ current phase ⸻"  — IM Fell SC, PARCHMENT_DARK, small
  Text phase_name            — Cormorant Unicase Bold, GOLD_LIGHT, large (~34px)
                               text-shadow ember glow → use TextColor + custom
                               (Bevy Text doesn't support text-shadow natively;
                               spawn a duplicate Text node behind it in EMBER at
                               slight blur offset, or skip the glow in MVP)
  Pips row (flex-row, 3 × Node circles):
    inactive: border GOLD_DARK, background transparent
    active: BackgroundColor(GOLD_LIGHT), BorderColor(GOLD)
```

System `sync_phase_banner(state: Res<BattleState>)` updates the text and pip
fill colors each frame when `BattleState` changes.

---

## 3. Inscribed Attempts Panel (Left Column, Upper)

**Replaces** the current rune-word lane at top-left which uses absolute
`Val::Px` positioning.

**Component**: `InscribedPanel`  
**Module**: `src/ui/inscribed.rs`

The panel uses the standard dark-leather style (§8.1).

### 3.1 Active attempt card (`ActiveAttemptCard`)

```
Node (relative):
  border: 1px BLOOD
  background: semi-transparent ember tint + dark base
  padding: 1% 0.9%

  "INSCRIBING" floating label — PositionType::Absolute, top: -12px:
    border: 1px BLOOD, background: NIGHT_2
    Text "INSCRIBING", IM Fell SC, BLOOD_BRIGHT, letter-spacing

  Rune display row:
    Text built from current rune slots  (drives existing RuneSlot logic)
    Blinking caret entity (see §9.3)

  Progress line (optional):
    Text "composing · N glyphs inscribed", italic Cormorant Garamond, PARCHMENT_DARK
```

**Key invariant**: this card NEVER shows the target word or any match
coloring. It is display-only until the row is committed.

The existing `spawn_battle_row` / `RuneSlot` entities move into this card.
Their `top`/`left` absolute positions will need to be replaced with flex
layout inside the card.

### 3.2 Divider

Thin flex-row: two `Node` lines (`flex: 1, height: 1px, BackgroundColor(GOLD_DARK with 0.5 alpha)`) flanking a `Text("previous strokes")` in IM Fell SC.

### 3.3 Ledger (`LedgerList`)

A flex-col `Node` (`flex: 1`) containing up to 4 `AttemptRow` entries.
Each row:

```
Node (grid: [auto 1fr], gap):
  Index column:
    Text "IV." / "III." etc, IM Fell SC, PARCHMENT_DARK

  Stroke column (flex-col):
    Tiles row (flex-row, gap 2px):
      Per rune: Node (min-width: ~2%, height: 2%)
        BackgroundColor driven by RuneMatchState (Missing/Present/Correct)
        Text rune character (futhark sprite or Text node)
        border: 1px rgba-black

    Word subtitle:
      if known:  Text "\"Word\"", Cormorant Garamond italic, PARCHMENT_DARK
      if unknown: Text "— word unknown —", same font, lower alpha, wider
                  letter-spacing (use a distinct component WordUnknownLabel)
```

`AttemptRow` component holds `row_id`, resolved word (if any), and tile states.  
The oldest visible row gets `opacity: 0.55` applied — **Bevy UI has no
inherited opacity**. Setting `BackgroundColor` alpha to 0.55 only dims the row
background rectangle, not child text or images. To fade the whole row, a sync
system must set `TextColor` alpha to 0.55 on every text child and
`ImageNode.color.set_alpha(0.55)` on every image child of the oldest
`AttemptRow` entity. A helper that walks the row's children recursively is the
cleanest approach.

The existing `RowLetterGraded` message already carries `RuneMatchState` per
letter — route this to populate the `AttemptRow` entries.

---

## 4. Rune Keyboard Panel (Left Column, Lower)

**Replaces** the free-floating keyboard that currently lives in `futhark.rs`
with absolute pixel coordinates.

**Component**: `KeyboardPanel`  
**Existing logic in `configure_futhark_keyboard`**: keep all systems; only
change the spawner so keys are children of the panel `Node` instead of
absolute sprites.

### 4.1 Panel frame

Standard dark-leather panel (§8.1), with header:

```
Panel header (flex-row, space-between):
  Text "Rune Keyboard"  — Cormorant Unicase SemiBold, GOLD_LIGHT
  Text "tab · legend"  — IM Fell SC italic, PARCHMENT_DARK
```

### 4.2 Key rows

The existing `KEYBOARD_ROW_OFFSETS: [0.0, 96.0, 128.0]` pixel offsets map
to CSS `padding-left` of 0 / 5.3cqh / 7.1cqh. In `Val::Percent` (relative
to the panel width), these ratios should be preserved.

```
Keyboard container (flex-col, align-items: flex-start):
  Row r1 (flex-row, padding_left: 0%):
    Tab key (action, ~1.67× wide) + 10 rune keys
  Row r2 (flex-row, padding_left: ~X%):
    9 rune keys
  Row r3 (flex-row, padding_left: ~Y%):
    3 rune keys + gap node + 4 rune keys + Del key (action)
```

Each rune key `Node`:
- `width`/`height`: `Val::Percent(~2.8%)` (relative to parent, tune to
  match the cqh-based sizing from the mockup).
- `BackgroundColor`: dark leather base.
- `BorderColor(GOLD_DARK)`, border 1px.
- Child `ImageNode` (futhark atlas sprite) in `GOLD_LIGHT`.
- `.glyph` state (rune already in active word): `BorderColor(GOLD)`, slightly
  lighter background.
- `.pressed` state: `BackgroundColor(BLOOD)`, `BorderColor(BLOOD_BRIGHT)`.

The existing `FutharkKeyBackground`, `FutharkKeyRuneVisual`, etc. components
are reused; only the spawn geometry changes.

### 4.3 Legend mode toggle

No change to logic. In legend mode the rune sprites are replaced by letter
`Text` nodes — already implemented via `FutharkKeyLetterVisual`.

---

## 5. Battle Arena (Centre Column)

**Replaces** the current 256×256 top-right absolute layout in `src/combat.rs`.

**Component**: `ArenaPanel`  
**Grid position**: row 2, col 2.

### 5.1 Background

`ImageNode` for `backdrop.png` with `image_mode: NodeImageMode::Stretch`
(resizes to fill the cell, ignoring intrinsic size — the correct equivalent of
CSS `background-size: cover` for a square backdrop). Preserve pixel art
sharpness by processing the asset after loading to set its sampler:
`ImageSamplerDescriptor::nearest`. The simplest way to do this in Bevy 0.18
is a one-shot startup system that calls
`images.get_mut(&game_assets.backdrop).unwrap().sampler = ImageSampler::nearest()`
after `GameState::Ready` is entered, before the first render.

Border: `BorderColor { top: GOLD, right: GOLD, bottom: GOLD, left: GOLD }`, 1px.
(`BorderColor` in Bevy 0.18 is a named-field struct, not a tuple struct.
Per-side colors are supported: e.g. `BorderColor { left: BLOOD, ..default() }`
for the active spell left-border in §6.3.)

### 5.2 Corner brackets

Spawn 4 `CornerBracket` child nodes at `PositionType::Absolute`:
- Each is a ~4% wide/tall `Node` at `top/left/right/bottom`.
- Two `Node` children: horizontal bar (width 100%, height 2px) and vertical
  bar (width 2px, height 100%), both `BackgroundColor(GOLD)`.
- Diamond pip: a `Node` rotated 45° at the outer corner, `BackgroundColor(GOLD_LIGHT)`.

If the bracket PNG asset (§0.1) is created, use it instead.

### 5.3 Phase-mark pill (`PhaseMark`)

```
PositionType::Absolute, top: ~1%, left: ~5%
Node:
  flex-row, align-items: center, gap small
  border: 1px GOLD_DARK, BackgroundColor(dark leather)

  Dot Node:
    width/height: ~0.7%, border-radius 50%
    BackgroundColor(EMBER)
    Driven by BattleUiClock for pulse animation (§9)

  Text "Acting Phase" — IM Fell SC, GOLD_LIGHT
```

System `sync_phase_mark` writes the text from `BattleState.phase`.

### 5.4 NPC sprite

Move from `src/combat.rs` current absolute layout into the Arena panel as a
child with `PositionType::Absolute`, centered:
- `left: 50%, top: 50%` + negative margins (or `margin: auto`)
- Size: `~22% × 22%` of arena height (was 128px within 256px = 50% of old
  fixed size; scale proportionally).
- `ImageSamplerDescriptor::nearest` (pixelated).
- Bob animation driven by `BattleUiClock` (§9).

Ground shadow: small `Node` ellipse (`border_radius: 50%`, dark semi-transparent
`BackgroundColor`) positioned at `bottom: 24%`. Breathe animation via clock.

### 5.5 Ember motes (**optional polish**)

7 small `Node` circles (`border_radius: 50%`, `BackgroundColor(EMBER)`) with
`PositionType::Absolute`. Drift animation driven by `BattleUiClock` (§9).
Can be deferred to a polish pass.

### 5.6 Torchlight flicker

An `ImageNode` or `Node` with `BackgroundColor` ember radial at center of
arena, blended at low alpha, alpha oscillated by `BattleUiClock`. Can be
a fullscreen arena overlay node with alpha-animated `BackgroundColor`.

---

## 6. Book of Acting (Right Column)

**Replaces** the current `ActingBookPanel` in `src/rune_words/battle_states/acting.rs`.

**Component**: `BookPanel`  
**Grid position**: row 2, col 3.

### 6.1 Outer wrapper

Dark leather panel frame (§8.1) containing:
- Header row: "Book of Acting" (Cormorant Unicase, GOLD_LIGHT) + "choose · inscribe" aside (IM Fell SC italic, PARCHMENT_DARK).

### 6.2 Inner parchment page (`BookPage`)

```
Node (flex: 1, flex-col, overflow: clip):
  BackgroundColor: layered parchment gradient approximation:
    Use ImageNode with parchment_tile.png and
    image_mode: NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 1.0 }
    or flat BackgroundColor(PARCHMENT_WARM) as fallback.
  border: 1px rgba(80,55,25,0.4) (inner leather border)
  box-shadow equivalent: inset border via nested Node or BorderColor

  Red bookmark:
    PositionType::Absolute, top: -8px, right: 20%
    Node: width ~1.4%, height ~5%, BackgroundColor(BLOOD)
    clip_path: not available in plain Bevy UI — approximate with
    a tall thin rectangle; skip the pointed bottom for MVP.

  Page head:
    Text "⸺ grimoire · folio xxiv ⸺", IM Fell SC italic, muted ink
    border-bottom 1px rule

  Spells list (flex-col, justify-content: space-between, flex: 1):
    4 × SpellEntry (§6.3)
```

### 6.3 Spell entries (`SpellEntry { index: usize }`)

Three-column grid: `[4cqh, 1fr, 3cqh]` → in percent: `[auto, 1fr, auto]`.

```
SpellEntry (relative):
  border-bottom: 1px dashed rgba(122,94,48,0.4)
  BackgroundColor: transparent (or ember tint if active)
  BorderColor(BLOOD) on left edge if active (border_left: 2px)

  Dropcap (auto column):
    Text first-letter, UnifrakturMaguntia, font_size large, Color(BLOOD)

  Content (1fr):
    Text word — Cormorant Unicase Bold, INK, uppercase
    Text runes — futhark sprite row (ImageNode children or Text if font
                 covers the rune codepoints)

  Sigil (auto):
    Node circle: border_radius 50%, border 1.5px GOLD_DARK
    Inner dashed ring (nested Node, border dashed — not directly supported
    in Bevy UI; use a solid thin ring instead, or a 32×32 sigil sprite)
    Text unicode glyph (🜂 🜄 ⚔ 🜛) or ImageNode from sigils atlas (§0.1)
```

**Active spell**: `BackgroundColor` ember tint, `BorderColor(BLOOD)` on left
side, pulsing `☛` pointer text node (alpha driven by `BattleUiClock`).

System `sync_book_panel` reads `ActingData.targets` and updates entries.
When `ActingData.grading_against_letters` is Some, highlight the matching
entry as active.

---

## 7. Binding Strain Panel (Bottom Row)

**DEFERRED** — Binding rules not yet defined. Per `MOCKUP_REDESIGN.md`:
> Do not port the link visuals verbatim. Wait for the Binding rules to be
> specified, then pick the HUD components to match.

### 7.1 What to implement now

Spawn the grid row 3 node with:
- `CombatBar`-style dark background, `BorderColor(GOLD_DARK)`, full-width.
- A TODO banner (flex-row):
  - Badge node: `Text "TODO"`, `BackgroundColor(EMBER)`, `Color(NIGHT)`, IM Fell SC.
  - Message: `Text "Binding Strain rules not yet defined."`, italic Cormorant Garamond, PARCHMENT.

### 7.2 What NOT to implement now

- Chain link entities with intact/strained/broken states.
- The three-column count/chain/title layout.
- Any animation for chain links.

---

## 8. Shared Primitives

### 8.1 Dark leather panel helper

A function `spawn_leather_panel(commands, grid_column, grid_row) -> Entity`
that spawns a `Node` with:
- `BackgroundColor`: `Color::srgba(0.07, 0.05, 0.02, 0.90)`.
- `BorderColor { top: GOLD_DARK, right: GOLD_DARK, bottom: GOLD_DARK, left: GOLD_DARK }`,
  `border: UiRect::all(Val::Px(1.0))`.
- **`BoxShadow` does NOT exist in Bevy 0.18.1.** Drop shadows / inset glows
  cannot be achieved with a built-in component. Workarounds:
  - Skip shadows entirely in MVP (flat border is legible).
  - Fake a drop shadow by spawning a slightly-larger duplicate node
    behind the panel at a small offset with a dark semi-transparent
    `BackgroundColor`.
  - Wait for a future Bevy version that adds box-shadow support.
- 4 corner diamond pips (§5.2 pattern, but smaller) via 4 absolute child nodes.
- Returns the entity so callers can add children.

### 8.2 Text helper

A function `panel_heading(text: &str, font: Handle<Font>) -> impl Bundle`
that returns a `(Text, TextFont { font, font_size: ... }, TextColor(GOLD_LIGHT))` bundle.

---

## 9. Animation — `BattleUiClock`

**New resource**: `src/ui/clock.rs`

```rust
#[derive(Resource, Default)]
pub struct BattleUiClock {
    pub elapsed: f32,
}

fn tick_clock(mut clock: ResMut<BattleUiClock>, time: Res<Time>) {
    clock.elapsed += time.delta_secs();
}
```

Register with `app.init_resource::<BattleUiClock>()` and
`app.add_systems(Update, tick_clock.run_if(in_state(GameState::Ready)))`.

### 9.1 Periodic animation helper

```rust
fn wave(clock: f32, period: f32, lo: f32, hi: f32) -> f32 {
    let t = (clock % period) / period;
    lo + (hi - lo) * (0.5 - 0.5 * (t * TAU).cos())
}
```

### 9.2 Systems driven by the clock

| Animation | Target component            | Period       | Range               | Driver system              |
| --------- | --------------------------- | ------------ | ------------------- | -------------------------- |
| `pulse`   | `PhaseMark` dot alpha       | 1.4s         | 0.45–1.0            | `animate_phase_dot`        |
| `pulse`   | Active spell `☛` alpha      | 1.4s         | 0.45–1.0            | `animate_spell_pointer`    |
| `flicker` | Arena torchlight alpha      | 3.6s         | 0.7–1.0             | `animate_torchlight`       |
| `bob`     | NPC sprite `top` offset     | 2.4s         | 0–7px               | `animate_npc_bob`          |
| `breathe` | Shadow `Transform::scale.x` | 2.4s         | 0.88–1.0            | `animate_npc_shadow`       |
| `blink`   | Caret entity alpha          | 0.9s         | 0–1 (steps 2)       | `animate_caret_blink`      |
| `shimmer` | Portrait ring rotation      | 10s          | 0–360°              | `animate_portrait_shimmer` |
| `drift`   | Mote entities               | 9–14s (vary) | y translate + alpha | `animate_motes`            |

### 9.3 Caret blink

Spawn a `Text("▍")` entity tagged `BlinkingCaret` as the last child of the
active-attempt rune display. `animate_caret_blink` sets its `TextColor` alpha
to 0.0 or 1.0 based on `(clock % 0.9) > 0.45`.

---

## 10. Refactoring Existing Code

### 10.1 `src/combat.rs`

The 256×256 `CombatScene` at top-right is replaced by `ArenaPanel` (§5).
`spawn_combat_scene` and `sync_npc_sprite` can be migrated to `src/ui/arena.rs`.
Keep the component names (`NpcSprite`, etc.) to avoid churn in the UAT tests.

### 10.2 `src/rune_words/battle.rs` — layout constants

`ACTIVE_ROW_TOP`, `ROW_LEFT`, `SLOT_SPACING`, `SLOT_SIZE` are pixel-absolute
constants for the old layout. After the rune word lane is moved into
`InscribedPanel`, these constants become irrelevant. Keep them temporarily
(renamed with `_LEGACY_` prefix) until all UAT tests pass with the new layout,
then remove.

### 10.3 `src/rune_words/battle_states/acting.rs`

`spawn_acting_book_panel` is replaced by the new `BookPanel` in §6.
`ActingBookPanel`, `ActingBookEntry`, `ActingBookEntryBackground` components
can be retired once `BookPanel` is driving from `ActingData`.

### 10.4 `src/loading.rs`

Add font handles to `GameAssets`:

```rust
#[asset(path = "fonts/CormorantUnicase-SemiBold.ttf")]
pub font_cormorant_unicase_semibold: Handle<Font>,
#[asset(path = "fonts/CormorantUnicase-Bold.ttf")]
pub font_cormorant_unicase_bold: Handle<Font>,
#[asset(path = "fonts/CormorantGaramond-Italic.ttf")]
pub font_cormorant_garamond_italic: Handle<Font>,
#[asset(path = "fonts/IMFellDWPicaSC-Regular.ttf")]
pub font_im_fell_sc: Handle<Font>,
#[asset(path = "fonts/UnifrakturMaguntia-Book.ttf")]
pub font_unifraktur: Handle<Font>,
// new image assets:
#[asset(path = "images/parchment_tile.png")]
pub parchment_tile: Handle<Image>,
#[asset(path = "images/corner_bracket.png")]
pub corner_bracket: Handle<Image>,
#[asset(path = "images/vignette.png")]
pub vignette: Handle<Image>,
#[asset(path = "images/sigils.png")]
pub sigils: Handle<Image>,
#[asset(texture_atlas_layout(tile_size_x = 32, tile_size_y = 32, columns = 4, rows = 1))]
pub sigils_layout: Handle<TextureAtlasLayout>,
```

---

## 11. Suggested Implementation Order

1. **§0.3** — Add palette constants. Zero risk.
2. **§9** — Add `BattleUiClock`. Zero risk; needed by everything animated.
3. **§1.1–1.2** — Introduce `BattleHudRoot` grid. Spawn it on `OnEnter(GameState::Ready)` **alongside** existing panels (not replacing them yet). Verify grid layout in UAT.
4. **§2** — Combat bar (HP bars with flat rectangles, phase banner). Stub HP with `PlayerHealthState` (resource) and `NpcHealthState` (component on the NPC entity) holding fixed values for now.
5. **§5** — Migrate arena + NPC into the centre grid column. Remove old `CombatScene` absolute layout. **Run `uat_battle_stages` to verify.**
6. **§4** — Wrap keyboard in `KeyboardPanel`. Migrate key spawn coordinates from absolute px to percent-relative. **Run `uat_shows_futhark_rune` and navigation UATs.**
7. **§3** — Replace rune-word lane with `InscribedPanel` + `LedgerList`. Wire `RowLetterGraded` → tile colors. **Run `uat_shows_acting_battle_state`.**
8. **§6** — Replace `ActingBookPanel` with full `BookPanel`. Wire `ActingData` → spell entries. **Run `uat_shows_acting_battle_state` again.**
9. **§7** — Stub `BindingPanel` with TODO banner.
10. **§0.2 + §10.4** — Add fonts to assets, propagate `Handle<Font>` into text nodes. **Run all UATs.**
11. **§8.1** — Polish shared panel frame helper (corner pips, box shadows).
12. **§9.2** — Wire `BattleUiClock` into each animation system.
13. **§0.1** — Add parchment tile, bracket, vignette, sigil assets. Swap into nodes.

---

## 12. Known Gaps / Deliberate Omissions

| Feature                        | Reason deferred                                                                                    |
| ------------------------------ | -------------------------------------------------------------------------------------------------- |
| Chevron-ended HP bar           | Requires clip-path or 9-slice asset; use flat rect initially                                       |
| Portrait shimmer conic ring    | Requires rotating `Node` or custom shader; low priority                                            |
| `text-shadow` glow on headings | Bevy `Text` has no `text-shadow`; duplicate shadow node or skip                                    |
| Book page woven-fiber texture  | CSS `mix-blend-mode: multiply` has no Bevy equivalent in plain UI; bake into parchment PNG instead |
| Binding chain links            | Rules undefined; see §7                                                                            |
| Ember motes                    | Pure polish; add after core layout is stable                                                       |
| 2-page book spread             | Deferred per open question in MOCKUP_REDESIGN.md                                                   |
| Spell sigil unicode coverage   | OS font coverage uncertain; use sigil PNG atlas (§0.1) for safety                                  |
| Portrait cameo sprites         | Placeholder glyphs until portrait atlas exists                                                     |

---

## 13. Bevy 0.18 API — Verified Correctness Notes

This section records the results of checking all API claims in this document
against Bevy 0.18.1 source (`bevy_ui-0.18.1`). Items marked ✅ are confirmed
correct; ⚠️ marks a Bevy limitation to work around; ❌ marks something that
was wrong in an earlier draft of this plan (already corrected above).

### Layout / Node

| Claim                                                   | Status | Notes                                      |
| ------------------------------------------------------- | ------ | ------------------------------------------ |
| `Val::Vw` / `Val::Vh`                                   | ✅      | Exist in 0.18                              |
| `Node.aspect_ratio: Option<f32>`                        | ✅      | Confirmed in `ui_node.rs`                  |
| `Display::Grid`                                         | ✅      | Supported                                  |
| `grid_template_columns: Vec<RepeatedGridTrack>`         | ✅      | Confirmed field name                       |
| `RepeatedGridTrack::fr(count: u16, value: f32)`         | ✅      | Confirmed                                  |
| `grid_column: GridPlacement` / `GridPlacement::span(n)` | ✅      | Confirmed                                  |
| `Overflow::clip()`                                      | ✅      | Confirmed via `OverflowAxis::Clip` backing |
| `BorderRadius` component                                | ✅      | Exists, separate from `Node`               |
| `UiRect` for per-side border widths                     | ✅      | `Node.border: UiRect`                      |

### BorderColor

| Claim                                                               | Status | Notes                                                                                                                       |
| ------------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------- |
| `BorderColor` is a **named-field** struct                           | ✅      | `{ top, right, bottom, left: Color }` — per-side colors fully supported                                                     |
| `BorderColor(single_color)` tuple syntax                            | ❌      | **Does not compile.** Use `BorderColor { top: X, right: X, bottom: X, left: X }`; also confirmed broken by repo memory note |
| Left-only active border: `BorderColor { left: BLOOD, ..default() }` | ✅      | Per-side colors mean this works cleanly                                                                                     |

### FlexDirection

| Claim                                            | Status | Notes                              |
| ------------------------------------------------ | ------ | ---------------------------------- |
| `FlexDirection::RowReverse`                      | ✅      | Confirmed enum variant             |
| `FlexDirection::Row::Reverse` (old plan wording) | ❌      | Not valid Rust — corrected in §2.3 |

### Images / ImageNode

| Claim                                                    | Status | Notes                                                                   |
| -------------------------------------------------------- | ------ | ----------------------------------------------------------------------- |
| `ImageNode.color: Color` for alpha tinting               | ✅      | Confirmed field; multiply tint                                          |
| `ImageNode.image_mode: NodeImageMode`                    | ✅      | Confirmed field                                                         |
| `NodeImageMode::Auto`                                    | ✅      | Default; auto-sizes from texture                                        |
| `NodeImageMode::Stretch`                                 | ✅      | Resizes to node size, ignores texture aspect ratio                      |
| `NodeImageMode::Tiled { tile_x, tile_y, stretch_value }` | ✅      | All three fields required                                               |
| `NodeImageMode::Sliced(TextureSlicer)`                   | ✅      | For 9-slice; use for HP bar end-caps                                    |
| `ImageScaleMode::Stretched` / `ImageScaleMode::Tiled`    | ❌      | Type does not exist in 0.18 — corrected in §5.1 and §6.2                |
| `BackgroundColor` as image tint                          | ❌      | Does not tint image pixels; only draws behind image — corrected in §1.4 |

### BoxShadow / TextShadow

| Claim                             | Status | Notes                                                                                                                                                               |
| --------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `BoxShadow` component on UI nodes | ❌      | **Not present in bevy_ui-0.18.1.** Entire bevy_ui src searched; no matches. Workaround: spawn a slightly-larger dark sibling node behind the panel, or omit in MVP. |
| `text-shadow` on `Text`           | ⚠️      | No built-in equivalent. Workaround: spawn a second text entity behind the first with shadow color and a small pixel offset.                                         |

### Animations / Transform on UI nodes

| Claim                                               | Status | Notes                                                                                                   |
| --------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------- |
| `Transform::translation.y` to bob NPC sprite        | ✅      | Respected for visual offset without affecting layout                                                    |
| `Transform::scale.x` for shadow breathe             | ✅      | Visually scales rendering without reflowing layout (correct for absolute-positioned decorative nodes)   |
| Mutating `Node.top: Val::Px(n)` to animate position | ✅      | Works but triggers layout recompute each frame; prefer `Transform::translation` for pure visual offsets |

### Pixel-art sampler (NPC / backdrop)

`ImageSamplerDescriptor::nearest` cannot be set via `bevy_asset_loader`
`#[asset(...)]` attribute syntax in 0.26. Set it after loading in a
one-shot system on `OnEnter(GameState::Ready)`:

```rust
fn set_nearest_samplers(
    mut images: ResMut<Assets<Image>>,
    game_assets: Res<GameAssets>,
) {
    for handle in [&game_assets.backdrop, &game_assets.goblin, &game_assets.robed] {
        if let Some(img) = images.get_mut(handle) {
            img.sampler = ImageSampler::nearest();
        }
    }
}
```

### Dashed borders

CSS `border-style: dashed` has **no equivalent** in Bevy 0.18 UI — all
`Node` borders are solid. The Book of Acting spell separator and Binding
TODO banner should use a faint solid `BorderColor` instead.

### Inherited opacity

Bevy 0.18 UI has **no cascading `opacity` property**. Alpha must be set
individually on each `TextColor`, `BackgroundColor`, and `ImageNode.color`.
See §3.3 for the ledger row fade workaround.
