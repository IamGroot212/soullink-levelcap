//! Auto-Triangulation der RAM-Offsets via Save-File-Signaturen.
//!
//! ORAS-Save (`main` in Citras sdmc) hat ein bekanntes festes Layout:
//!   - Misc-Block @ file-offset 0x4200 enthaelt Money(u32 LE)+Badges(u8)+padding
//!   - Party @ file-offset 0x14200 mit 260-Byte-Stride enthaelt 6 PB6-Slots,
//!     jeder mit Encryption-Constant (EC) als ersten 4 Bytes
//!
//! Bei laufendem Citra liegen dieselben Daten irgendwo in FCRAM. Wir scannen
//! die Citra-Region nach diesen Signaturen und leiten die RAM-Offsets ab.

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

use crate::memory::ProcessMemory;

const FCRAM_3DS_BASE: usize = 0x0800_0000;
const SAV_BADGES_OFFSET: usize = 0x4200 + 0x0C; // Misc-Block + Badge-Field
const SAV_PARTY_OFFSET: usize = 0x14200;
const SAV_PARTY_STRIDE: usize = 260;
const RAM_PARTY_STRIDE: usize = 484; // 484-Stride Battle-Layout
/// Konstanter Abstand Party-Base → Badge-Byte in ORAS-EUR Battle-Layout (RAM).
/// Empirisch verifiziert: PARTY 0x0F5182BC, BADGE 0x0F48EE14 → diff 0x294A8.
const PARTY_TO_BADGE_OFFSET: usize = 0x2_94A8;

#[derive(Debug, Clone, Copy)]
pub struct DetectedOffsets {
    pub badge_offset_3ds: usize,
    pub party_base_3ds: usize,
}

/// Versucht Citras `main` Save-File in den Standard-Pfaden zu finden.
/// Citra speichert ORAS-Saves unter:
///   `<sdmc>/Nintendo 3DS/<id0>/<id1>/title/00040000/0011c???/data/00000001/main`
/// wo id0/id1 typischerweise lauter Nullen sind (Citra hat keinen echten 3DS).
pub fn find_sav_path() -> Result<PathBuf> {
    let sdmc = citra_sdmc_root().ok_or_else(|| {
        anyhow!("Konnte Citra-sdmc-Verzeichnis nicht finden. Bitte --sav-path angeben.")
    })?;
    let nin = sdmc.join("Nintendo 3DS");
    if !nin.exists() {
        bail!(
            "Citra-sdmc-Verzeichnis '{}' existiert nicht. Citra schon mal mit ORAS gestartet?",
            nin.display()
        );
    }
    // Walk: Nintendo 3DS / <id0> / <id1> / title / 00040000 / 0011c??? / data / 00000001 / main
    for id0 in std::fs::read_dir(&nin)? {
        let id0 = id0?.path();
        if !id0.is_dir() {
            continue;
        }
        for id1 in std::fs::read_dir(&id0)? {
            let id1 = id1?.path();
            if !id1.is_dir() {
                continue;
            }
            let titles = id1.join("title/00040000");
            if !titles.is_dir() {
                continue;
            }
            for title in std::fs::read_dir(&titles)? {
                let title = title?.path();
                let name = title
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                // ORAS-Title-IDs: 0011c400 (US), 0011c500 (EU), 0011c401 (JP) — alle 0011c???
                if !name.starts_with("0011c") {
                    continue;
                }
                let main = title.join("data/00000001/main");
                if main.is_file() {
                    return Ok(main);
                }
            }
        }
    }
    bail!(
        "Kein ORAS-Save in {} gefunden. Citra schon mal mit ORAS gestartet & in-game gespeichert?",
        nin.display()
    );
}

fn citra_sdmc_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("Citra/sdmc"))
    }
    #[cfg(target_os = "linux")]
    {
        // Standard XDG: ~/.local/share/citra-emu/sdmc
        let home = std::env::var_os("HOME")?;
        let path = PathBuf::from(home).join(".local/share/citra-emu/sdmc");
        if path.exists() {
            Some(path)
        } else {
            // Alternativ: ~/.var/app/org.citra_emu.citra/data/citra-emu/sdmc (Flatpak)
            std::env::var_os("HOME").map(|home| {
                PathBuf::from(home).join(".var/app/org.citra_emu.citra/data/citra-emu/sdmc")
            })
        }
    }
}

/// Liest .sav-Datei, extrahiert Signaturen, scannt FCRAM, gibt Offsets zurueck.
///
/// Voraussetzungen:
///   - .sav-Datei existiert und hat die Default-ORAS-Struktur
///   - Citra-Prozess hat das Spiel geladen (FCRAM ist mit Game-Daten gefuellt)
pub fn detect_offsets(
    sav_path: &Path,
    mem: &impl ProcessMemory,
    fcram_base: usize,
    fcram_size: usize,
) -> Result<DetectedOffsets> {
    let sav = std::fs::read(sav_path)
        .with_context(|| format!("Save-File lesen: {}", sav_path.display()))?;
    if sav.len() < SAV_PARTY_OFFSET + 260 * 6 {
        bail!(
            "Save-File zu klein ({} bytes) — kein gueltiges ORAS-Save?",
            sav.len()
        );
    }

    // Misc-Block-Detection ueber Signatur funktioniert NICHT zuverlaessig:
    // bytes [0..8] des Misc-Blocks sind block-spezifische Checksum die das
    // Spiel im RAM anders haelt als auf Disk. Steam-Deck-User-Logs vom
    // 2026-05-28 bestaetigt: Signatur aus .sav nicht in RAM gefunden.
    //
    // Stattdessen: Party-Base via EC-Scan finden (= zuverlaessig), Badge
    // dann ueber konstanten Abstand `PARTY_TO_BADGE_OFFSET` ableiten.
    //
    // Erwarteter Badge-Wert aus .sav (Sanity-Check):
    let expected_badge_byte = sav[SAV_BADGES_OFFSET];
    let expected_badges = expected_badge_byte.count_ones();

    // Party-EC-Signaturen: erste 4 Bytes jedes belegten Party-Slots
    let mut party_ecs: Vec<[u8; 4]> = Vec::new();
    for slot in 0..6 {
        let off = SAV_PARTY_OFFSET + slot * SAV_PARTY_STRIDE;
        let ec_bytes: [u8; 4] = sav[off..off + 4].try_into().unwrap();
        let ec_u32 = u32::from_le_bytes(ec_bytes);
        if ec_u32 != 0 {
            party_ecs.push(ec_bytes);
        }
    }
    if party_ecs.is_empty() {
        bail!("Keine belegten Party-Slots im Save gefunden. Hast du schon ein Starter?");
    }

    // Party-Base finden: scan nach Slot-0 EC, dann triangulate via Slot-1+
    let slot0_hits = scan_region(mem, fcram_base, fcram_size, &party_ecs[0])?;
    if slot0_hits.is_empty() {
        bail!("Party Slot-0 EC nicht in FCRAM gefunden.");
    }

    let party_base_3ds = if party_ecs.len() >= 2 {
        // Cross-correlate: slot 0 EC at offset X, slot 1 EC at X+484 (und ggf.
        // slot 2 EC at X+968). Mehrere Treffer = mehrere 484-Stride-Kopien
        // (Player-Party, Battle-Opponent, Battle-Wild). Player-Party
        // erkennen wir durch Triangulation mit ALLEN bekannten EC-Slots.
        let mut candidates: Vec<usize> = slot0_hits.clone();
        for (slot_idx, ec) in party_ecs.iter().enumerate().skip(1) {
            if candidates.len() <= 1 {
                break;
            }
            let other_hits: std::collections::HashSet<usize> =
                scan_region(mem, fcram_base, fcram_size, ec)?
                    .into_iter()
                    .collect();
            let expected_offset = slot_idx * RAM_PARTY_STRIDE;
            candidates.retain(|&h| other_hits.contains(&(h + expected_offset)));
        }
        match candidates.len() {
            0 => bail!(
                "Keine Party-Triangulation: Slot 0 EC hat {} Hits, aber kein Hit hat alle anderen Slots an erwarteten Offsets.",
                slot0_hits.len()
            ),
            1 => FCRAM_3DS_BASE + candidates[0],
            n => {
                eprintln!(
                    "[WARN] {n} Party-Triangulationen nach Cross-Korrelation aller {} Slots — \
                     nehme die mit hoechster Adresse (heuristisch: Player-Party meist nach Save-Block-Kopie).",
                    party_ecs.len()
                );
                FCRAM_3DS_BASE + *candidates.iter().max().unwrap()
            }
        }
    } else {
        // Nur 1 Pokemon — koennen nicht cross-korrelieren, nimm einzelnen Hit
        if slot0_hits.len() != 1 {
            eprintln!(
                "[WARN] Nur 1 Pokemon im Save, {} EC-Hits in FCRAM, nehme den ersten.",
                slot0_hits.len()
            );
        }
        FCRAM_3DS_BASE + slot0_hits[0]
    };

    // Badge-Offset relativ zu Party-Base (ORAS-EUR Game-internes Struct-Layout).
    let badge_offset_3ds = party_base_3ds
        .checked_sub(PARTY_TO_BADGE_OFFSET)
        .ok_or_else(|| {
            anyhow!(
                "PARTY_BASE 0x{:08X} - 0x{:X} unterlaeuft FCRAM",
                party_base_3ds,
                PARTY_TO_BADGE_OFFSET
            )
        })?;

    // Sanity-Check: gelesener Badge-Wert sollte popcount >= .sav-Wert haben
    // (Spielprogress ist monoton). Bei groesserer Abweichung warnen.
    let ram_addr_in_host = fcram_base + (badge_offset_3ds - FCRAM_3DS_BASE);
    if let Ok(ram_badge) = mem.read_bytes(ram_addr_in_host, 1) {
        let ram_badges = ram_badge[0].count_ones();
        if ram_badges < expected_badges {
            eprintln!(
                "[WARN] Badge-Sanity-Check: RAM={} Orden, .sav={} Orden. \
                 Party-Base ist evtl. nicht die Battle-Party-Kopie. \
                 Triangulation versuchen wir trotzdem.",
                ram_badges, expected_badges
            );
        }
    }

    Ok(DetectedOffsets {
        badge_offset_3ds,
        party_base_3ds,
    })
}

fn scan_region(
    mem: &impl ProcessMemory,
    base: usize,
    size: usize,
    pattern: &[u8],
) -> Result<Vec<usize>> {
    let chunk = 4 * 1024 * 1024;
    let mut hits = Vec::new();
    let mut off = 0;
    while off < size {
        let read_size = chunk.min(size - off);
        let overlap = if off + read_size + pattern.len() <= size {
            pattern.len()
        } else {
            0
        };
        let buf = match mem.read_bytes(base + off, read_size + overlap) {
            Ok(b) => b,
            Err(_) => {
                off += read_size;
                continue;
            }
        };
        for i in 0..buf.len().saturating_sub(pattern.len()) {
            if buf[i..i + pattern.len()] == *pattern {
                hits.push(off + i);
            }
        }
        off += read_size;
    }
    Ok(hits)
}
