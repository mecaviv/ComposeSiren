# Clic UI layout → JUCE port notes

Source: `clic-ui-layout.json` (user-exported from `clic-ui-mockup.html`, 2026-10-03).
Design rules: `MEMORY-clic-metronome.md` + enable-override semantics.

## Behaviour (must implement exactly)

- **`ClicEnable`** is an independent override. It does **not** write `ClicVolume`.
  - OFF → effective gain **0** (knob position ignored for audio)
  - ON → effective gain = `ClicVolume`
- **Pan UI: Spread + Bias knobs** (user-confirmed layout). **No ring, no yoke rail.**
  - Small white `KnobLAF` knobs flanking Decay/Vol at the user-exported positions.
  - Params: `ClicSpread` (−1..1), `ClicBias` (−1..1).
- Only **Vol** is `NotchedKnobLAF` (black), ~+10%; Decay / Spread / Bias are white `KnobLAF`.
- Header: left-aligned `[ ]` white-square switch + label, full-width underline.
- Switch is **override only** (does not write `ClicVolume`).
- **Master block: no "Gain" knob label**.

## Bottom band rects (from export)

Window content width ~923–930. Band height **158**.

| Piece | x | y | w | h |
|---|---|---|---|---|
| Reverb strip | 2 | 0 | **653** | 83 |
| Clic pane | **662** | 1 | **136** | 158 |
| Master strip | **804** | 1 | **119** | 157 |
| Keyboard | 2 | 88 | 653 | 68 |

Inside Clic pane (local):

| Control | x | y | w | h |
|---|---|---|---|---|
| header `[ ]` + "Clic" + underline | — | 0 | full | 26 (+10 gap) |
| Vol | 36 | 64 | 64 | 64 |
| Decay | 48 | 0 | 44 | 52 |
| **Spread** | 2 | 20 | 34 | 44 |
| **Bias** | 96 | 20 | 34 | 44 |

`clicBorrow` (reverb+keyboard shrink) ≈ **136** + gap; Master is **narrower** than the non-clic 172 (user tightened it to 119).

## Files changed (JUCE, 2026-10-03)

Implemented on `ComposeSiren-clic` / `feat/clic-volume-pan` (uncommitted):

1. `parameterDefinitions.{h,cpp}` — **`ClicEnable`** (0/1, default 1), group `"C"`.
2. `PluginProcessor.{h,cpp}` — `renderClic`: `vol = ClicEnable ? ClicVolume : 0`.
3. `ClicPane.h` — rewritten: header `[ ]` + engraved `Clic` + **full-width rule**; Spread/Bias (30×42 white) < Decay (44×52) < Vol (58×58 black). **No RingPad.** Enable = `ButtonAttachment` on `ClicEnable` only.
4. `MasterVolumeComponent.*` — embedded Clic block **removed**.
5. `EngravedTitle.h` — left-aligned text, full-width underline.
6. `PluginEditor.cpp` — `clicBorrow = 136`, single Clic pane.

Build: `cmake --build cmake-build-universal --target SirenOrchestra_Standalone` — **green**.

1. `parameterDefinitions.*` — add **`ClicEnable`** (Bool, group `"C"`). Keep `ClicVolume` as the knob.
2. `PluginProcessor::renderClic` — `gain = ClicEnable ? ClicVolume : 0`.
3. `ClicPane.h` — independent enable (not `applyEnable` → volume); **Spread/Bias knobs, no RingPad**; adopt local rects above.
4. `MasterVolumeComponent.*` — **remove** the embedded Clic block (`clicAreaWidth=116`).
5. `PluginEditor::resized` — single Clic pane; `clicBorrow≈136`; master width per export; drop double UI.
6. Match strip chassis (`bottomColour` / `#4650c8`) and whitesmoke knobs to Reverb.

## Mockup

- Open: `design/clic-ui-mockup.html`
- Edit mode: drag / resize (corner handles), double-click labels to rename
- Toolbar: Spread/Bias knobs always on; densified reverb; visual clic loop
- **Copy layout JSON** re-exports after tuning
