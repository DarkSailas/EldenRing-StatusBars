use std::{fmt::Write as _, fs, io, path::Path};

/// All lengths are in pixels of a 1920x1080 picture; the overlay scales them to the real one.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub log: bool,
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub opacity: f32,
    /// Solid plate behind the bars: it is what hides the game's own bars.
    pub backing: bool,
    pub show_name: bool,
    pub show_numbers: bool,
    pub show_level: bool,
    /// Seconds the game's interface has to stay up before the panel comes in.
    pub show_delay: f32,
    /// Keep the panel away while the game fades the screen: loads, warps, cutscene cuts.
    pub hide_during_fade: bool,
    /// Hide the panel while a game menu or a popup is open.
    pub hide_in_menus: bool,
    pub hp_px_per_point: f32,
    pub fp_px_per_point: f32,
    pub stamina_px_per_point: f32,
    pub min_length: f32,
    pub max_length: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            log: true,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            opacity: 1.0,
            backing: true,
            show_name: true,
            show_numbers: true,
            show_level: true,
            show_delay: 0.0,
            hide_during_fade: true,
            hide_in_menus: true,
            hp_px_per_point: 0.4,
            fp_px_per_point: 1.75,
            stamina_px_per_point: 3.0,
            min_length: 60.0,
            max_length: 900.0,
        }
    }
}

fn flag(value: &str) -> Option<bool> {
    match value {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

impl Config {
    pub fn parse(text: &str) -> Self {
        let mut cfg = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with([';', '#', '[']) {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().to_ascii_lowercase();
            let num = value.parse::<f32>().ok().filter(|v| v.is_finite());
            let set_flag = |target: &mut bool| {
                if let Some(v) = flag(&value) {
                    *target = v;
                }
            };
            let set_num = |target: &mut f32| {
                if let Some(v) = num {
                    *target = v;
                }
            };
            match key.trim().to_ascii_lowercase().as_str() {
                "enabled" => set_flag(&mut cfg.enabled),
                "log" => set_flag(&mut cfg.log),
                "scale" => set_num(&mut cfg.scale),
                "offsetx" => set_num(&mut cfg.offset_x),
                "offsety" => set_num(&mut cfg.offset_y),
                "opacity" => set_num(&mut cfg.opacity),
                "backing" => set_flag(&mut cfg.backing),
                "showname" => set_flag(&mut cfg.show_name),
                "shownumbers" => set_flag(&mut cfg.show_numbers),
                "showlevel" => set_flag(&mut cfg.show_level),
                "showdelay" => set_num(&mut cfg.show_delay),
                "hideduringfade" => set_flag(&mut cfg.hide_during_fade),
                "hideinmenus" => set_flag(&mut cfg.hide_in_menus),
                "hppixelsperpoint" => set_num(&mut cfg.hp_px_per_point),
                "fppixelsperpoint" => set_num(&mut cfg.fp_px_per_point),
                "staminapixelsperpoint" => set_num(&mut cfg.stamina_px_per_point),
                "minlength" => set_num(&mut cfg.min_length),
                "maxlength" => set_num(&mut cfg.max_length),
                _ => {}
            }
        }
        cfg.clamp();
        cfg
    }

    pub fn clamp(&mut self) {
        self.scale = self.scale.clamp(0.5, 3.0);
        self.offset_x = self.offset_x.clamp(-4000.0, 4000.0);
        self.offset_y = self.offset_y.clamp(-4000.0, 4000.0);
        self.opacity = self.opacity.clamp(0.1, 1.0);
        self.show_delay = self.show_delay.clamp(0.0, 10.0);
        self.hp_px_per_point = self.hp_px_per_point.clamp(0.0, 10.0);
        self.fp_px_per_point = self.fp_px_per_point.clamp(0.0, 10.0);
        self.stamina_px_per_point = self.stamina_px_per_point.clamp(0.0, 10.0);
        self.min_length = self.min_length.clamp(60.0, 1600.0);
        self.max_length = self.max_length.clamp(self.min_length, 1600.0);
    }

    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path).map(|text| Self::parse(&text)).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut out = String::new();
        let b = |v: bool| u8::from(v);
        let _ = writeln!(out, "; er_status_bars: the file is re-read while the game runs, about a second after it is saved.");
        let _ = writeln!(out, "; Lengths are in pixels of a 1920x1080 picture.");
        let _ = writeln!(out, "[Panel]");
        let _ = writeln!(out, "Enabled = {}", b(self.enabled));
        let _ = writeln!(out, "Scale = {}", self.scale);
        let _ = writeln!(out, "OffsetX = {}", self.offset_x);
        let _ = writeln!(out, "OffsetY = {}", self.offset_y);
        let _ = writeln!(out, "Opacity = {}", self.opacity);
        let _ = writeln!(out, "; Solid plate behind the bars. It hides the game's own bars; switch it off only if they are hidden another way.");
        let _ = writeln!(out, "Backing = {}", b(self.backing));
        let _ = writeln!(out, "ShowName = {}", b(self.show_name));
        let _ = writeln!(out, "ShowNumbers = {}", b(self.show_numbers));
        let _ = writeln!(out, "ShowLevel = {}", b(self.show_level));
        let _ = writeln!(out, "; Seconds the game's interface has to stay up before the panel comes in.");
        let _ = writeln!(out, "ShowDelay = {}", self.show_delay);
        let _ = writeln!(out, "; Keeps the panel away while the game fades the screen. Switch it off if the panel never appears.");
        let _ = writeln!(out, "HideDuringFade = {}", b(self.hide_during_fade));
        let _ = writeln!(out, "; Hides the panel while a game menu or a popup is open. Switch it off if the panel goes missing during play.");
        let _ = writeln!(out, "HideInMenus = {}", b(self.hide_in_menus));
        let _ = writeln!(out);
        let _ = writeln!(out, "[Bars]");
        let _ = writeln!(out, "; Bar length = maximum value * pixels per point, kept between MinLength and MaxLength.");
        let _ = writeln!(out, "; Raise a value if the game's bar shows from under the panel on the right.");
        let _ = writeln!(out, "HpPixelsPerPoint = {}", self.hp_px_per_point);
        let _ = writeln!(out, "FpPixelsPerPoint = {}", self.fp_px_per_point);
        let _ = writeln!(out, "StaminaPixelsPerPoint = {}", self.stamina_px_per_point);
        let _ = writeln!(out, "MinLength = {}", self.min_length);
        let _ = writeln!(out, "MaxLength = {}", self.max_length);
        let _ = writeln!(out);
        let _ = writeln!(out, "[Debug]");
        let _ = writeln!(out, "Log = {}", b(self.log));
        fs::write(path, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_keys_and_ignores_the_rest() {
        let cfg = Config::parse("[Panel]\nEnabled = 0\nScale=1.5\nunknown = 7\n; note\nShowName = off\nShowDelay = 2.5\nHideDuringFade = 0\n[Bars]\nMaxLength = 700\n");
        assert!(!cfg.enabled);
        assert_eq!(cfg.show_delay, 2.5);
        assert!(!cfg.hide_during_fade);
        assert_eq!(cfg.scale, 1.5);
        assert!(!cfg.show_name);
        assert_eq!(cfg.max_length, 700.0);
        assert_eq!(cfg.min_length, Config::default().min_length);
    }

    #[test]
    fn clamps_out_of_range_values() {
        let cfg = Config::parse("Scale = 99\nOpacity = -3\nMinLength = 500\nMaxLength = 100\nHpPixelsPerPoint = nan\n");
        assert_eq!(cfg.scale, 3.0);
        assert_eq!(cfg.opacity, 0.1);
        assert_eq!(cfg.max_length, 500.0);
        assert_eq!(cfg.hp_px_per_point, Config::default().hp_px_per_point);
    }

    #[test]
    fn saved_file_reads_back_the_same() {
        let mut cfg = Config::default();
        cfg.scale = 1.25;
        cfg.backing = false;
        cfg.show_level = false;
        cfg.show_delay = 0.5;
        let path = std::env::temp_dir().join("er_status_bars_roundtrip_test.ini");
        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path), cfg);
        let _ = fs::remove_file(path);
    }
}
