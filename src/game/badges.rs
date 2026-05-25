use anyhow::Result;

use crate::emulator::CitraProcess;
use crate::memory::ProcessMemory;

/// Offset des Orden-Bytes im 3DS-Adressraum (FCRAM-relativ) für ORAS v1.4.
///
/// Quelle: kcblack42/Citra-Tracker-v2 `citra-updater.py` Zeile 809
/// (`badgeaddress = 0x8C6DDD4` für OmegaRuby/AlphaSapphire).
///
/// **Unverifiziert** — muss mit echtem Citra + Save-States bei 0/3/8 Orden
/// gegengeprüft werden, ob hier wirklich ein Byte mit popcount = Orden-Anzahl liegt.
/// Siehe docs/OFFSETS.md R3.
pub const BADGE_BYTE_OFFSET_3DS: usize = 0x08C6_DDD4;

/// Liest die Anzahl der gewonnenen Hoenn-Liga-Orden aus dem Live-Memory.
///
/// In ORAS sind die 8 Orden als Bitflags in einem Byte (bzw. Word) im
/// Trainer-Profil-Bereich gespeichert. Bit 0 = Felsorden, Bit 7 = Schemenorden.
pub fn read_badge_count(mem: &impl ProcessMemory, citra: &CitraProcess) -> Result<u8> {
    let byte = mem.read_u8(citra.fcram_addr(BADGE_BYTE_OFFSET_3DS))?;
    Ok(byte.count_ones() as u8)
}
