use anyhow::Result;

use crate::emulator::CitraProcess;
use crate::memory::ProcessMemory;

/// Offset der Party-Base im 3DS-Adressraum (FCRAM-relativ) für ORAS v1.4.
///
/// Quelle: kcblack42/Citra-Tracker-v2 `citra-updater.py` Zeile 799
/// (`partyaddress = 0x8CF727C` für OmegaRuby/AlphaSapphire).
///
/// **Unverifiziert** für unseren Use-Case — der Tracker liest hier erfolgreich,
/// aber die Daten sind **verschlüsselt** (XOR mit PV-derived Seed + Block-Shuffle).
/// Siehe docs/OFFSETS.md R2 + "Gen-6-Encryption".
pub const PARTY_BASE_3DS: usize = 0x08CF_727C;

/// Größe eines Party-Slots in Gen 6 (verschlüsselter 232-Byte PKM-Block + 112 Byte Battle-Status + 22 Byte Stats + 118 Byte Reserve = 484 Bytes Stride).
/// Quelle: Citra-Tracker `SLOT_OFFSET = 484` (citra-updater.py:2428) und PKHeX `PB6.cs`.
pub const POKEMON_SIZE: usize = 484;

/// Maximale Slot-Anzahl in der Party.
pub const PARTY_SIZE: u8 = 6;

// Offsets innerhalb des 484-Byte-Slots.
// WICHTIG: 0..232 ist verschlüsselt (PKM-Daten). EXP liegt im Block A bei decrypted-offset 0x10,
// aber Block A ist nach Shuffle nicht zwingend an Slot-Position 8 — der Shuffle-Index hängt vom PV ab.
// Der Tracker liest Level aus dem dekrypteten konkatenierten (party + stats) Stream an offset 0xEC.
// Im absoluten Slot bei +344+4 = +348, immer noch XOR-verschlüsselt.
// TODO(R4): verschlüsselte Lese-/Schreib-Routine implementieren (siehe docs/OFFSETS.md "Encryption").
const OFF_SPECIES_ENCRYPTED: usize = 0x08; // nach decrypt + unshuffle: Block A start
const OFF_EXP_ENCRYPTED: usize = 0x10; // nach decrypt + unshuffle: Block A + 8
const OFF_LEVEL_STATS: usize = 0xEC; // im konkatenierten Stream nach decrypt (= absolute Slot-Offset 348)

// Alte Konstantennamen für Kompilation behalten (mit Warnung): aktueller Code liest unverschlüsselt — wird beim ersten Live-Test bullshit zurückgeben.
const OFF_SPECIES: usize = OFF_SPECIES_ENCRYPTED;
const OFF_EXP: usize = OFF_EXP_ENCRYPTED;
const OFF_LEVEL: usize = OFF_LEVEL_STATS;

#[derive(Debug, Clone, Copy)]
pub struct PartyPokemon {
    pub slot: u8,
    pub species: u16,
    pub level: u8,
    pub exp: u32,
}

impl PartyPokemon {
    pub fn read(mem: &impl ProcessMemory, citra: &CitraProcess, slot: u8) -> Result<Option<Self>> {
        let base = citra.fcram_addr(PARTY_BASE_3DS + (slot as usize) * POKEMON_SIZE);
        let species = mem.read_u16_le(base + OFF_SPECIES)?;
        if species == 0 {
            return Ok(None); // Leerer Slot
        }
        let exp = mem.read_u32_le(base + OFF_EXP)?;
        let level = mem.read_u8(base + OFF_LEVEL)?;
        Ok(Some(Self {
            slot,
            species,
            level,
            exp,
        }))
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
