# Bevy Redesign — Living TODO List

Companion to `BEVY_REDESIGN_PLAN.md`. Tracks concrete tasks in rough
dependency order. Check items off as they are completed.

> **Terminology**: "Acting" phase is now called **Combat** phase throughout
> all new code. The **Reacting** phase has been removed from the game.
> Existing code that references `Acting`/`Reacting` should be renamed as
> each area is touched.

---

## 1. Core Resources

- [x] Add color constants module (`src/ui/palette.rs`) with all `PARCHMENT`,
      `GOLD`, `BLOOD`, `NIGHT`, etc. constants (see §0.3 of plan)
- [x] Add `BattleUiClock` resource (`src/ui/clock.rs`) with `elapsed: f32`
      and a `tick_clock` system running in `Update` while `GameState::Ready`
- [x] Add health state: `PlayerCombatState { hp, max }` as a `Resource`
      (`src/ui/health.rs`); `NpcCombatState { hp, max }` as a `Component`
      attached to the NPC sprite entity — stub with fixed values initially
- [x] Register all new fonts in `GameAssets` (`src/lib.rs`) via
      `bevy_asset_loader` `#[asset(...)]` fields (CormorantUnicase
      SemiBold/Bold, CormorantGaramond Italic variable, IM Fell DW Pica SC,
      UnifrakturMaguntia Regular)
- [x] Register image assets in `GameAssets` (parchment_tile, corner_bracket,
      vignette, sigils + sigils_layout)
- [x] Add a `set_nearest_samplers` startup system (runs
      `OnEnter(GameState::Ready)`) in `src/ui/samplers.rs` that sets
      `ImageSampler::nearest()` on backdrop, goblin, and robed handles

---

## 2. Overall Layout

- [x] Create `src/ui/mod.rs` with `pub fn configure_ui(app: &mut App)`
      (renamed from `configure_hud` to avoid churn) and call it from `lib.rs`
- [x] Spawn outer centering wrapper + `BattleHudRoot` 3×3 grid node on
      `OnEnter(GameState::Ready)` (16:9 aspect-ratio locked, columns
      `[22fr, 50fr, 22fr]`, rows `[auto, 1fr, auto]`)
- [x] Spawn placeholder children for each grid cell so layout can be
      verified visually before individual panels are built:
      `CombatBar` (row 1, span 3), `LeftColumn` (row 2 col 1),
      `ArenaPanel` (row 2 col 2), `BookPanel` (row 2 col 3),
      `BindingPanel` (row 3, span 3)
- [x] Set `ClearColor` to `NIGHT` (`#0f0a07`) replacing current blue
- [ ] Spawn fullscreen vignette overlay (`ImageNode` with `color` alpha 0.4,
      `NodeImageMode::Stretch`, high `ZIndex`) once vignette asset exists

---

## 3. Rune Keyboard (Left Column — Lower)

> Mostly relocating existing code; the logic itself does not change.
> Keyboard widget code was moved out of `src/futhark.rs` into a dedicated
> `src/ui/keyboard.rs` module. `futhark.rs` re-exports the public API so
> existing callers (game code + UATs) keep working without churn.

- [x] Wrap existing keyboard spawn in a `KeyboardPanel` node that is a flex
      child of `LeftColumn` (percent-sized, scales to column width) instead
      of absolute top-left positioning
- [x] Replace pixel-absolute key positions with percent-relative
      `padding_left` on each key row, preserving the existing row offsets
      as proportions of panel width. Key sizes are also percent-based
      (keys 8% wide, wide keys 13.3%, square via `aspect_ratio`).
- [x] Add panel header row: "Rune Keyboard" (Cormorant Unicase SemiBold,
      GOLD_LIGHT) + "tab · legend" aside (IM Fell SC italic, PARCHMENT_DARK)
- [x] Verify `FutharkKeyBackground`, `FutharkKeyRuneVisual`,
      `FutharkKeyLetterVisual` components still work after reparenting
- [x] Run `uat_shows_futhark_rune` and `uat_shows_rune_word_navigation`
      and confirm zero exit

---

## 4. Left Inscriptions Area

> The existing absolute-pixel rune-word lane is replaced entirely.
> Most existing game logic (RuneSlot, RowLetterGraded, RuneMatchState)
> is kept; only the layout entities change.

- [x] Create `src/ui/inscribed.rs` with `pub fn configure_inscribed(app)`
- [x] Spawn `InscribedPanel` as a flex-col child of `LeftColumn`
      (flex: 1, takes remaining height above keyboard)
- [x] Spawn `ActiveAttemptCard` inside `InscribedPanel`:
      - `INSCRIBING` floating label (absolute, top edge, BLOOD_BRIGHT)
      - Rune display row wired to existing `RuneSlot` entities
      - Removed blinking caret from the active row presentation
- [x] Spawn divider row between active card and ledger
- [x] Spawn `LedgerList` (flex-col, flex: 1, up to 4 `AttemptRow` entries):
      - Per-row: index numeral + tiles row (color from `RuneMatchState`) +
        word subtitle (known/unknown states)
      - Oldest row fade: walk children, set `TextColor` alpha and
        `ImageNode.color` alpha to 0.55 (no inherited opacity in Bevy)
- [x] Wire `RowLetterGraded` events → populate `AttemptRow` tile colors
- [x] Remove old absolute `Val::Px` layout constants (`ACTIVE_ROW_TOP`,
      `ROW_LEFT`, `SLOT_SPACING`, `SLOT_SIZE`) — rename to `_LEGACY_` first,
      remove once UATs pass
- [x] Run `uat_shows_rune_slots`, `uat_shows_loading_rune_reveal`,
      `uat_shows_typed_futhark_rune`, and `uat_battle_stages` — confirm zero

---

## 5. Central Combat Area

> Replaces the current 256×256 absolute top-right `CombatScene`.

- [x] Create `src/ui/arena.rs` with `pub fn configure_arena(app)`
- [x] Spawn `ArenaPanel` in grid row 2 col 2:
      - `ImageNode` backdrop (`NodeImageMode::Stretch`, nearest sampler)
      - `BorderColor` all sides GOLD, 1px
- [x] Spawn 4 `CornerBracket` child nodes (absolute, GOLD bars + diamond pip)
      — use bracket PNG if asset exists, otherwise two-bar Node approach
- [x] Spawn `PhaseMark` pill (absolute, top-left of arena):
      - Pulsing dot driven by `BattleUiClock`
      - Text updated by `sync_phase_mark` system reading `BattleState.phase`
      - **Phase name should read "Combat" not "Acting"**
- [x] Move NPC sprite logic from `src/combat.rs` into `ArenaPanel`:
      - Center sprite with percent-based sizing (~22% of arena)
      - Bob animation driven by `BattleUiClock`
      - Ground shadow ellipse with breathe animation
- [x] Remove old `CombatScene` / `spawn_combat_scene` absolute layout from
      `src/combat.rs` once `ArenaPanel` is live
- [x] Run `uat_battle_stages` — confirm zero exit

---

## 6. Health Bars

> New; no equivalent exists currently.

- [x] Spawn `CombatBar` (grid row 1, span 3) with three-column inner grid
      `[1fr, auto, 1fr]`
- [x] Spawn player `CombatantBlock` (left, flex-row):
      - No portraits
      - Combatant name text (Cormorant Unicase SemiBold, GOLD_LIGHT)
      - Player `HpBarNode` (see below)
- [x] Spawn enemy `CombatantBlock` (right, `FlexDirection::RowReverse`):
      - Same structure mirrored; HP fill anchors from the right
- [x] Spawn `HpBarNode` for each side:
      - Outer node: `Overflow::clip()`, 1px GOLD_DARK border
      - Fill node (absolute): width driven by the relevant health state
        (player fill reads `PlayerCombatState`; enemy fill reads the
        spawned NPC's `NpcCombatState`)
      - Tick overlay: 10 flex-row children with divider borders
      - HP text label (absolute, centered): IM Fell SC, PARCHMENT color
- [x] Spawn `PhaseBannerNode` (center column of combat bar):
      - "current phase" subtitle (IM Fell SC, PARCHMENT_DARK)
      - Phase name text (Cormorant Unicase Bold, GOLD_LIGHT)
        — reads "Combat" for the combat phase
      - 3 pip row (inactive/active driven by phase index)
- [x] Add `sync_hp_bars` system: reads `PlayerCombatState` and the active
      NPC's `NpcCombatState`, updates fill node widths
- [x] Add `sync_phase_banner` system: reads `BattleState.phase`, updates
      text and pip colors

---

## 7. Book of Combat (right column)

> Replaces `ActingBookPanel` in `src/rune_words/battle_states/acting.rs`.
> Renamed from "Book of Acting" → **"Book of Combat"**.
> Rules for spell selection will change soon; keep data wiring minimal.

- [ ] Create `src/ui/book.rs` with `pub fn configure_book(app)`
- [ ] Spawn `BookPanel` in grid row 2 col 3 using `spawn_leather_panel`
      helper (§8.1 of plan):
      - Header: "Book of Combat" (Cormorant Unicase, GOLD_LIGHT)
      - "choose · inscribe" aside (IM Fell SC italic, PARCHMENT_DARK)
- [ ] Spawn `BookPage` inner parchment node (flat `PARCHMENT_WARM`
      `BackgroundColor` until parchment tile asset exists; swap to
      `NodeImageMode::Tiled` once asset is ready):
      - Page head rule text
      - Red bookmark tab (absolute, top edge, BLOOD)
- [ ] Spawn 4 `SpellEntry` nodes (3-column grid: dropcap / content / sigil):
      - Dropcap: UnifrakturMaguntia, BLOOD
      - Word text: Cormorant Unicase Bold, INK, uppercase
      - Rune display row (futhark sprites)
      - Sigil circle placeholder (solid ring border, Node; swap to sigil
        atlas `ImageNode` once `sigils.png` asset exists)
- [ ] **TODO**: Wire `SpellEntry` to combat phase data — exact data source
      TBD when combat rules are finalised
- [ ] **TODO**: Active spell highlight (BLOOD left border, ember background
      tint, pulsing `☛` pointer) — implement once selection logic is defined
- [ ] Remove `spawn_acting_book_panel`, `ActingBookPanel`,
      `ActingBookEntry`, `ActingBookEntryBackground` from
      `src/rune_words/battle_states/acting.rs` once `BookPanel` is live
- [ ] Remove `Reacting`-related state files and systems
      (`src/rune_words/battle_states/reacting.rs`) — the Reacting phase
      no longer exists
- [ ] Run `uat_shows_acting_battle_state` (rename to
      `uat_shows_combat_battle_state` when convenient) — confirm zero exit

---

## 8. Binding Strain Panel

> **Rules not yet defined.** Only the stub panel is in scope now.

- [ ] Spawn `BindingPanel` in grid row 3, span 3:
      - `CombatBar`-style dark background, GOLD_DARK border, full-width
      - TODO banner: "TODO" badge (EMBER background, NIGHT text) +
        "Binding Strain rules not yet defined." message (Cormorant
        Garamond italic, PARCHMENT)
- [ ] **TODO (design gate)**: Replace stub with real chain-link layout once
      Binding rules are decided
- [ ] **TODO (design gate)**: Intact / strained / broken link states and
      animations
- [ ] **TODO (design gate)**: Three-column count / chain / title layout
