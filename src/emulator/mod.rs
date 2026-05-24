pub mod citra;

pub use citra::CitraProcess;

/// Trait-Anker, falls später weitere Emulatoren (z. B. ein Citra-Fork) hinzukommen.
pub trait Emulator {
    fn pid(&self) -> u32;
    fn fcram_addr(&self, addr_3ds: usize) -> usize;
}
