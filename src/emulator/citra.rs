use anyhow::{anyhow, bail, Result};
use sysinfo::System;

use super::Emulator;

/// 3DS-FCRAM ist 128 MiB groß und im 3DS-virtuellen Adressraum bei 0x08000000 gemappt.
/// Im Host-Prozess von Citra liegt sie als ein großer anonymer RW-Block.
const FCRAM_SIZE: usize = 128 * 1024 * 1024;
const FCRAM_3DS_BASE: usize = 0x0800_0000;

pub struct CitraProcess {
    pub pid: u32,
    pub fcram_base: usize,
    /// FCRAM-relative 3DS-virtuelle Adresse des Badge-Bytes. Default-Wert kommt
    /// aus unserer Triangulation; per `crate::setup` wird er beim Start
    /// auto-detected.
    pub badge_offset_3ds: usize,
    /// FCRAM-relative 3DS-virtuelle Adresse des ersten Party-Slots (484-Stride
    /// Battle-Layout).
    pub party_base_3ds: usize,
}

impl CitraProcess {
    /// Findet Citra-Prozess + FCRAM-Base. Setzt Offsets auf Default
    /// (fbeck's Build). Fuer andere Setups: `crate::setup::detect_offsets`
    /// aufrufen + Felder ueberschreiben.
    pub fn find() -> Result<Self> {
        let pid = find_citra_pid()
            .ok_or_else(|| anyhow!("Citra-Prozess nicht gefunden. Citra mit ORAS gestartet?"))?;
        let fcram_base = find_fcram_base(pid)?;
        Ok(Self {
            pid,
            fcram_base,
            badge_offset_3ds: 0x0F48_EE14,
            party_base_3ds: 0x0F51_82BC,
        })
    }

    /// Übersetzt eine 3DS-FCRAM-Adresse (0x08000000+) in die Host-Adresse im Citra-Prozess.
    pub fn fcram_addr(&self, addr_3ds: usize) -> usize {
        debug_assert!(
            addr_3ds >= FCRAM_3DS_BASE,
            "3DS-Adresse 0x{:x} unterhalb FCRAM-Base",
            addr_3ds
        );
        self.fcram_base + (addr_3ds - FCRAM_3DS_BASE)
    }
}

impl Emulator for CitraProcess {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn fcram_addr(&self, addr_3ds: usize) -> usize {
        CitraProcess::fcram_addr(self, addr_3ds)
    }
}

fn find_citra_pid() -> Option<u32> {
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    for (pid, proc_) in sys.processes() {
        let name = proc_.name().to_string_lossy().to_lowercase();
        // Bekannte Citra-Binärnamen: citra-qt, citra, citra-qt.exe, Citra.exe, citra-room
        // (`citra-room` wollen wir explizit NICHT — das ist der Multiplayer-Lobby-Server.)
        if name.starts_with("citra") && !name.contains("room") {
            return Some(pid.as_u32());
        }
    }
    None
}

/// Findet die FCRAM-Base-Adresse im Citra-Prozess.
///
/// TODO(R1): Verifizierung mit echtem Citra-Prozess.
/// Aktuell: größten anonymen RW-Block (~128 MiB) suchen.
#[cfg(target_os = "linux")]
pub fn find_fcram_base(pid: u32) -> Result<usize> {
    use crate::memory::linux::LinuxProcessMemory;

    let mem = LinuxProcessMemory::open(pid)?;
    let candidates: Vec<_> = mem
        .maps()?
        .into_iter()
        .filter(|m| m.is_anonymous_rw() && m.size() >= FCRAM_SIZE)
        .collect();

    match candidates.len() {
        0 => bail!(
            "Kein anonymer RW-Block ≥ {} MiB im Prozess {} gefunden — Citra läuft, aber kein Spiel geladen?",
            FCRAM_SIZE / 1024 / 1024,
            pid
        ),
        1 => Ok(candidates[0].start),
        n => {
            // Mehrdeutigkeit: User-sichtbare Diagnose, dann den am besten passenden (≈128 MiB) wählen.
            eprintln!(
                "[WARN] {} FCRAM-Kandidaten gefunden — wähle den mit Größe am nächsten an {} MiB",
                n,
                FCRAM_SIZE / 1024 / 1024
            );
            let best = candidates
                .iter()
                .min_by_key(|m| m.size().abs_diff(FCRAM_SIZE))
                .unwrap();
            Ok(best.start)
        }
    }
}

#[cfg(target_os = "windows")]
pub fn find_fcram_base(pid: u32) -> Result<usize> {
    use crate::memory::windows::WindowsProcessMemory;

    let mem = WindowsProcessMemory::open(pid)?;
    let candidates: Vec<_> = mem
        .regions()?
        .into_iter()
        .filter(|r| r.size >= FCRAM_SIZE)
        .collect();

    match candidates.len() {
        0 => bail!(
            "Kein RW-Block ≥ {} MiB im Prozess {} gefunden — Citra läuft, aber kein Spiel geladen?",
            FCRAM_SIZE / 1024 / 1024,
            pid
        ),
        _ => {
            let best = candidates
                .iter()
                .min_by_key(|r| r.size.abs_diff(FCRAM_SIZE))
                .unwrap();
            Ok(best.base)
        }
    }
}
