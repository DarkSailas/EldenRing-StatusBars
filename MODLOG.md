# MODLOG

## Target

- Elden Ring 1.17.1, `eldenring.exe` 2.7.1.0, The Convergence, me3 0.13.0, Seamless Co-op.
- Idea: a better-looking player panel (HP, FP, stamina) in a dark fantasy style (blackened iron, dragon horns, a dragon's eye, runes), with the equipped Great Rune shown by the eye in a medallion and the character level under it.
- Done means: works in the game. Current state (2026-10-04): 0.1.0 loads, the panel is drawn over the game's bars with real values, the medallion is empty without a Great Rune, the level is shown. 0.2.0 (the gold look, no show delay) was deployed and its look rejected. 0.3.0 (iron, dragon and rune look) is built and deployed and has not been seen in the game.

## Route

Native DLL loaded by me3 (`[[natives]]`). Rust cdylib, no game files changed.

- Game structures: `fromsoftware-rs` rev `59fbd3b3b7daaf14aca47c9f73530493dba6bc79` (`eldenring`, `fromsoftware-shared`).
- Overlay: hudhook 0.9.3, `dx12` feature, imgui 0.12. Everything is drawn with the background draw list from polygons; no textures.
- Game logic runs as a recurring task in `CSTaskGroupIndex::ChrIns_PostPhysics`. The task handle is leaked on purpose, dropping it unregisters the task.
- Rejected: replacing the HUD layout (`menu/*.gfx`, `01_common.tpf.dcx`). Convergence ships its own atlas there, and an earlier atlas edit broke spell icons.

## What is read

| Value | Source |
|---|---|
| HP, FP, stamina and their maximums | `WorldChrMan.main_player.chr_ins.modules.data` |
| Name, level | `player_game_data.character_name`, `.level` |
| Great Rune slot | `player_game_data.equipment.equip_item_data.great_rune`: `gaitem_handle` read as `u32`, `index` |
| Rune Arc | `player_game_data.rune_arc_active` |
| Interface state | `CSFeManImp.hud_state` |
| Screen fades | `CSFade.fade_plates`, nine pointers, `current_color.a` |

Rune slot without a rune, seen in the log: `handle 0x00000000 index -1`. The mod treats the slot as filled when the handle is neither 0 nor `0xFFFFFFFF` and the index is not negative. The filled case has not been seen.

## Visibility

First build showed the panel as soon as a player object existed, which is before the game's own bars come in. Current rule:

1. a player exists;
2. `hud_state == Default` (`HideInMenus`, on by default). `ShowAll` is the esc menu and `PopupMenu` a popup; with the key off the rule is only `hud_state != HideAll` and the panel stays up in menus;
3. no fade plate has alpha above 0.5 (`HideDuringFade`);
4. all of that held for `ShowDelay` seconds (0 by default since 0.2.0; it was 1).

With a 1 s delay and a slow fade-in the game's own bars were seen first and the panel covered them late. Since 0.2.0 the panel comes in and leaves within a few frames (`FADE_IN` 14, `FADE_OUT` 16). A plate that stays dark for 20 s is ignored until it clears, so a stuck plate cannot hide the panel for good.

There is no player object on the title screen, so the panel cannot show at launch. The one time it did, it was the `Preview` test mode (gotcha 2).

Log of a normal load: `hud state Default`, `fade plate 2 covers the screen, alpha 1.00`, about 4.4 s later `fade cleared`, one second later `panel shown` (0.1.0, with the 1 s delay).

## Style

The first look (pointed bar ends, diamonds on the medallion, tick marks, a gold ring, a glowing tip, a name plate) was rejected as too ornate and not dark enough. Current look: near-black plates, a 2 px dark steel frame, square bar ends, dull red / steel blue / olive fills with a vertical gradient, grey for the part just lost, the name as plain text, a black medallion in one ring, dim bone-coloured text. The frame of the HP bar turns dark red below a quarter.

That look (0.1.0) was rejected in turn: too dark, the medallion and the bars could not be made out in the game. 0.2.0 goes back to an ornate look, this time with legible colours: a warm black plate with a pointed end, gold bar frames with a gloss line, bright red / blue / green gradient fills, pale gold for the part just lost, a gold lozenge with a stone at the end of each bar, the name between two gold rules that end in spiral scrolls, a medallion in a beaded gold ring with star rays, a hexagonal level plaque. The sign in the medallion is dim bronze, gold, or gold with a glow.

The gold look was rejected on 2026-10-05 as gaudy; what was wanted is dragons and dark magic, with no gold anywhere. 0.3.0: blackened iron frames with a light top edge, a barbed blade with a stone at the end of each bar, FP in violet, grey for the part just lost. The rules around the name end like a dragon's tail (a damped wave and a barbed tip; the lower one has spines). The medallion is an iron shield with two horns, spikes and ten runes on its ring, and a dragon's eye with a slit pupil in the middle. Without a Great Rune the eye is ashen and the runes grey; with one equipped the eye burns orange and the runes are violet; with a Rune Arc the eye blazes yellow, the runes glow and a violet halo sits behind the shield. The level hangs under it on a pointed iron scale. Horns are built from quads along a curve, since imgui fills only convex shapes.

Drawing lives in `src/panel.rs` behind the `Surface` trait; `src/overlay.rs` only feeds it the imgui draw list.

The test `every_filled_shape_is_convex` draws three sample states to an SVG surface, checks every filled polygon for convexity (imgui fills only convex shapes) and writes `%TEMP%/er_status_bars_preview.svg`. `docs/preview.png` is that file rendered by a browser.

## Layout (pixels of 1920x1080)

- Bars start at x 150; since 0.2.0 the frame starts at x 118 and runs under the medallion, and the plate ends 25 px past the longest bar. Rows: HP 45 to 58, FP 64 to 77, stamina 83 to 96. With a 2 px frame the rows keep a 2 px gap; at a 4 px distance the frames touched and the FP frame was painted over.
- Length = maximum value × pixels per point (0.4 / 1.75 / 3.0), between 60 and 900. Checked on one character: 990 HP, 97 FP, 110 stamina.
- Medallion centre (91, 70), radius 44. Level plate under it. The horns reach y 4, four pixels from the top of the screen at the default position.

## Gotchas

1. Two hudhook overlays starting together crash the game in `sl.interposer.dll` (Streamline, shipped with ERSS-FG): each one probes Direct3D with a throwaway device and patches the same swap chain functions. Fix: a named mutex `Local\er_overlay_hook_init` around `Hudhook::apply()`, held 500 ms longer. `er_ping_marker` uses the same name.
2. The `Preview` key drew made-up values ("Tarnished", level 150) whenever the real panel was hidden, the title screen included. It looked like the panel showing at launch with wrong numbers. The key and its code are removed.
3. Empty fade plate slots are null pointers; they are read as raw pointers and skipped.
4. One start ended in an access violation in `damage_competition.dll_unloaded` (a third-party native in the same profile) before this mod wrote its log. The next start was clean. Not reproduced.
5. A loaded DLL cannot be overwritten. Rename it, copy the new one, restart the game.

## Deployment on the dev machine

- DLL and ini: `D:\Games\ConvergenceER\mod\dll\er_status_bars.dll`, `er_status_bars.ini`.
- Profiles `convergence.me3` and `convergence - seamless.me3` got a `[[natives]]` entry. Copies from before the change: `D:\Games\ConvergenceER\_fds_backup\statusbars_2026-10-04\`.
- Undo: restore both `.me3` files from that folder, or delete the added lines.
- 0.2.0 replaced the DLL; the 0.1.0 build is kept there as `er_status_bars_0.1.0.dll`. The deployed ini got `ShowDelay = 0`.
- 0.3.0 replaced the DLL on 2026-10-05; the 0.2.0 build is kept there as `er_status_bars_0.2.0.dll`.

## Not verified

1. The eye and the runes with a Great Rune equipped, and lit with a Rune Arc.
2. Cutscenes, death screen, fast travel: whether the fade rule hides the panel at the right moments.
3. The 0.3.0 look in the game: the font, the row spacing with the 2.5 px frames, the blades against the game's own bar ends, the horns at the top edge of the screen. Only the SVG preview has been looked at.
4. Resolutions other than 1920x1080 and non-16:9 pictures.
5. Characters with much longer bars than the one tested.
6. `HideInMenus`: that the panel leaves in the esc menu, the map, the inventory and at a grace, and that nothing in plain play reports a state other than `Default`.
7. That with `ShowDelay = 0` the panel is up before the game's own bars are seen, after a load and after closing a menu.
