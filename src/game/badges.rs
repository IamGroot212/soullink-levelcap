use anyhow::Result;

use crate::emulator::CitraProcess;
use crate::memory::ProcessMemory;

/// Offset des Orden-Bytes im 3DS-Adressraum (FCRAM-relativ).
///
/// TODO(R3): Aktueller Wert ist ein **PLATZHALTER** — vor v0.1.0 mit `scanmem`
/// / Cheat Engine triangulieren. Siehe docs/OFFSETS.md.
pub const BADGE_BYTE_OFFSET_3DS: usize = 0x0800_0000; // TODO: ersetzen

/// Liest die Anzahl der gewonnenen Hoenn-Liga-Orden aus dem Live-Memory.
///
/// In ORAS sind die 8 Orden als Bitflags in einem Byte (bzw. Word) im
/// Trainer-Profil-Bereich gespeichert. Bit 0 = Felsorden, Bit 7 = Schemenorden.
pub fn read_badge_count(mem: &impl ProcessMemory, citra: &CitraProcess) -> Result<u8> {
    let byte = mem.read_u8(citra.fcram_addr(BADGE_BYTE_OFFSET_3DS))?;
    Ok((byte & 0xFF).count_ones() as u8)
}
