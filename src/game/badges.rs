use anyhow::Result;

use crate::emulator::CitraProcess;
use crate::memory::ProcessMemory;

/// Offset des Orden-Bytes im 3DS-Adressraum (FCRAM-relativ) für ORAS v1.4.
///
/// **Triangulation via PKHeX-Misc-Block-Signatur** (Phase A, 2026-05-25):
/// Save-File-Misc-Block @ 0x4200 enthält Money(u32)+Badges(u8) als
/// distinkte Signatur. Scan in Citras 256 MiB FCRAM-Buffer fand einen
/// einzigen Treffer bei FCRAM-Offset 0x0748EE14 → 3DS-Adresse 0x0F48EE14.
///
/// citra-updater.py:1057 nennt `0x8C6DDD4`; bei diesem Citra-Build
/// (citra-windows-msvc-20240303-0ff3440) + randomisierter CXI ist die
/// Layout-Position aber komplett anders. Siehe docs/OFFSETS.md.
pub const BADGE_BYTE_OFFSET_3DS: usize = 0x0F48_EE14;

/// Liest die Anzahl der gewonnenen Hoenn-Liga-Orden aus dem Live-Memory.
///
/// In ORAS sind die 8 Orden als Bitflags in einem Byte (bzw. Word) im
/// Trainer-Profil-Bereich gespeichert. Bit 0 = Felsorden, Bit 7 = Schemenorden.
pub fn read_badge_count(mem: &impl ProcessMemory, citra: &CitraProcess) -> Result<u8> {
    let byte = mem.read_u8(citra.fcram_addr(BADGE_BYTE_OFFSET_3DS))?;
    Ok(byte.count_ones() as u8)
}
