use anyhow::Result;

use crate::emulator::CitraProcess;
use crate::memory::ProcessMemory;

/// Liest die Anzahl der gewonnenen Hoenn-Liga-Orden aus dem Live-Memory.
///
/// In ORAS sind die 8 Orden als Bitflags in einem Byte (bzw. Word) im
/// Trainer-Profil-Bereich gespeichert. Bit 0 = Felsorden, Bit 7 = Schemenorden.
/// Offset kommt aus `citra.badge_offset_3ds` (default oder via
/// `crate::setup::detect_offsets` auto-detected).
pub fn read_badge_count(mem: &impl ProcessMemory, citra: &CitraProcess) -> Result<u8> {
    let byte = mem.read_u8(citra.fcram_addr(citra.badge_offset_3ds))?;
    Ok(byte.count_ones() as u8)
}
