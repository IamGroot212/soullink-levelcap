use anyhow::{bail, Result};

use crate::emulator::CitraProcess;
use crate::game::decrypt::{decrypt_slot, encrypt_slot};
use crate::game::growth_rates::{growth_rate_of, level_from_exp};
use crate::memory::ProcessMemory;

/// Stride zwischen Party-Slots in Bytes (Gen-6 Battle-Layout).
/// **Party-Base-Adresse** liegt in `citra.party_base_3ds` (default oder
/// via `crate::setup::detect_offsets` auto-detected).
pub const POKEMON_SIZE: usize = 484;

/// Maximale Slot-Anzahl in der Party.
pub const PARTY_SIZE: u8 = 6;

/// 484-Layout: 232 Byte Encrypted-Blocks + 112 Byte Battle-Status-Gap + 22 Byte
/// Stats (verschluesselt) + 118 Reserve. Decrypt nutzt 232 + 22 = 254 Bytes
/// konkateniert. Battle-Status (slot+232..slot+344) wird uebersprungen.
const PKM_BLOCKS_LEN: usize = 232;
const STATS_START: usize = 344;
const STATS_LEN: usize = 22;
const CONCAT_LEN: usize = PKM_BLOCKS_LEN + STATS_LEN; // 254

// Offsets im decrypteten 254-Byte-Konkat (Block A nach Unshuffle).
const PLAIN_SPECIES: usize = 0x08; // Block A: Species (u16 LE)
const PLAIN_EXP: usize = 0x10; // Block A: EXP (u32 LE)

/// PB6 Block-Checksum (PKHeX `RefreshChecksum`): u16-Summe ueber decrypted+unshuffled
/// Bytes [0x08..0xE8] modulo 2^16, in plain[0x06..0x08] gespeichert.
fn pb6_block_checksum(plain: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0x08;
    while i + 1 < 0xE8 {
        sum = sum.wrapping_add(u16::from_le_bytes([plain[i], plain[i + 1]]) as u32);
        i += 2;
    }
    (sum & 0xFFFF) as u16
}
// HINWEIS: Level steht in PB6 bei plain[0xEC] (= stats[4]), aber das Stats-Decrypt
// liefert in unserem ORAS-Build keinen brauchbaren Wert. Wir berechnen Level
// stattdessen aus EXP + Growth-Rate (siehe `growth_rates::level_from_exp`).

#[derive(Debug, Clone, Copy)]
pub struct PartyPokemon {
    pub slot: u8,
    pub species: u16,
    pub level: u8,
    pub exp: u32,
}

impl PartyPokemon {
    /// Liest den Party-Slot, dekryptet ihn, und gibt die Felder zurück.
    /// `Ok(None)` wenn der Slot leer ist (PV == 0).
    pub fn read(mem: &impl ProcessMemory, citra: &CitraProcess, slot: u8) -> Result<Option<Self>> {
        let base = citra.fcram_addr(citra.party_base_3ds + (slot as usize) * POKEMON_SIZE);

        let blocks = mem.read_bytes(base, PKM_BLOCKS_LEN)?;
        if blocks.len() != PKM_BLOCKS_LEN {
            bail!(
                "kurzer Read bei Slot {}: got {} bytes statt {}",
                slot,
                blocks.len(),
                PKM_BLOCKS_LEN
            );
        }

        let pv = u32::from_le_bytes([blocks[0], blocks[1], blocks[2], blocks[3]]);
        if pv == 0 {
            return Ok(None);
        }
        // Sanity-Bytes (PB6 +0x04, u16 LE) müssen 0 sein für valide PKM.
        // Slots mit sanity != 0 sind korrumpiert / hacked / nicht im Standard-Format.
        let sanity = u16::from_le_bytes([blocks[4], blocks[5]]);
        if sanity != 0 {
            return Ok(None);
        }

        let stats = mem.read_bytes(base + STATS_START, STATS_LEN)?;
        if stats.len() != STATS_LEN {
            bail!(
                "kurzer Read der Stats bei Slot {}: got {} bytes statt {}",
                slot,
                stats.len(),
                STATS_LEN
            );
        }

        let mut concat = [0u8; CONCAT_LEN];
        concat[..PKM_BLOCKS_LEN].copy_from_slice(&blocks);
        concat[PKM_BLOCKS_LEN..].copy_from_slice(&stats);

        let plain = decrypt_slot(&concat);

        let species = u16::from_le_bytes([plain[PLAIN_SPECIES], plain[PLAIN_SPECIES + 1]]);
        // Gen 1-6 hat 721 Species. Außerhalb 1..=721 = Garbage / nicht-Gen6-Pokemon.
        if species == 0 || species > 721 {
            return Ok(None);
        }
        let exp = u32::from_le_bytes([
            plain[PLAIN_EXP],
            plain[PLAIN_EXP + 1],
            plain[PLAIN_EXP + 2],
            plain[PLAIN_EXP + 3],
        ]);
        // EXP über 1.640.000 ist über Lvl 100 in jedem Growth-Rate → Garbage.
        if exp > 1_640_000 {
            return Ok(None);
        }
        // Level aus EXP herleiten statt aus plain[0xEC] (siehe Hinweis oben).
        let level = level_from_exp(exp, growth_rate_of(species));

        Ok(Some(Self {
            slot,
            species,
            level,
            exp,
        }))
    }

    /// Schreibt einen neuen EXP-Wert zurück: read → decrypt → patch EXP → encrypt → write.
    pub fn write_exp(
        &self,
        mem: &impl ProcessMemory,
        citra: &CitraProcess,
        new_exp: u32,
    ) -> Result<()> {
        let base = citra.fcram_addr(citra.party_base_3ds + (self.slot as usize) * POKEMON_SIZE);

        let blocks = mem.read_bytes(base, PKM_BLOCKS_LEN)?;
        let stats = mem.read_bytes(base + STATS_START, STATS_LEN)?;
        let mut concat = [0u8; CONCAT_LEN];
        concat[..PKM_BLOCKS_LEN].copy_from_slice(&blocks);
        concat[PKM_BLOCKS_LEN..].copy_from_slice(&stats);

        let mut plain = decrypt_slot(&concat);
        plain[PLAIN_EXP..PLAIN_EXP + 4].copy_from_slice(&new_exp.to_le_bytes());

        // PB6 Checksum bei +0x06 (u16 LE) ueber u16-Summe der Block-Region 0x08..0xE8
        // neu berechnen — sonst markiert das Spiel das Pokemon als Egg (Bug 2026-05-26).
        let checksum = pb6_block_checksum(&plain);
        plain[0x06..0x08].copy_from_slice(&checksum.to_le_bytes());

        let enc = encrypt_slot(&plain);
        mem.write_bytes(base, &enc[..PKM_BLOCKS_LEN])?;
        mem.write_bytes(base + STATS_START, &enc[PKM_BLOCKS_LEN..])?;
        Ok(())
    }
}
