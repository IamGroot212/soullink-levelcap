//! Gen-6 Party-Slot Encryption/Decryption für ORAS.
//!
//! Referenz: kcblack42/Citra-Tracker-v2 `citra-updater.py:100-181`
//! (`crypt`, `crypt_array`, `shuffle_array`, `decrypt_data`).
//!
//! Ein Party-Slot ist 484 Bytes. Nur 254 Bytes davon sind verschlüsselt:
//!   `[0..8]`   Header (PV + Sanity) — plaintext
//!   `[8..232]` 4 Blöcke à 56 Bytes (PKM-Daten, XOR + Shuffle)
//!   `[232..344]` Battle-Status — wird übersprungen
//!   `[344..366]` 22 Bytes Stats (Level + HP + 5 Stats) — nur XOR
//!
//! Diese Funktionen arbeiten auf dem **konkatenierten 254-Byte-Slice**
//! `slot[0..232] + slot[344..366]`. Reader/Writer in `party.rs` machen das
//! Splicing zur/von Citra-Memory.

const BLOCK_SIZE: usize = 56;
const HEADER_END: usize = 8;
const BLOCKS_END: usize = HEADER_END + 4 * BLOCK_SIZE; // 232
const SLOT_CONCAT_LEN: usize = 254;

/// LCG-Konstanten aus dem ORAS-Encryption-Stream.
const LCG_MUL: u32 = 0x41C6_4E6D;
const LCG_ADD: u32 = 0x0000_6073;

/// Block-Position-Lookup (4 logische Blöcke × 24 Shuffle-Werte).
/// Aus `citra-updater.py:129-132`. `BLOCK_POSITION[canonical_block][sv]` =
/// Position des canonical Blocks in der gescrambelten Reihenfolge.
const BLOCK_POSITION: [[u8; 24]; 4] = [
    [
        0, 0, 0, 0, 0, 0, 1, 1, 2, 3, 2, 3, 1, 1, 2, 3, 2, 3, 1, 1, 2, 3, 2, 3,
    ],
    [
        1, 1, 2, 3, 2, 3, 0, 0, 0, 0, 0, 0, 2, 3, 1, 1, 3, 2, 2, 3, 1, 1, 3, 2,
    ],
    [
        2, 3, 1, 1, 3, 2, 2, 3, 1, 1, 3, 2, 0, 0, 0, 0, 0, 0, 3, 2, 3, 2, 1, 1,
    ],
    [
        3, 2, 3, 2, 1, 1, 3, 2, 3, 2, 1, 1, 3, 2, 3, 2, 1, 1, 0, 0, 0, 0, 0, 0,
    ],
];

/// Berechnet den Shuffle-Value aus der Personality Value.
#[inline]
pub fn shuffle_value(pv: u32) -> usize {
    (((pv >> 13) & 0x1F) % 24) as usize
}

/// XOR den Byte-Range `[start..end]` mit einem frisch von `pv` gestarteten
/// LCG-Stream (matches Python `crypt_array`: temp_seed = seed jedes Aufrufs).
fn xor_lcg(data: &mut [u8], pv: u32, start: usize, end: usize) {
    let mut seed = pv;
    let mut i = start;
    while i < end {
        seed = seed.wrapping_mul(LCG_MUL).wrapping_add(LCG_ADD);
        data[i] ^= ((seed >> 16) & 0xFF) as u8;
        if i + 1 < end {
            data[i + 1] ^= ((seed >> 24) & 0xFF) as u8;
        }
        i += 2;
    }
}

/// Decrypts a 254-byte concatenated party-slot.
///
/// **Wichtig zur Stats-Sektion** (bytes 232..254 = slot[344..366]):
/// Wir XOR-en die Stats mit derselben LCG-Reset-Variante wie der Citra-Tracker
/// (Python `crypt_array` setzt `temp_seed = seed = pv` bei jedem Aufruf).
/// Bei unserem Build kommt für die Stats-Felder unbrauchbarer Inhalt raus —
/// vermutlich nutzt ORAS in RAM einen anderen Stats-Encryption-Layer. Wir
/// extrahieren Level deshalb live aus EXP + Growth-Rate (siehe
/// `PartyPokemon::read`), nicht aus `plain[0xEC]`. Stats-Decrypt bleibt
/// "best-effort" für Diagnose-Dumps; auf seine Korrektheit darf sich Production
/// nicht verlassen.
pub fn decrypt_slot(slot: &[u8; SLOT_CONCAT_LEN]) -> [u8; SLOT_CONCAT_LEN] {
    let pv = u32::from_le_bytes([slot[0], slot[1], slot[2], slot[3]]);
    let sv = shuffle_value(pv);

    let mut buf = *slot;
    xor_lcg(&mut buf, pv, HEADER_END, BLOCKS_END);
    xor_lcg(&mut buf, pv, BLOCKS_END, SLOT_CONCAT_LEN);

    // Block-Unshuffle: buf[8..232] in scrambled Reihenfolge → kanonisch.
    let mut out = buf;
    for (canonical_block, perm_row) in BLOCK_POSITION.iter().enumerate() {
        let scrambled_pos = perm_row[sv] as usize;
        let src_start = HEADER_END + scrambled_pos * BLOCK_SIZE;
        let dst_start = HEADER_END + canonical_block * BLOCK_SIZE;
        out[dst_start..dst_start + BLOCK_SIZE]
            .copy_from_slice(&buf[src_start..src_start + BLOCK_SIZE]);
    }
    out
}

/// Re-encrypts a plain 254-byte slot zurück in die on-wire Form.
pub fn encrypt_slot(plain: &[u8; SLOT_CONCAT_LEN]) -> [u8; SLOT_CONCAT_LEN] {
    let pv = u32::from_le_bytes([plain[0], plain[1], plain[2], plain[3]]);
    let sv = shuffle_value(pv);

    // Inverse-Shuffle: kanonisch → scrambled.
    let mut buf = *plain;
    for (canonical_block, perm_row) in BLOCK_POSITION.iter().enumerate() {
        let scrambled_pos = perm_row[sv] as usize;
        let src_start = HEADER_END + canonical_block * BLOCK_SIZE;
        let dst_start = HEADER_END + scrambled_pos * BLOCK_SIZE;
        buf[dst_start..dst_start + BLOCK_SIZE]
            .copy_from_slice(&plain[src_start..src_start + BLOCK_SIZE]);
    }

    // XOR-encrypt (symmetrisch zu decrypt; reset zwischen Blocks und Stats).
    xor_lcg(&mut buf, pv, HEADER_END, BLOCKS_END);
    xor_lcg(&mut buf, pv, BLOCKS_END, SLOT_CONCAT_LEN);

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_value_basic() {
        // pv >> 13 = 0 → sv = 0
        assert_eq!(shuffle_value(0x0000_0000), 0);
        // pv >> 13 = 1, % 24 = 1
        assert_eq!(shuffle_value(0x0000_2000), 1);
        // Aus dem Save: PV0 = 0xFC645FE6 → (>>13)&0x1F = (0x7E322)&0x1F = 2, %24 = 2
        assert_eq!(shuffle_value(0xFC64_5FE6), 2);
    }

    #[test]
    fn roundtrip_zero_slot() {
        let zero = [0u8; SLOT_CONCAT_LEN];
        let encrypted = encrypt_slot(&zero);
        let decrypted = decrypt_slot(&encrypted);
        assert_eq!(decrypted, zero);
    }

    #[test]
    fn roundtrip_pseudo_random_slots() {
        // Eigene deterministische LCG fuer reproduzierbare Pseudo-Random-Inputs
        // (kein rand-Crate, weil nicht in [dev-dependencies]).
        let mut seed: u64 = 0x5066_1EA7_D00D_FACE;
        for trial in 0..50 {
            let mut plain = [0u8; SLOT_CONCAT_LEN];
            for b in plain.iter_mut() {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                *b = (seed >> 33) as u8;
            }
            let cipher = encrypt_slot(&plain);
            let recovered = decrypt_slot(&cipher);
            assert_eq!(
                recovered,
                plain,
                "round-trip mismatch on trial {} (PV=0x{:08X})",
                trial,
                u32::from_le_bytes([plain[0], plain[1], plain[2], plain[3]])
            );
        }
    }

    #[test]
    fn header_unchanged() {
        // PV + sanity (slot[0..8]) bleibt durch decrypt unverändert
        let mut buf = [0u8; SLOT_CONCAT_LEN];
        buf[0..8].copy_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
        let dec = decrypt_slot(&buf);
        assert_eq!(&dec[0..8], &buf[0..8]);
    }

    #[test]
    fn decrypt_then_encrypt_is_identity() {
        let mut rng_buf = [0u8; SLOT_CONCAT_LEN];
        // Manuell ein paar PV-Variationen testen
        for &pv in &[0x1234_5678u32, 0xDEAD_BEEF, 0xFC64_5FE6, 0x406A_C1F7] {
            rng_buf[0..4].copy_from_slice(&pv.to_le_bytes());
            for (i, b) in rng_buf.iter_mut().enumerate().skip(8) {
                *b = (i as u8).wrapping_mul(31).wrapping_add(7);
            }
            let dec = decrypt_slot(&rng_buf);
            let re_enc = encrypt_slot(&dec);
            assert_eq!(
                re_enc, rng_buf,
                "encrypt(decrypt(x)) != x for PV=0x{:08X}",
                pv
            );
        }
    }
}
