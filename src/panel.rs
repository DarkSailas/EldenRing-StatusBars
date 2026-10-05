use std::f32::consts::{PI, TAU};

use crate::{config::Config, state::Stats};

type Rgb = [f32; 3];
type Point = (f32, f32);

/// Blackened iron, embers and arcane light: the metal stays dark, the fills and the eye carry the colour.
const IRON: Rgb = [0.30, 0.31, 0.36];
const IRON_LIGHT: Rgb = [0.66, 0.68, 0.75];
const IRON_DARK: Rgb = [0.12, 0.12, 0.15];
const INK: Rgb = [0.02, 0.02, 0.03];
const BACK: Rgb = [0.045, 0.045, 0.06];
const TROUGH: Rgb = [0.10, 0.09, 0.12];
const FACE: Rgb = [0.07, 0.065, 0.09];
const TEXT: Rgb = [0.91, 0.90, 0.88];
const WHITE: Rgb = [1.0, 1.0, 1.0];

const HP_TOP: Rgb = [0.90, 0.18, 0.13];
const HP_BOTTOM: Rgb = [0.42, 0.03, 0.06];
const FP_TOP: Rgb = [0.52, 0.44, 1.0];
const FP_BOTTOM: Rgb = [0.18, 0.11, 0.55];
const STAMINA_TOP: Rgb = [0.40, 0.80, 0.42];
const STAMINA_BOTTOM: Rgb = [0.10, 0.36, 0.21];
const LAG: Rgb = [0.88, 0.86, 0.82];
const DANGER: Rgb = [1.0, 0.22, 0.12];
const EMBER: Rgb = [1.0, 0.50, 0.10];
const EMBER_LIT: Rgb = [1.0, 0.86, 0.35];
const EMBER_DEEP: Rgb = [0.55, 0.08, 0.03];
const ASH: Rgb = [0.42, 0.42, 0.50];
const ASH_DARK: Rgb = [0.14, 0.14, 0.18];
const ARCANE: Rgb = [0.66, 0.42, 1.0];

/// Layout in pixels of a 1920x1080 picture; the rows sit on the game's own bars.
const BARS_X: f32 = 150.0;
/// The frames start here, under the medallion.
const NECK_X: f32 = 118.0;
const HP_ROW: (f32, f32) = (45.0, 58.0);
const FP_ROW: (f32, f32) = (64.0, 77.0);
const STAMINA_ROW: (f32, f32) = (83.0, 96.0);
const MEDALLION: (f32, f32, f32) = (91.0, 70.0, 44.0);
const FRAME: f32 = 2.5;
/// How far the dark backing reaches past a bar; it is what covers the game's own bar.
const BACKING: f32 = 5.0;
/// Length of the blade at the end of a bar.
const FINIAL: f32 = 22.0;
const BANDS: usize = 6;
const LOW_HP: f32 = 0.25;
const NAME_Y: f32 = 25.0;
const TOP_RULE_Y: f32 = 38.0;
const BOTTOM_RULE_Y: f32 = 103.5;

/// Font sizes in the order they are added to the atlas; the constants below index it.
pub const FONT_SIZES: [f32; 2] = [16.0, 21.0];
pub const FONT_SMALL: usize = 0;
pub const FONT_NAME: usize = 1;

/// What the panel is drawn on, in screen pixels.
pub trait Surface {
    /// Filled polygon; it has to be convex.
    fn polygon(&self, points: &[[f32; 2]], color: [f32; 4]);
    fn polyline(&self, points: &[[f32; 2]], color: [f32; 4], thickness: f32);
    fn text_size(&self, font: usize, text: &str) -> [f32; 2];
    fn text(&self, font: usize, pos: [f32; 2], color: [f32; 4], text: &str);
}

pub struct Panel<'a> {
    pub cfg: &'a Config,
    pub stats: &'a Stats,
    /// Shares of HP, FP and stamina shown with a delay after a loss.
    pub lagged: [f32; 3],
    pub origin: [f32; 2],
    pub scale: f32,
    pub alpha: f32,
}

pub fn share(value: f32, max: f32) -> f32 {
    if max > 0.0 { (value / max).clamp(0.0, 1.0) } else { 0.0 }
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> [Point; 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn circle(x: f32, y: f32, radius: f32, segments: usize) -> Vec<Point> {
    (0..segments)
        .map(|k| TAU * k as f32 / segments as f32)
        .map(|angle| (x + angle.cos() * radius, y + angle.sin() * radius))
        .collect()
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
    surface: &'a dyn Surface,
    origin: [f32; 2],
    scale: f32,
    alpha: f32,
}

impl Canvas<'_> {
    fn at(&self, (x, y): Point) -> [f32; 2] {
        [self.origin[0] + x * self.scale, self.origin[1] + y * self.scale]
    }

    fn color(&self, rgb: Rgb, alpha: f32) -> [f32; 4] {
        [rgb[0], rgb[1], rgb[2], alpha * self.alpha]
    }

    fn fill(&self, points: &[Point], rgb: Rgb, alpha: f32) {
        let points: Vec<[f32; 2]> = points.iter().map(|&p| self.at(p)).collect();
        self.surface.polygon(&points, self.color(rgb, alpha));
    }

    fn stroke(&self, points: &[Point], closed: bool, rgb: Rgb, alpha: f32, thickness: f32) {
        let mut points: Vec<[f32; 2]> = points.iter().map(|&p| self.at(p)).collect();
        if closed && let Some(&first) = points.first() {
            points.push(first);
        }
        self.surface.polyline(&points, self.color(rgb, alpha), thickness * self.scale);
    }

    fn disc(&self, x: f32, y: f32, radius: f32, rgb: Rgb, alpha: f32) {
        let segments = if radius < 4.0 { 12 } else { 48 };
        self.fill(&circle(x, y, radius, segments), rgb, alpha);
    }

    fn ring(&self, x: f32, y: f32, radius: f32, rgb: Rgb, alpha: f32, thickness: f32) {
        self.stroke(&circle(x, y, radius, 48), true, rgb, alpha, thickness);
    }

    /// Part of a circle between two angles, as a line.
    fn arc(&self, (x, y, radius): (f32, f32, f32), (from, to): (f32, f32), rgb: Rgb, alpha: f32, thickness: f32) {
        let points: Vec<Point> = (0..=20)
            .map(|k| from + (to - from) * k as f32 / 20.0)
            .map(|angle| (x + angle.cos() * radius, y + angle.sin() * radius))
            .collect();
        self.stroke(&points, false, rgb, alpha, thickness);
    }

    /// Text with a dark edge; `anchor` is the share of the text size left of and above the point.
    fn text(&self, font: usize, at: Point, anchor: Point, rgb: Rgb, text: &str) {
        let size = self.surface.text_size(font, text);
        let pos = self.at(at);
        let pos = [(pos[0] - size[0] * anchor.0).round(), (pos[1] - size[1] * anchor.1).round()];
        let edge = self.scale.max(1.0);
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (0.0, 2.0)] {
            self.surface.text(font, [pos[0] + dx * edge, pos[1] + dy * edge], self.color(INK, 0.8), text);
        }
        self.surface.text(font, pos, self.color(rgb, 1.0), text);
    }

    /// Text width in panel units.
    fn text_width(&self, font: usize, text: &str) -> f32 {
        self.surface.text_size(font, text)[0] / self.scale
    }

    /// Dark plate under a bar and its finial, pointed at the far end.
    fn backing(&self, bar: &Bar) {
        let (y0, y1) = (bar.row.0 - BACKING, bar.row.1 + BACKING);
        let end = BARS_X + bar.length + FRAME + 16.0;
        let shape = [(NECK_X - 20.0, y0), (end, y0), (end + 9.0, (y0 + y1) * 0.5), (end, y1), (NECK_X - 20.0, y1)];
        self.fill(&shape, BACK, 1.0);
    }

    /// Fills the part of the bar between `from` and `to` (pixels from its start) with a vertical gradient.
    fn bar_span(&self, bar: &Bar, (from, to): (f32, f32), (top, bottom): (Rgb, Rgb), alpha: f32) {
        if to <= from {
            return;
        }
        let (y0, y1) = bar.row;
        let (x0, x1) = (BARS_X + from, BARS_X + to);
        for band in 0..BANDS {
            let ta = band as f32 / BANDS as f32;
            let tb = (band + 1) as f32 / BANDS as f32;
            let (ya, yb) = (y0 + (y1 - y0) * ta, y0 + (y1 - y0) * tb);
            self.fill(&rect(x0, ya, x1, yb), mix(top, bottom, (ta + tb) * 0.5), alpha);
        }
    }

    /// Barbed iron blade with a stone of the bar's colour, at the far end of a bar.
    fn finial(&self, bar: &Bar, metal: Rgb) {
        let (y0, y1) = bar.row;
        let mid = (y0 + y1) * 0.5;
        let tip = BARS_X + bar.length + FRAME;
        let (notch, point) = (tip + 3.0, tip + FINIAL);
        let upper = [(notch, mid), (tip - 2.0, y0 - 5.0), (point, mid)];
        let lower = [(notch, mid), (point, mid), (tip - 2.0, y1 + 5.0)];

        self.fill(&upper, mix(metal, IRON_LIGHT, 0.6), 1.0);
        self.fill(&lower, mix(metal, IRON_DARK, 0.6), 1.0);
        self.stroke(&[upper[0], upper[1], upper[2], lower[2]], true, INK, 0.9, 1.0);

        let centre = tip + 7.0;
        let stone = [(centre - 3.0, mid), (centre, mid - 3.5), (centre + 4.5, mid), (centre, mid + 3.5)];
        self.fill(&stone, mix(bar.bottom, INK, 0.2), 1.0);
        self.fill(&[stone[0], stone[1], stone[2]], bar.top, 1.0);
        self.stroke(&stone, true, INK, 0.9, 1.0);
    }

    fn bar(&self, bar: &Bar) {
        let (y0, y1) = bar.row;
        let end = BARS_X + bar.length;
        let metal = if bar.danger { DANGER } else { IRON };
        let frame = rect(NECK_X, y0 - FRAME, end + FRAME, y1 + FRAME);
        let trough = rect(BARS_X, y0, end, y1);

        self.fill(&frame, mix(IRON_DARK, metal, if bar.danger { 0.4 } else { 0.0 }), 1.0);
        self.fill(&trough, TROUGH, 1.0);

        let filled = bar.length * bar.value;
        self.bar_span(bar, (filled, bar.length * bar.lagged.min(1.0)), (LAG, mix(LAG, INK, 0.5)), 0.9);
        self.bar_span(bar, (0.0, filled), (bar.top, bar.bottom), 1.0);
        if filled > 3.0 {
            let gloss = mix(bar.top, WHITE, 0.55);
            self.stroke(&[(BARS_X + 1.0, y0 + 2.0), (BARS_X + filled - 1.0, y0 + 2.0)], false, gloss, 0.5, 1.0);
            if bar.value < 1.0 {
                self.stroke(&[(BARS_X + filled, y0), (BARS_X + filled, y1)], false, gloss, 0.9, 1.0);
            }
        }

        self.stroke(&trough, true, INK, 0.9, 1.0);
        self.stroke(&frame, true, INK, 0.85, 2.8);
        self.stroke(&frame, true, metal, 1.0, 1.4);
        let light = if bar.danger { mix(DANGER, WHITE, 0.3) } else { IRON_LIGHT };
        self.stroke(&[(NECK_X, y0 - FRAME), (end + FRAME, y0 - FRAME)], false, light, 0.8, 1.0);
        self.disc(BARS_X - 6.0, (y0 + y1) * 0.5, 2.0, mix(bar.top, WHITE, 0.25), 1.0);
        self.finial(bar, metal);
    }

    /// Iron line that ends like the tail of a dragon: a wave and a barbed tip.
    /// `turn` is -1 for the first swing upwards and 1 for downwards; the lower one carries spines.
    fn tail(&self, x0: f32, x1: f32, y: f32, turn: f32) {
        const STEPS: usize = 30;
        const WAVE: f32 = 54.0;
        let start = (x1 - WAVE).max(x0);
        let mut points = vec![(x0, y)];
        points.extend((0..=STEPS).map(|k| {
            let t = k as f32 / STEPS as f32;
            (start + (x1 - start) * t, y + turn * (t * 1.5 * TAU).sin() * 4.0 * t.sqrt() * (1.0 - t * 0.35))
        }));
        self.stroke(&points, false, INK, 0.75, 3.2);
        self.stroke(&points, false, IRON_LIGHT, 1.0, 1.5);

        let upper = [(x1 + 1.0, y), (x1 - 4.0, y - 5.0), (x1 + 13.0, y)];
        let lower = [(x1 + 1.0, y), (x1 + 13.0, y), (x1 - 4.0, y + 5.0)];
        self.fill(&upper, IRON_LIGHT, 1.0);
        self.fill(&lower, IRON, 1.0);
        self.stroke(&[upper[0], upper[1], upper[2], lower[2]], true, INK, 0.9, 1.0);

        if turn > 0.0 {
            let straight = start - x0;
            for share in [0.2, 0.45, 0.7, 0.95] {
                let x = x0 + straight * share;
                let spine = [(x - 4.0, y), (x + 5.0, y + 6.0), (x + 3.0, y)];
                self.fill(&spine, IRON_LIGHT, 1.0);
                self.stroke(&spine, true, INK, 0.8, 0.8);
            }
        }
    }

    /// Faceted spike behind the medallion.
    fn spike(&self, angle: f32, tip: f32, half_width: f32) {
        let (cx, cy, r) = MEDALLION;
        let (c, s) = (angle.cos(), angle.sin());
        let base = (cx + c * (r - 4.0), cy + s * (r - 4.0));
        let tip = (cx + c * (r + tip), cy + s * (r + tip));
        let left = (base.0 + s * half_width, base.1 - c * half_width);
        let right = (base.0 - s * half_width, base.1 + c * half_width);
        self.fill(&[left, tip, base], IRON_LIGHT, 1.0);
        self.fill(&[base, tip, right], IRON_DARK, 1.0);
        self.stroke(&[left, tip, right], false, INK, 0.85, 1.0);
    }

    /// Horn along a quadratic curve, `width` thick at the root and pointed at the end.
    fn horn(&self, [a, b, c]: [Point; 3], width: f32) {
        const STEPS: usize = 9;
        let at = |t: f32| {
            let u = 1.0 - t;
            (u * u * a.0 + 2.0 * u * t * b.0 + t * t * c.0, u * u * a.1 + 2.0 * u * t * b.1 + t * t * c.1)
        };
        let edge = |k: usize, side: f32| {
            let t = k as f32 / STEPS as f32;
            let (p, q) = (at((t - 0.02).max(0.0)), at((t + 0.02).min(1.0)));
            let (dx, dy) = (q.0 - p.0, q.1 - p.1);
            let length = (dx * dx + dy * dy).sqrt().max(1e-3);
            let half = width * 0.5 * (1.0 - t) * side;
            let spine = at(t);
            (spine.0 - dy / length * half, spine.1 + dx / length * half)
        };
        for k in 0..STEPS {
            let (from, to) = (edge(k, 0.0), edge(k + 1, 0.0));
            self.fill(&[edge(k, 1.0), edge(k + 1, 1.0), to, from], IRON_LIGHT, 1.0);
            self.fill(&[from, to, edge(k + 1, -1.0), edge(k, -1.0)], IRON_DARK, 1.0);
        }
        let outline: Vec<Point> =
            (0..=STEPS).map(|k| edge(k, 1.0)).chain((0..STEPS).rev().map(|k| edge(k, -1.0))).collect();
        self.stroke(&outline, false, INK, 0.9, 1.0);
        // Ridges across the horn, towards the root.
        for k in 1..5 {
            self.stroke(&[edge(k, 1.0), edge(k, -1.0)], false, INK, 0.55, 1.0);
        }
    }

    /// Runes cut into the ring of the medallion; a Great Rune lights them.
    fn runes(&self, equipped: bool, lit: bool) {
        const GLYPHS: [&[&[Point]]; 6] = [
            &[&[(0.0, -3.0), (0.0, 3.0)], &[(0.0, -3.0), (2.5, -0.5), (0.0, 1.0)]],
            &[&[(-2.0, -3.0), (2.0, 3.0)], &[(2.0, -3.0), (-2.0, 3.0)]],
            &[&[(0.0, -3.0), (0.0, 3.0)], &[(-2.5, -0.5), (0.0, -3.0), (2.5, -0.5)]],
            &[&[(-2.0, 3.0), (-2.0, -3.0), (2.0, 3.0), (2.0, -3.0)]],
            &[&[(0.0, -3.0), (-2.5, 0.0), (0.0, 3.0), (2.5, 0.0), (0.0, -3.0)]],
            &[&[(-1.5, -3.0), (-1.5, 3.0)], &[(-1.5, -3.0), (2.0, -1.0), (-1.5, 1.0)]],
        ];
        const COUNT: usize = 10;
        let (cx, cy, r) = MEDALLION;
        let (rgb, alpha) = match (equipped, lit) {
            (_, true) => (mix(ARCANE, WHITE, 0.45), 1.0),
            (true, false) => (ARCANE, 0.95),
            (false, false) => (ASH, 0.75),
        };
        for k in 0..COUNT {
            let angle = TAU * (k as f32 + 0.5) / COUNT as f32;
            let (c, s) = (angle.cos(), angle.sin());
            let centre = (cx + c * (r - 9.5), cy + s * (r - 9.5));
            for line in GLYPHS[k % GLYPHS.len()] {
                let points: Vec<Point> =
                    line.iter().map(|&(u, v)| (centre.0 - s * u - c * v, centre.1 + c * u - s * v)).collect();
                if lit {
                    self.stroke(&points, false, ARCANE, 0.35, 3.2);
                }
                self.stroke(&points, false, rgb, alpha, 1.1);
            }
        }
    }

    /// Eye of a dragon in the medallion: ashen without a Great Rune, burning with one,
    /// blazing while a Rune Arc powers it.
    fn eye(&self, equipped: bool, lit: bool) {
        const RINGS: usize = 7;
        const STREAKS: usize = 22;
        const HALF: usize = 10;
        let (cx, cy, r) = MEDALLION;
        let radius = r - 17.0;
        let (rim, core, slit) = match (equipped, lit) {
            (_, true) => (EMBER_DEEP, EMBER_LIT, 3.6),
            (true, false) => (mix(EMBER_DEEP, INK, 0.35), EMBER, 4.6),
            (false, false) => (ASH_DARK, ASH, 5.4),
        };
        if lit {
            self.disc(cx, cy, radius + 5.0, EMBER, 0.28);
        }
        self.disc(cx, cy, radius + 1.2, INK, 1.0);
        for k in 0..RINGS {
            let t = k as f32 / (RINGS - 1) as f32;
            self.disc(cx, cy, radius * (1.0 - 0.74 * t), mix(rim, core, t), 1.0);
        }
        for k in 0..STREAKS {
            let angle = TAU * k as f32 / STREAKS as f32 + 0.07;
            let (c, s) = (angle.cos(), angle.sin());
            let (from, to) = (radius * (0.34 + 0.12 * (k % 3) as f32), radius * 0.97);
            self.stroke(&[(cx + c * from, cy + s * from), (cx + c * to, cy + s * to)], false, rim, 0.5, 1.0);
        }

        let height = radius * 0.88;
        let side = |sign: f32, k: usize| {
            let t = k as f32 / HALF as f32 * 2.0 - 1.0;
            (cx + sign * slit * (1.0 - t * t), cy + height * t * sign)
        };
        let pupil: Vec<Point> =
            (0..HALF * 2).map(|k| if k < HALF { side(1.0, k) } else { side(-1.0, k - HALF) }).collect();
        self.fill(&pupil, INK, 1.0);
        self.stroke(&pupil, true, mix(core, WHITE, 0.3), if equipped { 0.55 } else { 0.25 }, 0.8);
        self.disc(cx - 8.0, cy - 9.0, 2.4, WHITE, if equipped { 0.75 } else { 0.35 });
    }

    /// Round iron shield with horns and spikes; the runes and the eye go on top.
    fn medallion(&self, level_shown: bool, lit: bool) {
        let (cx, cy, r) = MEDALLION;
        if lit {
            self.disc(cx, cy, r + 12.0, ARCANE, 0.14);
            self.disc(cx, cy, r + 8.0, ARCANE, 0.14);
        } else {
            self.disc(cx, cy + 2.0, r + 8.0, INK, 0.55);
        }

        for side in [-1.0, 1.0] {
            let root = (cx + side * 22.0, cy - 34.0);
            self.horn([root, (cx + side * 49.0, cy - 46.0), (cx + side * 31.0, cy - 66.0)], 13.0);
        }
        self.spike(-PI * 0.5, 9.0, 5.0);
        self.spike(PI, 14.0, 7.0);
        self.spike(PI * 0.82, 8.0, 5.0);
        self.spike(PI * 1.18, 8.0, 5.0);
        self.spike(PI * 0.68, 11.0, 6.0);
        self.spike(PI * 0.32, 11.0, 6.0);
        if !level_shown {
            self.spike(PI * 0.5, 15.0, 7.0);
        }

        self.disc(cx, cy, r + 4.5, INK, 1.0);
        self.disc(cx, cy, r + 3.5, IRON_DARK, 1.0);
        self.ring(cx, cy, r + 0.5, IRON, 1.0, 3.0);
        self.arc((cx, cy, r + 1.5), (PI * 1.1, PI * 1.9), IRON_LIGHT, 0.9, 1.2);
        self.disc(cx, cy, r - 3.0, INK, 1.0);
        self.disc(cx, cy, r - 4.0, FACE, 1.0);
        self.ring(cx, cy, r - 15.0, IRON, 1.0, 1.2);
    }

    /// Pointed iron scale with the character level, hung under the medallion.
    fn level_plate(&self, level: u32) {
        let text = level.to_string();
        let (cx, cy, r) = MEDALLION;
        let (y, half_h) = (cy + r + 13.0, 10.5);
        let half_w = (self.text_width(FONT_SMALL, &text) * 0.5 + 9.0).max(17.0);
        let shape = [
            (cx - half_w, y - half_h),
            (cx + half_w, y - half_h),
            (cx + half_w + 5.0, y),
            (cx + half_w - 4.0, y + half_h),
            (cx, y + half_h + 7.0),
            (cx - half_w + 4.0, y + half_h),
            (cx - half_w - 5.0, y),
        ];
        self.fill(&shape, FACE, 1.0);
        self.stroke(&shape, true, INK, 0.9, 3.6);
        self.stroke(&shape, true, IRON_LIGHT, 1.0, 1.4);
        self.text(FONT_SMALL, (cx, y - 1.0), (0.5, 0.5), TEXT, &text);
    }
}

pub fn draw(surface: &dyn Surface, panel: &Panel) {
    let (cfg, stats) = (panel.cfg, panel.stats);
    let canvas = Canvas {
        surface,
        origin: panel.origin,
        scale: panel.scale,
        alpha: panel.alpha,
    };

    let hp = share(stats.hp, stats.hp_max);
    let length = |max: f32, per_point: f32| (max * per_point).clamp(cfg.min_length, cfg.max_length);
    let bars = [
        Bar {
            row: HP_ROW,
            length: length(stats.hp_max, cfg.hp_px_per_point),
            value: hp,
            lagged: panel.lagged[0],
            top: HP_TOP,
            bottom: HP_BOTTOM,
            danger: hp < LOW_HP && stats.hp_max > 0.0,
        },
        Bar {
            row: FP_ROW,
            length: length(stats.fp_max, cfg.fp_px_per_point),
            value: share(stats.fp, stats.fp_max),
            lagged: panel.lagged[1],
            top: FP_TOP,
            bottom: FP_BOTTOM,
            danger: false,
        },
        Bar {
            row: STAMINA_ROW,
            length: length(stats.stamina_max, cfg.stamina_px_per_point),
            value: share(stats.stamina, stats.stamina_max),
            lagged: panel.lagged[2],
            top: STAMINA_TOP,
            bottom: STAMINA_BOTTOM,
            danger: false,
        },
    ];

    if cfg.backing {
        for bar in &bars {
            canvas.backing(bar);
        }
    }
    for bar in &bars {
        canvas.bar(bar);
    }

    if cfg.show_numbers {
        let values = [(stats.hp, stats.hp_max), (stats.fp, stats.fp_max), (stats.stamina, stats.stamina_max)];
        for (bar, (value, max)) in bars.iter().zip(values) {
            let text = format!("{value:.0} / {max:.0}");
            let x = BARS_X + bar.length + FRAME + FINIAL + 8.0;
            canvas.text(FONT_SMALL, (x, (bar.row.0 + bar.row.1) * 0.5), (0.0, 0.5), TEXT, &text);
        }
    }

    let named = cfg.show_name && !stats.name.is_empty();
    let name_width = if named { canvas.text_width(FONT_NAME, &stats.name) } else { 0.0 };
    let rule_end = BARS_X + (name_width + 70.0).max(170.0);
    canvas.tail(NECK_X + 14.0, rule_end, TOP_RULE_Y, -1.0);
    canvas.tail(NECK_X + 14.0, BARS_X + 130.0, BOTTOM_RULE_Y, 1.0);
    if named {
        canvas.text(FONT_NAME, (BARS_X + 2.0, NAME_Y), (0.0, 0.5), TEXT, &stats.name);
    }

    let level_shown = cfg.show_level && stats.level > 0;
    canvas.medallion(level_shown, stats.rune_active);
    canvas.runes(stats.great_rune, stats.rune_active);
    canvas.eye(stats.great_rune, stats.rune_active);
    if level_shown {
        canvas.level_plate(stats.level);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, fmt::Write as _};

    use super::*;

    /// Records the drawing as SVG and keeps the filled polygons for checks.
    #[derive(Default)]
    struct Svg {
        body: RefCell<String>,
        polygons: RefCell<Vec<Vec<[f32; 2]>>>,
    }

    fn paint(color: [f32; 4]) -> String {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("rgba({},{},{},{:.3})", c(color[0]), c(color[1]), c(color[2]), color[3])
    }

    fn path(points: &[[f32; 2]]) -> String {
        points.iter().map(|p| format!("{:.2},{:.2}", p[0], p[1])).collect::<Vec<_>>().join(" ")
    }

    impl Surface for Svg {
        fn polygon(&self, points: &[[f32; 2]], color: [f32; 4]) {
            self.polygons.borrow_mut().push(points.to_vec());
            let _ = writeln!(self.body.borrow_mut(), r#"<polygon points="{}" fill="{}"/>"#, path(points), paint(color));
        }

        fn polyline(&self, points: &[[f32; 2]], color: [f32; 4], thickness: f32) {
            let _ = writeln!(
                self.body.borrow_mut(),
                r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{thickness:.2}"/>"#,
                path(points),
                paint(color)
            );
        }

        fn text_size(&self, font: usize, text: &str) -> [f32; 2] {
            let size = FONT_SIZES[font];
            [text.chars().count() as f32 * size * 0.47, size * 1.2]
        }

        fn text(&self, font: usize, pos: [f32; 2], color: [f32; 4], text: &str) {
            let size = FONT_SIZES[font];
            let _ = writeln!(
                self.body.borrow_mut(),
                r#"<text x="{:.1}" y="{:.1}" font-family="Palatino Linotype" font-size="{size}" fill="{}">{text}</text>"#,
                pos[0],
                pos[1] + size * 0.92,
                paint(color)
            );
        }
    }

    fn convex(points: &[[f32; 2]]) -> bool {
        let n = points.len();
        let mut sign = 0.0f32;
        for i in 0..n {
            let (a, b, c) = (points[i], points[(i + 1) % n], points[(i + 2) % n]);
            let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
            if cross.abs() < 1e-3 {
                continue;
            }
            if sign != 0.0 && sign.signum() != cross.signum() {
                return false;
            }
            sign = cross;
        }
        true
    }

    fn sample(hp: f32, great_rune: bool, rune_active: bool) -> Stats {
        Stats {
            visible: true,
            hp,
            hp_max: 990.0,
            fp: 62.0,
            fp_max: 97.0,
            stamina: 110.0,
            stamina_max: 110.0,
            name: "Mara".to_owned(),
            level: 70,
            great_rune,
            rune_active,
        }
    }

    /// The overlay fills polygons as convex ones, so every shape has to be convex.
    /// Also writes a preview of the panel next to the temporary files.
    #[test]
    fn every_filled_shape_is_convex() {
        let cfg = Config::default();
        let svg = Svg::default();
        let cases = [sample(700.0, false, false), sample(180.0, true, false), sample(990.0, true, true)];
        for (row, stats) in cases.iter().enumerate() {
            let panel = Panel {
                cfg: &cfg,
                stats,
                lagged: [share(stats.hp, stats.hp_max) + 0.12, 0.8, 1.0],
                origin: [0.0, row as f32 * 150.0],
                scale: 1.0,
                alpha: 1.0,
            };
            draw(&svg, &panel);
        }
        for polygon in svg.polygons.borrow().iter() {
            assert!(convex(polygon), "not convex: {polygon:?}");
        }

        let page = format!(
            concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="760" height="450">"#,
                r##"<rect width="380" height="450" fill="#2a3226"/><rect x="380" width="380" height="450" fill="#9fb2c2"/>"##,
                "{}</svg>"
            ),
            svg.body.borrow()
        );
        let _ = std::fs::write(std::env::temp_dir().join("er_status_bars_preview.svg"), page);
    }
}
