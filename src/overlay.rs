use std::{
    f32::consts::PI,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::atomic::Ordering,
};

use hudhook::{
    ImguiRenderLoop, MessageFilter, RenderContext,
    imgui::{Context, DrawListMut, FontConfig, FontGlyphRanges, FontId, FontSource, Io, Ui},
    windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CYSCREEN},
};

use crate::{
    config::Config,
    logf,
    state::{DISABLED, Stats, shared},
};

type Rgb = [f32; 3];

/// Near-black plates in a plain dark steel frame; the fills are dull and nothing shines.
const IRON: Rgb = [0.17, 0.17, 0.175];
const IRON_DARK: Rgb = [0.06, 0.06, 0.065];
const PLATE: Rgb = [0.015, 0.015, 0.018];
const TROUGH: Rgb = [0.03, 0.03, 0.032];
const TEXT: Rgb = [0.58, 0.57, 0.54];

const HP_TOP: Rgb = [0.36, 0.05, 0.05];
const HP_BOTTOM: Rgb = [0.15, 0.02, 0.025];
const FP_TOP: Rgb = [0.13, 0.19, 0.30];
const FP_BOTTOM: Rgb = [0.05, 0.08, 0.15];
const STAMINA_TOP: Rgb = [0.22, 0.25, 0.15];
const STAMINA_BOTTOM: Rgb = [0.09, 0.11, 0.07];
const LAG: Rgb = [0.38, 0.36, 0.33];
const DANGER: Rgb = [0.45, 0.05, 0.04];
const RUNE_LIT: Rgb = [0.72, 0.67, 0.55];
const RUNE_DIM: Rgb = [0.27, 0.26, 0.25];

/// Layout in pixels of a 1920x1080 picture; the rows sit on the game's own bars.
const BARS_X: f32 = 150.0;
const HP_ROW: (f32, f32) = (45.0, 58.0);
const FP_ROW: (f32, f32) = (64.0, 77.0);
const STAMINA_ROW: (f32, f32) = (83.0, 96.0);
const MEDALLION: (f32, f32, f32) = (91.0, 70.0, 44.0);
const FRAME: f32 = 2.0;
/// How far the dark backing reaches past a bar; it is what covers the game's own bar.
const BACKING: f32 = 5.0;
const BANDS: usize = 6;

/// Seconds a bar keeps showing the lost part before it starts to shrink.
const LAG_HOLD: f32 = 0.45;
/// Share of the bar the lost part shrinks by in a second.
const LAG_SPEED: f32 = 0.7;
/// The panel comes in slowly, like the game's own bars, and leaves at once.
const FADE_IN: f32 = 2.5;
const FADE_OUT: f32 = 16.0;
const LOW_HP: f32 = 0.25;

/// Font sizes in the order they are added to the atlas; the constants below index it.
const FONT_SIZES: [f32; 2] = [15.0, 19.0];
const FONT_SMALL: usize = 0;
const FONT_NAME: usize = 1;

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Value shown with a delay after a loss, as a share of the maximum.
#[derive(Default)]
struct Lag {
    shown: f32,
    hold: f32,
}

impl Lag {
    fn update(&mut self, value: f32, dt: f32) -> f32 {
        if value >= self.shown {
            self.shown = value;
            self.hold = LAG_HOLD;
        } else if self.hold > 0.0 {
            self.hold -= dt;
        } else {
            self.shown = (self.shown - LAG_SPEED * dt).max(value);
        }
        self.shown
    }
}

struct Bar {
    row: (f32, f32),
    length: f32,
    value: f32,
    lagged: f32,
    top: Rgb,
    bottom: Rgb,
    danger: bool,
}

/// Draws in 1920x1080 units at the panel position.
struct Canvas<'a> {
    list: DrawListMut<'a>,
    origin: [f32; 2],
    scale: f32,
    alpha: f32,
}

impl Canvas<'_> {
    fn at(&self, x: f32, y: f32) -> [f32; 2] {
        [self.origin[0] + x * self.scale, self.origin[1] + y * self.scale]
    }

    fn color(&self, rgb: Rgb, alpha: f32) -> [f32; 4] {
        [rgb[0], rgb[1], rgb[2], alpha * self.alpha]
    }

    fn fill(&self, points: &[(f32, f32)], rgb: Rgb, alpha: f32) {
        let points: Vec<[f32; 2]> = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.list.add_polyline(points, self.color(rgb, alpha)).filled(true).build();
    }

    fn outline(&self, points: &[(f32, f32)], rgb: Rgb, alpha: f32, thickness: f32) {
        let mut points: Vec<[f32; 2]> = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        if let Some(&first) = points.first() {
            points.push(first);
        }
        self.list
            .add_polyline(points, self.color(rgb, alpha))
            .thickness(thickness * self.scale)
            .build();
    }

    fn line(&self, from: (f32, f32), to: (f32, f32), rgb: Rgb, alpha: f32, thickness: f32) {
        self.list
            .add_line(self.at(from.0, from.1), self.at(to.0, to.1), self.color(rgb, alpha))
            .thickness(thickness * self.scale)
            .build();
    }

    fn disc(&self, x: f32, y: f32, radius: f32, rgb: Rgb, alpha: f32) {
        self.list
            .add_circle(self.at(x, y), radius * self.scale, self.color(rgb, alpha))
            .filled(true)
            .num_segments(48)
            .build();
    }

    fn ring(&self, x: f32, y: f32, radius: f32, rgb: Rgb, alpha: f32, thickness: f32) {
        self.list
            .add_circle(self.at(x, y), radius * self.scale, self.color(rgb, alpha))
            .num_segments(48)
            .thickness(thickness * self.scale)
            .build();
    }

    /// Text with a dark shadow; `anchor` is the share of the text size left of and above the point.
    fn text(&self, ui: &Ui, x: f32, y: f32, anchor: (f32, f32), rgb: Rgb, text: &str) {
        let size = ui.calc_text_size(text);
        let pos = self.at(x, y);
        let pos = [(pos[0] - size[0] * anchor.0).round(), (pos[1] - size[1] * anchor.1).round()];
        let shadow = self.scale.max(1.0);
        self.list.add_text([pos[0] + shadow, pos[1] + shadow], self.color([0.0; 3], 0.9), text);
        self.list.add_text(pos, self.color(rgb, 1.0), text);
    }

    /// Bar rectangle grown by `grow` on every side.
    fn bar_shape(&self, bar: &Bar, grow: f32) -> [(f32, f32); 4] {
        let (y0, y1) = bar.row;
        let (x0, x1) = (BARS_X - grow, BARS_X + bar.length + grow);
        [(x0, y0 - grow), (x1, y0 - grow), (x1, y1 + grow), (x0, y1 + grow)]
    }

    /// Fills the part of the bar between `from` and `to` (pixels from its start) with a vertical gradient.
    fn bar_span(&self, bar: &Bar, from: f32, to: f32, top: Rgb, bottom: Rgb, alpha: f32) {
        if to <= from {
            return;
        }
        let (y0, y1) = bar.row;
        let (x0, x1) = (BARS_X + from, BARS_X + to);
        for band in 0..BANDS {
            let ta = band as f32 / BANDS as f32;
            let tb = (band + 1) as f32 / BANDS as f32;
            let (ya, yb) = (y0 + (y1 - y0) * ta, y0 + (y1 - y0) * tb);
            let rgb = mix(top, bottom, (ta + tb) * 0.5);
            self.fill(&[(x0, ya), (x1, ya), (x1, yb), (x0, yb)], rgb, alpha);
        }
    }

    fn bar(&self, bar: &Bar) {
        let frame = if bar.danger { mix(IRON, DANGER, 0.7) } else { IRON };

        self.fill(&self.bar_shape(bar, FRAME), IRON_DARK, 1.0);
        self.fill(&self.bar_shape(bar, 0.0), TROUGH, 1.0);

        let filled = bar.length * bar.value;
        self.bar_span(bar, filled, bar.length * bar.lagged, LAG, mix(LAG, [0.0; 3], 0.5), 0.7);
        self.bar_span(bar, 0.0, filled, bar.top, bar.bottom, 1.0);

        self.outline(&self.bar_shape(bar, 0.0), [0.0; 3], 0.85, 1.0);
        self.outline(&self.bar_shape(bar, FRAME), frame, 1.0, 1.5);
    }

    /// Part of a circle between two angles, as a line.
    fn arc(&self, x: f32, y: f32, radius: f32, (from, to): (f32, f32), rgb: Rgb, alpha: f32) {
        let points: Vec<[f32; 2]> = (0..=16)
            .map(|k| from + (to - from) * k as f32 / 16.0)
            .map(|angle| self.at(x + angle.cos() * radius, y + angle.sin() * radius))
            .collect();
        self.list
            .add_polyline(points, self.color(rgb, alpha))
            .thickness(2.0 * self.scale)
            .build();
    }

    /// Sign of an equipped Great Rune in the medallion; paler while a Rune Arc powers it.
    fn rune(&self, lit: bool) {
        let (cx, cy, _) = MEDALLION;
        let rgb = if lit { RUNE_LIT } else { RUNE_DIM };
        self.line((cx, cy - 24.0), (cx, cy + 24.0), rgb, 1.0, 2.0);
        self.ring(cx, cy - 5.0, 12.0, rgb, 1.0, 2.0);
        self.arc(cx, cy - 22.0, 30.0, (PI * 0.28, PI * 0.72), rgb, 1.0);
        self.arc(cx, cy - 6.0, 22.0, (PI * 0.2, PI * 0.8), rgb, 1.0);
    }

    /// Small plate under the medallion with the character level.
    fn level_plate(&self, ui: &Ui, level: u32) {
        let text = level.to_string();
        let (cx, cy, r) = MEDALLION;
        let (y, half_h) = (cy + r + 9.0, 10.0);
        let half_w = (ui.calc_text_size(&text)[0] / self.scale * 0.5 + 9.0).max(17.0);
        let shape = [
            (cx - half_w, y - half_h),
            (cx + half_w, y - half_h),
            (cx + half_w, y + half_h),
            (cx - half_w, y + half_h),
        ];
        self.fill(&shape, PLATE, 1.0);
        self.outline(&shape, IRON, 1.0, 1.5);
        self.text(ui, cx, y, (0.5, 0.5), TEXT, &text);
    }

    /// A black disc in one heavy ring; it stays empty without a Great Rune.
    fn medallion(&self) {
        let (cx, cy, r) = MEDALLION;
        self.disc(cx, cy + 2.0, r + 3.0, [0.0; 3], 0.7);
        self.disc(cx, cy, r, IRON_DARK, 1.0);
        self.disc(cx, cy, r - 6.0, PLATE, 1.0);
        self.ring(cx, cy, r - 6.0, [0.0; 3], 0.9, 1.5);
        self.ring(cx, cy, r, IRON, 1.0, 1.5);
    }
}

fn share(value: f32, max: f32) -> f32 {
    if max > 0.0 { (value / max).clamp(0.0, 1.0) } else { 0.0 }
}

pub struct Overlay {
    font_scale: f32,
    fonts_loaded: bool,
    alpha: f32,
    hp: Lag,
    fp: Lag,
    stamina: Lag,
    last: Stats,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            font_scale: 1.0,
            fonts_loaded: false,
            alpha: 0.0,
            hp: Lag::default(),
            fp: Lag::default(),
            stamina: Lag::default(),
            last: Stats::default(),
        }
    }

    /// `FontId` is not `Send`, so the fonts are looked up in the atlas by index.
    fn font(&self, ui: &Ui, index: usize) -> Option<FontId> {
        if !self.fonts_loaded {
            return None;
        }
        ui.fonts().fonts().get(index).copied()
    }

    fn panel_scale(cfg: &Config, display: [f32; 2]) -> f32 {
        display[1] / 1080.0 * cfg.scale
    }

    fn frame(&mut self, ui: &Ui) {
        let (cfg, stats) = {
            let shared = shared();
            (shared.cfg.clone(), shared.stats.clone())
        };
        let dt = ui.io().delta_time.clamp(0.0, 0.1);

        let target = if cfg.enabled && stats.visible { 1.0 } else { 0.0 };
        let speed = if target > self.alpha { FADE_IN } else { FADE_OUT };
        self.alpha += (target - self.alpha) * (dt * speed).min(1.0);
        if stats.visible {
            self.last = stats;
        }
        if self.alpha < 0.01 {
            // Nothing is left of a bar's delayed part once the panel is gone.
            (self.hp.shown, self.fp.shown, self.stamina.shown) = (0.0, 0.0, 0.0);
            return;
        }
        let stats = &self.last;

        let display = ui.io().display_size;
        if display[1] < 1.0 {
            return;
        }
        let scale = Self::panel_scale(&cfg, display);
        let base = display[1] / 1080.0;
        // The game keeps its interface inside the middle 16:9 part of a wider picture.
        let side = ((display[0] - display[1] * 16.0 / 9.0) * 0.5).max(0.0);
        let canvas = Canvas {
            list: ui.get_background_draw_list(),
            origin: [side + cfg.offset_x * base, cfg.offset_y * base],
            scale,
            alpha: self.alpha * cfg.opacity,
        };

        let hp = share(stats.hp, stats.hp_max);
        let danger = hp < LOW_HP && stats.hp_max > 0.0;
        let length = |max: f32, per_point: f32| (max * per_point).clamp(cfg.min_length, cfg.max_length);
        let bars = [
            Bar {
                row: HP_ROW,
                length: length(stats.hp_max, cfg.hp_px_per_point),
                value: hp,
                lagged: self.hp.update(hp, dt),
                top: HP_TOP,
                bottom: HP_BOTTOM,
                danger,
            },
            Bar {
                row: FP_ROW,
                length: length(stats.fp_max, cfg.fp_px_per_point),
                value: share(stats.fp, stats.fp_max),
                lagged: self.fp.update(share(stats.fp, stats.fp_max), dt),
                top: FP_TOP,
                bottom: FP_BOTTOM,
                danger: false,
            },
            Bar {
                row: STAMINA_ROW,
                length: length(stats.stamina_max, cfg.stamina_px_per_point),
                value: share(stats.stamina, stats.stamina_max),
                lagged: self.stamina.update(share(stats.stamina, stats.stamina_max), dt),
                top: STAMINA_TOP,
                bottom: STAMINA_BOTTOM,
                danger: false,
            },
        ];

        if cfg.backing {
            for bar in &bars {
                canvas.fill(&canvas.bar_shape(bar, BACKING), PLATE, 1.0);
            }
        }
        for bar in &bars {
            canvas.bar(bar);
        }

        if cfg.show_numbers {
            let _font = self.font(ui, FONT_SMALL).map(|font| ui.push_font(font));
            let values = [(stats.hp, stats.hp_max), (stats.fp, stats.fp_max), (stats.stamina, stats.stamina_max)];
            for (bar, (value, max)) in bars.iter().zip(values) {
                let text = format!("{value:.0} / {max:.0}");
                let x = BARS_X + bar.length + 12.0;
                canvas.text(ui, x, (bar.row.0 + bar.row.1) * 0.5, (0.0, 0.5), TEXT, &text);
            }
        }
        if cfg.show_name && !stats.name.is_empty() {
            let _font = self.font(ui, FONT_NAME).map(|font| ui.push_font(font));
            canvas.text(ui, BARS_X, 28.0, (0.0, 0.5), TEXT, &stats.name);
        }

        canvas.medallion();
        if stats.great_rune {
            canvas.rune(stats.rune_active);
        }
        if cfg.show_level && stats.level > 0 {
            let _font = self.font(ui, FONT_SMALL).map(|font| ui.push_font(font));
            canvas.level_plate(ui, stats.level);
        }
    }
}

impl ImguiRenderLoop for Overlay {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        ctx.set_ini_filename(None);
        // Fonts are baked once, for the screen height; `before_render` corrects for the real picture.
        let screen = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(720) as f32;
        self.font_scale = screen / 1080.0 * shared().cfg.scale;
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_owned());
        let data = ["pala.ttf", "georgia.ttf", "segoeui.ttf"]
            .iter()
            .find_map(|name| std::fs::read(format!("{windir}\\Fonts\\{name}")).ok());
        let Some(data) = data else {
            logf!("overlay: no system font loaded, the built-in one is used");
            return;
        };
        let mut add = |size: f32| {
            ctx.fonts().add_font(&[FontSource::TtfData {
                data: &data,
                size_pixels: (size * self.font_scale).round(),
                config: Some(FontConfig {
                    glyph_ranges: FontGlyphRanges::cyrillic(),
                    ..FontConfig::default()
                }),
            }])
        };
        for size in FONT_SIZES {
            add(size);
        }
        self.fonts_loaded = true;
    }

    fn before_render<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        let cfg_scale = shared().cfg.scale;
        let io = ctx.io_mut();
        io.mouse_draw_cursor = false;
        let wanted = io.display_size[1] / 1080.0 * cfg_scale;
        if wanted > 0.0 && self.font_scale > 0.0 {
            io.font_global_scale = wanted / self.font_scale;
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        if DISABLED.load(Ordering::Relaxed) {
            return;
        }
        if catch_unwind(AssertUnwindSafe(|| self.frame(ui))).is_err() {
            DISABLED.store(true, Ordering::Relaxed);
            logf!("overlay: panic, the mod is switched off until the game restarts");
        }
    }

    fn message_filter(&self, _io: &Io) -> MessageFilter {
        MessageFilter::empty()
    }
}
