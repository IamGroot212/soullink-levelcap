use anyhow::Result;

use crate::emulator::CitraProcess;
use crate::memory::ProcessMemory;

/// Offset der Party-Base im 3DS-Adressraum (FCRAM-relativ).
///
/// TODO(R2): Aktueller Wert ist ein **PLATZHALTER** aus der Brief-Hypothese
/// (~0x8C861C8 für ORAS v1.4) — vor v0.1.0 verifizieren. Siehe docs/OFFSETS.md.
pub const PARTY_BASE_3DS: usize = 0x08C8_61C8; // TODO: verifizieren

/// Größe der dekrypteten Pokémon-Battle-Struktur im Party-Bereich (Gen 6).
pub const POKEMON_SIZE: usize = 484;

/// Maximale Slot-Anzahl in der Party.
pub const PARTY_SIZE: u8 = 6;

// Offsets innerhalb der 484-Byte-Battle-Struct (TODO(R4): verifizieren via PB6.cs)
const OFF_SPECIES: usize = 0x08;
const OFF_EXP: usize = 0x10;
const OFF_LEVEL: usize = 0xE0;

#[derive(Debug, Clone, Copy)]
pub struct PartyPokemon {
    pub slot: u8,
    pub species: u16,
    pub level: u8,
    pub exp: u32,
}

impl PartyPokemon {
    pub fn read(
        mem: &impl ProcessMemory,
        citra: &CitraProcess,
        slot: u8,
    ) -> Result<Option<Self>> {
        let base = citra.fcram_addr(PARTY_BASE_3DS + (slot as usize) * POKEMON_SIZE);
        let species = mem.read_u16_le(base + OFF_SPECIES)?;
        if species == 0 {
            return Ok(None); // Leerer Slot
        }
        let exp = mem.read_u32_le(base + OFF_EXP)?;
        let level = mem.read_u8(base + OFF_LEVEL)?;
        Ok(Some(Self { slot, species, level, exp }))
    }

    pub fn write_exp(
        &self,
        mem: &impl ProcessMemory,
        citra: &CitraProcess,
        new_exp: u32,
    ) -> Result<()> {
        let base = citra.fcram_addr(PARTY_BASE_3DS + (self.slot as usize) * POKEMON_SIZE);
        mem.write_u32_le(base + OFF_EXP, new_exp)
    }
}
