use anyhow::{bail, Result};

/// Plattform-unabhängige Sicht auf den virtuellen Adressraum eines Fremdprozesses.
pub trait ProcessMemory: Send {
    fn read_bytes(&self, addr: usize, size: usize) -> Result<Vec<u8>>;
    fn write_bytes(&self, addr: usize, data: &[u8]) -> Result<()>;

    fn read_u8(&self, addr: usize) -> Result<u8> {
        let b = self.read_bytes(addr, 1)?;
        if b.len() != 1 {
            bail!("partial read u8 @ 0x{:x} (got {} bytes)", addr, b.len());
        }
        Ok(b[0])
    }

    fn read_u16_le(&self, addr: usize) -> Result<u16> {
        let b = self.read_bytes(addr, 2)?;
        if b.len() != 2 {
            bail!("partial read u16 @ 0x{:x} (got {} bytes)", addr, b.len());
        }
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn read_u32_le(&self, addr: usize) -> Result<u32> {
        let b = self.read_bytes(addr, 4)?;
        if b.len() != 4 {
            bail!("partial read u32 @ 0x{:x} (got {} bytes)", addr, b.len());
        }
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn write_u32_le(&self, addr: usize, value: u32) -> Result<()> {
        self.write_bytes(addr, &value.to_le_bytes())
    }
}

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::LinuxProcessMemory as DefaultProcessMemory;
#[cfg(target_os = "windows")]
pub use windows::WindowsProcessMemory as DefaultProcessMemory;
