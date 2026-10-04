use std::{
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
    panel::{self, FONT_SIZES, Panel, Surface, share},
    state::{DISABLED, Stats, shared},
};

/// Seconds a bar keeps showing the lost part before it starts to shrink.
const LAG_HOLD: f32 = 0.45;
/// Share of the bar the lost part shrinks by in a second.
const LAG_SPEED: f32 = 0.7;
/// The panel is up within a few frames, so the game's own bars are not seen first.
const FADE_IN: f32 = 14.0;
const FADE_OUT: f32 = 16.0;

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

/// The background draw list of the frame, with the fonts of the panel.
struct Screen<'a> {
    ui: &'a Ui,
    list: DrawListMut<'a>,
    fonts: [Option<FontId>; 2],
}

impl Surface for Screen<'_> {
    fn polygon(&self, points: &[[f32; 2]], color: [f32; 4]) {
        self.list.add_polyline(points.to_vec(), color).filled(true).build();
    }

    fn polyline(&self, points: &[[f32; 2]], color: [f32; 4], thickness: f32) {
        self.list.add_polyline(points.to_vec(), color).thickness(thickness).build();
    }

    fn text_size(&self, font: usize, text: &str) -> [f32; 2] {
        let _font = self.fonts[font].map(|font| self.ui.push_font(font));
        self.ui.calc_text_size(text)
    }

    fn text(&self, font: usize, pos: [f32; 2], color: [f32; 4], text: &str) {
        let _font = self.fonts[font].map(|font| self.ui.push_font(font));
        self.list.add_text(pos, color, text);
    }
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
        let base = display[1] / 1080.0;
        // The game keeps its interface inside the middle 16:9 part of a wider picture.
        let side = ((display[0] - display[1] * 16.0 / 9.0) * 0.5).max(0.0);
        let lagged = [
            self.hp.update(share(stats.hp, stats.hp_max), dt),
            self.fp.update(share(stats.fp, stats.fp_max), dt),
            self.stamina.update(share(stats.stamina, stats.stamina_max), dt),
        ];
        let screen = Screen {
            ui,
            list: ui.get_background_draw_list(),
            fonts: [self.font(ui, panel::FONT_SMALL), self.font(ui, panel::FONT_NAME)],
        };
        let panel = Panel {
            cfg: &cfg,
            stats,
            lagged,
            origin: [side + cfg.offset_x * base, cfg.offset_y * base],
            scale: Self::panel_scale(&cfg, display),
            alpha: self.alpha * cfg.opacity,
        };
        panel::draw(&screen, &panel);
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
