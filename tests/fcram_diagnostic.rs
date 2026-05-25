//! Diagnose-Tool für Badge- und Party-Offset-Triangulation.
//!
//! Lauf mit:
//! ```bash
//! cargo test --test fcram_diagnostic -- --ignored --nocapture
//! ```

use soullink_levelcap::emulator::CitraProcess;
use soullink_levelcap::memory::windows::WindowsProcessMemory;
use soullink_levelcap::memory::ProcessMemory;

const FCRAM_3DS_BASE: usize = 0x0800_0000;

// Signaturen aus dem ORAS-Save-File (PowerShell-Dump):
//   Misc-Block @ 0x4200: Money=11012 (0x2B04) + Badges=0xFF + padding
//   Party-Slot-0 @ 0x14200: PV = 0xFC645FE6 (LE: E6 5F 64 FC)
//   Party-Slot-1 @ 0x14200+484: PV = 0x406AC1F7
const MISC_SIG: &[u8] = &[0x04, 0x2B, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00];
const PV0_SIG: &[u8] = &[0xE6, 0x5F, 0x64, 0xFC];
const PV1_SIG: &[u8] = &[0xF7, 0xC1, 0x6A, 0x40];
const PV2_SIG: &[u8] = &[0x5C, 0x1C, 0x42, 0xA2];

#[test]
#[ignore = "braucht laufenden Citra-Prozess"]
fn locate_misc_and_party_in_fcram() {
    let citra = CitraProcess::find().expect("Citra läuft nicht");
    let mem = WindowsProcessMemory::open(citra.pid).expect("Citra-Memory nicht öffenbar");
    let regions = mem.regions().expect("regions");
    let region = regions
        .iter()
        .find(|r| r.base == citra.fcram_base)
        .expect("chosen region");

    eprintln!(
        "\n=== Citra PID: {}  FCRAM-Base: 0x{:016X}  Size: {} MiB ===",
        citra.pid,
        citra.fcram_base,
        region.size / 1024 / 1024
    );

    let find_pattern = |label: &str, pat: &[u8]| -> Vec<usize> {
        let chunk_size = 4 * 1024 * 1024;
        let mut hits = Vec::new();
        let mut offset = 0;
        while offset < region.size && hits.len() < 50 {
            let read_size = std::cmp::min(chunk_size, region.size - offset);
            let overlap = if offset + read_size + pat.len() <= region.size {
                pat.len()
            } else {
                0
            };
            let buf = match mem.read_bytes(citra.fcram_base + offset, read_size + overlap) {
                Ok(b) => b,
                Err(_) => {
                    offset += read_size;
                    continue;
                }
            };
            for i in 0..buf.len().saturating_sub(pat.len()) {
                if buf[i..i + pat.len()] == *pat {
                    hits.push(offset + i);
                }
            }
            offset += read_size;
        }
        eprintln!(
            "\n=== {}: pattern={} ===",
            label,
            pat.iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ")
        );
        eprintln!("  -> {} Treffer", hits.len());
        for (i, h) in hits.iter().enumerate().take(20) {
            eprintln!(
                "  [{:2}] FCRAM-Offset 0x{:08X}  3DS-Addr 0x{:08X}",
                i,
                h,
                FCRAM_3DS_BASE + h
            );
        }
        hits
    };

    let misc_hits = find_pattern("MISC-Block (Money+Badges)", MISC_SIG);
    let pv0_hits = find_pattern("Party-Slot-0 PV", PV0_SIG);
    let pv1_hits = find_pattern("Party-Slot-1 PV", PV1_SIG);
    let pv2_hits = find_pattern("Party-Slot-2 PV", PV2_SIG);

    // Cross-correlate: a party_base candidate is a PV0 hit where PV1 sits at +484 AND PV2 at +968
    eprintln!("\n=== Party-Triangulation: PV0-Hit + PV1@+484 + PV2@+968 ===");
    let pv1_set: std::collections::HashSet<usize> = pv1_hits.iter().copied().collect();
    let pv2_set: std::collections::HashSet<usize> = pv2_hits.iter().copied().collect();
    let mut party_candidates: Vec<usize> = Vec::new();
    for &h0 in &pv0_hits {
        let pv1_ok = pv1_set.contains(&(h0 + 484));
        let pv2_ok = pv2_set.contains(&(h0 + 968));
        eprintln!(
            "  PV0 @ FCRAM 0x{:08X} (3DS 0x{:08X})  PV1@+484:{}  PV2@+968:{}",
            h0,
            FCRAM_3DS_BASE + h0,
            if pv1_ok { "yes" } else { "no" },
            if pv2_ok { "yes" } else { "no" }
        );
        if pv1_ok && pv2_ok {
            party_candidates.push(h0);
        }
    }

    if let Some(misc) = misc_hits.first() {
        let badge_3ds = FCRAM_3DS_BASE + misc + 4;
        eprintln!("\n=== Summary ===");
        eprintln!("  BADGE_BYTE_OFFSET_3DS = 0x{:08X}", badge_3ds);
        if let Some(party) = party_candidates.first() {
            let party_3ds = FCRAM_3DS_BASE + party;
            eprintln!("  PARTY_BASE_3DS       = 0x{:08X}", party_3ds);
            let diff = party_3ds as isize - badge_3ds as isize;
            eprintln!("  PARTY - BADGE        = 0x{:X} ({} bytes)", diff, diff);
            eprintln!("  (citra-updater Diff: 0x894A8 — vergleichbar?)");
        }
    }
}
