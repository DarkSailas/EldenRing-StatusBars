use std::{
    path::PathBuf,
    sync::{
        LazyLock, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use hudhook::windows::Win32::{Foundation::HMODULE, System::LibraryLoader::GetModuleFileNameW};

use crate::config::Config;

/// Set after a panic inside the mod: the game task and the overlay stop doing anything.
pub static DISABLED: AtomicBool = AtomicBool::new(false);

static MODULE: AtomicUsize = AtomicUsize::new(0);

/// Written by the game task, read by the overlay.
#[derive(Clone, Default)]
pub struct Stats {
    /// The game shows its own bars right now.
    pub visible: bool,
    pub hp: f32,
    pub hp_max: f32,
    pub fp: f32,
    pub fp_max: f32,
    pub stamina: f32,
    pub stamina_max: f32,
    pub name: String,
    pub level: u32,
    /// A Great Rune is equipped.
    pub great_rune: bool,
    /// The equipped Great Rune is powered by a Rune Arc.
    pub rune_active: bool,
}

pub struct Shared {
    pub cfg: Config,
    pub stats: Stats,
}

static SHARED: LazyLock<Mutex<Shared>> = LazyLock::new(|| {
    Mutex::new(Shared {
        cfg: Config::default(),
        stats: Stats::default(),
    })
});

pub fn shared() -> MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_module(raw: usize) {
    MODULE.store(raw, Ordering::Relaxed);
}

fn dll_dir() -> PathBuf {
    let mut buf = [0u16; 1024];
    let module = HMODULE(MODULE.load(Ordering::Relaxed) as _);
    let len = unsafe { GetModuleFileNameW(Some(module), &mut buf) } as usize;
    let mut path = PathBuf::from(String::from_utf16_lossy(&buf[..len.min(buf.len())]));
    path.pop();
    path
}

pub fn ini_path() -> PathBuf {
    dll_dir().join("er_status_bars.ini")
}

pub fn log_path() -> PathBuf {
    dll_dir().join("er_status_bars.log")
}
