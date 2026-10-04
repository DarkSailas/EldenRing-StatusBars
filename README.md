# Elden Ring Status Bars

A native DLL for Elden Ring 1.17.1 (`eldenring.exe` 2.7.1.0) that redraws the player's HP, FP and stamina panel in a plain, near-black style:

- three square-ended bars in a thin dark steel frame, with dull red, blue and olive fills; the part just lost stays visible for a moment and then shrinks;
- current and maximum values next to each bar;
- the character name above the bars;
- a round black medallion in one steel ring on the left. It shows a rune sign when a Great Rune is equipped, paler while a Rune Arc is active, and stays empty otherwise;
- the character level on a small plate under the medallion.

There is no gold, no ornaments and no glow. The mod changes no game files. It draws over the game's own bars and covers them with a dark plate.

> Status: version 0.1.0 runs in the game with The Convergence and Seamless Co-op. Confirmed there with an earlier, more ornate look: loading, the empty medallion, the level. Not confirmed: the current look, hiding in menus, the rune sign with a Great Rune equipped, behaviour in cutscenes and on other resolutions than 1920x1080. See "Known limits".

## Install

The mod is loaded as a native by [me3](https://github.com/garyttierney/me3) or by Mod Engine 2.

1. Copy `release/er_status_bars.dll` next to your other native mods, for example `mod/dll/`.
2. Add it to the profile.

   me3 (`*.me3`):

   ```toml
   [[natives]]
   path = './../mod/dll/er_status_bars.dll'
   ```

   Mod Engine 2 (`config_eldenring.toml`):

   ```toml
   external_dlls = ["mod/dll/er_status_bars.dll"]
   ```

3. Start the game offline or through Seamless Co-op. Do not use the mod with Easy Anti-Cheat enabled.

On the first start the mod writes `er_status_bars.ini` and `er_status_bars.log` next to the DLL. There are no hotkeys and no settings window.

## Settings

`er_status_bars.ini` is re-read while the game runs, about a second after you save it. Lengths are in pixels of a 1920x1080 picture and scale with the resolution.

| Section | Key | Meaning |
|---|---|---|
| `[Panel]` | `Enabled` | Turn the panel off without removing the mod |
| | `Scale` | Panel size, 0.5 to 3 |
| | `OffsetX`, `OffsetY` | Shift of the panel |
| | `Opacity` | 0.1 to 1 |
| | `Backing` | Dark plate behind the bars. It is what hides the game's own bars |
| | `ShowName`, `ShowNumbers`, `ShowLevel` | Name, values, level plate |
| | `ShowDelay` | Seconds the game's interface has to stay up before the panel comes in, 0 to 10 |
| | `HideDuringFade` | Keep the panel away while the game fades the screen. Set to 0 if the panel never appears |
| | `HideInMenus` | Hide the panel while a game menu or a popup is open. Set to 0 if the panel goes missing during play |
| `[Bars]` | `HpPixelsPerPoint`, `FpPixelsPerPoint`, `StaminaPixelsPerPoint` | Bar length per point of the maximum value |
| | `MinLength`, `MaxLength` | Limits of the bar length |
| `[Debug]` | `Log` | Write `er_status_bars.log` |

If a game bar shows from under the panel on the right, raise the matching `PixelsPerPoint` value.

## How it works

A recurring game task reads the player's values from the game structures ([fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs)): HP, FP, stamina, name, level, the Great Rune slot and the Rune Arc flag. An ImGui overlay on the DirectX 12 swap chain ([hudhook](https://github.com/veeenu/hudhook)) draws the panel from plain shapes, without textures.

The panel is shown when there is a player character, the game's interface is in its plain play state (no menu, no popup) and no fade plate covers the screen. It then waits `ShowDelay` seconds and fades in; it leaves at once when any of the three stops being true. There is no player character on the title screen, so the panel cannot appear there.

## Known limits

- The game's own bars are covered, not removed. A bar longer than the panel's shows from under it; tune `[Bars]`.
- Positions follow the default interface layout at 16:9. On ultrawide the panel is shifted like the game's interface, which has not been tried.
- The rune sign is the mod's own drawing and is the same for every Great Rune. It has not been seen in the game yet.
- Status effect icons, buffs and the boss bar are left to the game.
- Several overlays hooking the same swap chain can conflict. This mod and [er_ping_marker](https://github.com/DarkSailas) take turns at start through a named mutex; mods that do not know about it may still collide. If the game does not start with this DLL, remove it from the profile and check the log.
- Built for 1.17.1. Other game versions may shift the structures the mod reads.

## Build

Rust with the `x86_64-pc-windows-gnu` toolchain and MinGW-w64 on `PATH`:

```
cargo test --release
cargo build --release
```

The DLL appears in `target/x86_64-pc-windows-gnu/release/`. `.cargo/config.toml` links the C++ runtime statically, so the DLL needs nothing beyond system libraries.

## License

MIT, see `LICENSE`.

---

# Панель здоровья, маны и выносливости (по-русски)

Нативная DLL для Elden Ring 1.17.1. Перерисовывает панель игрока в строгом, почти чёрном стиле:

- три прямоугольные полосы в тонкой рамке из тёмной стали, с тусклыми красным, синим и оливковым цветами; потерянная часть на мгновение остаётся видна и затем убывает;
- текущее и максимальное значение рядом с каждой полосой;
- имя персонажа над полосами;
- круглый чёрный медальон в одном стальном кольце слева: знак руны, когда надета Великая руна (светлее, пока действует Рунная дуга), иначе пусто;
- уровень персонажа на плашке под медальоном.

Золота, украшений и свечения нет. Файлы игры мод не меняет: он рисует поверх штатных полос и закрывает их тёмной подложкой.

> Состояние: версия 0.1.0 работает в игре с The Convergence и Seamless Co-op. Подтверждено на прежнем, более нарядном оформлении: загрузка, пустой медальон, уровень. Не подтверждено: нынешний вид, скрытие в меню, знак руны с надетой Великой руной, поведение в роликах и на разрешениях, отличных от 1920x1080.

## Установка

1. Скопируйте `release/er_status_bars.dll` к остальным нативным модам, например в `mod/dll/`.
2. Добавьте в профиль me3:

   ```toml
   [[natives]]
   path = './../mod/dll/er_status_bars.dll'
   ```

3. Запускайте игру без Easy Anti-Cheat: офлайн или через Seamless Co-op.

При первом запуске рядом с DLL появятся `er_status_bars.ini` и `er_status_bars.log`. Горячих клавиш и окна настроек у мода нет.

## Настройки

Файл `er_status_bars.ini` перечитывается на ходу, примерно через секунду после сохранения. Длины заданы в пикселях картинки 1920x1080.

- `Scale`, `OffsetX`, `OffsetY`, `Opacity` — размер, сдвиг и прозрачность панели.
- `Backing` — подложка, которая закрывает штатные полосы.
- `ShowName`, `ShowNumbers`, `ShowLevel` — имя, числа, уровень.
- `ShowDelay` — сколько секунд интерфейс игры должен быть на экране, прежде чем появится панель.
- `HideDuringFade` — прятать панель, пока игра затемняет экран. Поставьте 0, если панель не появляется совсем.
- `HideInMenus` — прятать панель, пока открыто меню игры или всплывающее окно. Поставьте 0, если панель пропадает во время игры.
- `HpPixelsPerPoint`, `FpPixelsPerPoint`, `StaminaPixelsPerPoint`, `MinLength`, `MaxLength` — длина полос. Если штатная полоса торчит из-под панели справа, увеличьте соответствующее значение.

Панель видна только в самой игре: на титульном экране персонажа ещё нет, в меню и на экранах загрузки она скрыта.

## Ограничения

- Штатные полосы закрыты, а не убраны. Полоса длиннее панельной видна из-под неё.
- Расположение рассчитано на обычный интерфейс 16:9; на сверхшироких экранах не проверялось.
- Знак руны нарисован самим модом и одинаков для всех Великих рун. В игре его пока не видели.
- Значки эффектов и полосу босса рисует игра.
- Несколько оверлеев на одной цепочке кадров могут конфликтовать. Если игра не запускается, уберите DLL из профиля и посмотрите лог.
