use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use eldenring::cs::{CSFD4FadePlate, CSFade, CSFeManHudState, CSFeManImp, WorldChrMan};
use fromsoftware_shared::FromStatic;

use crate::{
    config::Config,
    logf,
    state::{Stats, ini_path, shared},
};

const INI_POLL: Duration = Duration::from_secs(1);
/// Alpha of a fade plate from which the screen counts as covered.
const FADE_DARK: f32 = 0.5;
/// A plate that stays dark this long is taken for a stuck one and ignored until it clears.
const FADE_TIMEOUT: Duration = Duration::from_secs(20);

pub struct Game {
    ini: PathBuf,
    ini_time: Option<SystemTime>,
    ini_checked: Instant,
    was_visible: bool,
    last_hud_state: Option<CSFeManHudState>,
    last_rune: Option<(u32, i32, bool)>,
    dark_since: Option<Instant>,
    fade_ignored: bool,
    ready_since: Option<Instant>,
}

struct Reading {
    stats: Stats,
    hud_state: CSFeManHudState,
    /// Raw handle and index of the Great Rune slot.
    rune: (u32, i32),
    /// Maximum hit points as the game's own bars know them.
    hud_hp_max: u32,
}

fn modified(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// The darkest fade plate: its number and alpha. The game covers the screen with them on loads and warps.
fn darkest_fade_plate() -> Option<(usize, f32)> {
    let fade = unsafe { CSFade::instance() }.ok()?;
    // Read as plain pointers: a slot the game left empty must not be dereferenced.
    let plates = unsafe { &*(&raw const fade.fade_plates).cast::<[*const CSFD4FadePlate; 9]>() };
    plates
        .iter()
        .enumerate()
        .filter(|(_, plate)| !plate.is_null())
        .map(|(index, plate)| (index, unsafe { (**plate).current_color.a }))
        .filter(|(_, alpha)| alpha.is_finite())
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

fn read() -> Option<Reading> {
    let fe = unsafe { CSFeManImp::instance() }.ok()?;
    let chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    let player = chr_man.main_player.as_ref()?;
    let data = &player.chr_ins.modules.data;
    let game_data = unsafe { player.player_game_data.as_ref() };

    let slot = &game_data.equipment.equip_item_data.great_rune;
    let handle = unsafe { *(&raw const slot.gaitem_handle).cast::<u32>() };
    let great_rune = handle != 0 && handle != u32::MAX && slot.index >= 0;

    let name = &game_data.character_name;
    let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let stats = Stats {
        visible: false,
        hp: data.hp.max(0) as f32,
        hp_max: data.max_hp.max(0) as f32,
        fp: data.fp.max(0) as f32,
        fp_max: data.max_fp.max(0) as f32,
        stamina: data.stamina.max(0) as f32,
        stamina_max: data.max_stamina.max(0) as f32,
        name: String::from_utf16_lossy(&name[..len]),
        level: game_data.level,
        great_rune,
        rune_active: great_rune && game_data.rune_arc_active,
    };
    Some(Reading {
        stats,
        hud_state: fe.hud_state,
        rune: (handle, slot.index),
        hud_hp_max: fe.frontend_values.hp_max_uncapped,
    })
}

impl Game {
    pub fn new() -> Self {
        let ini = ini_path();
        Self {
            ini_time: modified(&ini),
            ini,
            ini_checked: Instant::now(),
            was_visible: false,
            last_hud_state: None,
            last_rune: None,
            dark_since: None,
            fade_ignored: false,
            ready_since: None,
        }
    }

    fn reload_ini(&mut self) {
        if self.ini_checked.elapsed() < INI_POLL {
            return;
        }
        self.ini_checked = Instant::now();
        let time = modified(&self.ini);
        if time == self.ini_time {
            return;
        }
        self.ini_time = time;
        let cfg = Config::load(&self.ini);
        crate::log::set_enabled(cfg.log);
        shared().cfg = cfg;
        logf!("config: ini re-read");
    }

    /// True while a fade plate covers the screen.
    fn screen_covered(&mut self) -> bool {
        let plate = darkest_fade_plate().filter(|&(_, alpha)| alpha > FADE_DARK);
        let Some((index, alpha)) = plate else {
            if self.dark_since.take().is_some() {
                logf!("game: fade cleared");
            }
            self.fade_ignored = false;
            return false;
        };
        let since = *self.dark_since.get_or_insert_with(|| {
            logf!("game: fade plate {index} covers the screen, alpha {alpha:.2}");
            Instant::now()
        });
        if !self.fade_ignored && since.elapsed() > FADE_TIMEOUT {
            self.fade_ignored = true;
            logf!("game: fade plate {index} stays dark, ignored until it clears");
        }
        !self.fade_ignored
    }

    pub fn tick(&mut self) {
        self.reload_ini();
        let (show_delay, hide_during_fade, hide_in_menus) = {
            let shared = shared();
            (shared.cfg.show_delay, shared.cfg.hide_during_fade, shared.cfg.hide_in_menus)
        };
        let Some(mut reading) = read() else {
            if self.was_visible {
                logf!("game: no player, panel hidden");
            }
            self.was_visible = false;
            self.ready_since = None;
            shared().stats.visible = false;
            return;
        };

        if self.last_hud_state != Some(reading.hud_state) {
            logf!("game: hud state {:?}", reading.hud_state);
            self.last_hud_state = Some(reading.hud_state);
        }
        let rune = (reading.rune.0, reading.rune.1, reading.stats.rune_active);
        if self.last_rune != Some(rune) {
            logf!(
                "game: great rune slot handle {:#010x} index {}, equipped {}, active {}",
                rune.0,
                rune.1,
                reading.stats.great_rune,
                rune.2
            );
            self.last_rune = Some(rune);
        }

        let covered = self.screen_covered() && hide_during_fade;
        // `Default` is plain play; `ShowAll` and `PopupMenu` mean a menu or a popup is open.
        let in_menu = hide_in_menus && reading.hud_state != CSFeManHudState::Default;
        let ready = reading.hud_state != CSFeManHudState::HideAll && !in_menu && !covered;
        if !ready {
            self.ready_since = None;
        }
        // The game brings its bars in a moment after it hands over control; the panel waits as long.
        let since = *self.ready_since.get_or_insert_with(Instant::now);
        reading.stats.visible = ready && since.elapsed().as_secs_f32() >= show_delay;

        if reading.stats.visible && !self.was_visible {
            logf!(
                "game: panel shown, maximum hp {} fp {} stamina {}, hud hp {}",
                reading.stats.hp_max,
                reading.stats.fp_max,
                reading.stats.stamina_max,
                reading.hud_hp_max
            );
        } else if !reading.stats.visible && self.was_visible {
            logf!("game: panel hidden");
        }
        self.was_visible = reading.stats.visible;
        shared().stats = reading.stats;
    }
}
