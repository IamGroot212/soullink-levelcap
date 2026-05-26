use anyhow::Result;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use crate::caps::CapTable;
use crate::emulator::CitraProcess;
use crate::game::{
    growth_rates::growth_rate_of, min_exp_for_level, read_badge_count, PartyPokemon, PARTY_SIZE,
};
use crate::memory::{DefaultProcessMemory, ProcessMemory};
use crate::setup;

const TICK_INTERVAL: Duration = Duration::from_millis(500);
const RECONNECT_BACKOFF: Duration = Duration::from_secs(2);
const FCRAM_SIZE: usize = 256 * 1024 * 1024; // New3DS Extended FCRAM

pub fn run(caps: CapTable, sav_path_override: Option<PathBuf>) -> Result<()> {
    loop {
        println!("[soullink-levelcap] Suche Citra-Prozess...");
        let mut citra = match CitraProcess::find() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[WARN] {e} — neuer Versuch in {:?}", RECONNECT_BACKOFF);
                sleep(RECONNECT_BACKOFF);
                continue;
            }
        };
        println!(
            "[soullink-levelcap] Citra gefunden (PID: {}, FCRAM @ 0x{:x})",
            citra.pid, citra.fcram_base
        );

        let mem = match DefaultProcessMemory::open(citra.pid) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[ERROR] Kann Citra-Memory nicht öffnen: {e}");
                sleep(RECONNECT_BACKOFF);
                continue;
            }
        };

        // Auto-Triangulation der Offsets via .sav-Signaturen.
        let sav_path = match sav_path_override.clone() {
            Some(p) => p,
            None => match setup::find_sav_path() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!(
                        "[ERROR] .sav-Pfad nicht auto-gefunden: {e}\n        \
                         Mit --sav-path angeben."
                    );
                    sleep(RECONNECT_BACKOFF);
                    continue;
                }
            },
        };
        println!("[INFO] Save-File: {}", sav_path.display());

        match setup::detect_offsets(&sav_path, &mem, citra.fcram_base, FCRAM_SIZE) {
            Ok(off) => {
                println!(
                    "[INFO] Offsets detected: BADGE=0x{:08X}, PARTY=0x{:08X}",
                    off.badge_offset_3ds, off.party_base_3ds
                );
                citra.badge_offset_3ds = off.badge_offset_3ds;
                citra.party_base_3ds = off.party_base_3ds;
            }
            Err(e) => {
                eprintln!(
                    "[WARN] Offset-Detection fehlgeschlagen: {e}\n       \
                     Verwende Default-Offsets — funktioniert nur fuer fbeck's Citra-Build."
                );
            }
        }

        run_loop(&mem, &citra, &caps);

        eprintln!("[INFO] Verbindung zu Citra verloren — versuche neu...");
        sleep(RECONNECT_BACKOFF);
    }
}

fn run_loop(mem: &impl ProcessMemory, citra: &CitraProcess, caps: &CapTable) {
    let mut last_badges: Option<u8> = None;

    loop {
        match tick(mem, citra, caps, &mut last_badges) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("[WARN] Tick fehlgeschlagen: {e}");
                return; // Reconnect-Schleife im Aufrufer übernimmt
            }
        }
        sleep(TICK_INTERVAL);
    }
}

fn tick(
    mem: &impl ProcessMemory,
    citra: &CitraProcess,
    caps: &CapTable,
    last_badges: &mut Option<u8>,
) -> Result<()> {
    let badges = read_badge_count(mem, citra)?;
    let cap = caps.cap_for(badges)?;

    if last_badges.replace(badges) != Some(badges) {
        println!("[INFO] Orden-Anzahl: {} → Cap: Level {}", badges, cap);
    }

    for slot in 0..PARTY_SIZE {
        let Some(pkmn) = PartyPokemon::read(mem, citra, slot)? else {
            continue;
        };
        if pkmn.level < cap {
            continue;
        }

        // Overshoot-Policy: friere auf dem **aktuellen** Level ein (nicht auf cap),
        // damit kein "Rückwärts-De-Level" passiert, falls jemand vor dem Daemon-Start drüber war.
        let freeze_level = pkmn.level.max(cap);
        let max_exp = min_exp_for_level(freeze_level, growth_rate_of(pkmn.species));

        if pkmn.exp > max_exp {
            pkmn.write_exp(mem, citra, max_exp)?;
            println!(
                "[CAP] Slot {} (species {}) auf Lvl {} eingefroren (EXP {} → {})",
                pkmn.slot, pkmn.species, pkmn.level, pkmn.exp, max_exp
            );
        }
    }

    Ok(())
}
