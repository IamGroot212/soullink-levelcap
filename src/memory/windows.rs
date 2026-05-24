use anyhow::{bail, Context, Result};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows_sys::Win32::System::Memory::{
    VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, MEM_PRIVATE, PAGE_READWRITE,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

use super::ProcessMemory;

pub struct WindowsProcessMemory {
    pid: u32,
    handle: HANDLE,
}

unsafe impl Send for WindowsProcessMemory {}

impl WindowsProcessMemory {
    pub fn open(pid: u32) -> Result<Self> {
        let access = PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_QUERY_INFORMATION;
        let handle = unsafe { OpenProcess(access, 0, pid) };
        if handle.is_null() {
            bail!(
                "OpenProcess({}) failed: {}",
                pid,
                std::io::Error::last_os_error()
            );
        }
        Ok(Self { pid, handle })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Iteriert die Memory-Regionen via `VirtualQueryEx` und liefert
    /// alle committed, RW, private Regionen (Kandidaten für FCRAM).
    pub fn regions(&self) -> Result<Vec<Region>> {
        let mut regions = Vec::new();
        let mut addr: usize = 0;
        loop {
            let mut info: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
            let ret = unsafe {
                VirtualQueryEx(
                    self.handle,
                    addr as *const _,
                    &mut info,
                    std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            if ret == 0 {
                break;
            }
            let base = info.BaseAddress as usize;
            let size = info.RegionSize;
            if info.State == MEM_COMMIT
                && info.Type == MEM_PRIVATE
                && info.Protect == PAGE_READWRITE
            {
                regions.push(Region { base, size });
            }
            addr = base.checked_add(size).context("VirtualQueryEx overflow")?;
        }
        Ok(regions)
    }
}

impl Drop for WindowsProcessMemory {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub base: usize,
    pub size: usize,
}

impl ProcessMemory for WindowsProcessMemory {
    fn read_bytes(&self, addr: usize, size: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; size];
        let mut read: usize = 0;
        let ok = unsafe {
            ReadProcessMemory(
                self.handle,
                addr as *const _,
                buf.as_mut_ptr() as *mut _,
                size,
                &mut read,
            )
        };
        if ok == 0 {
            bail!(
                "ReadProcessMemory(addr=0x{:x}, size={}) failed: {}",
                addr,
                size,
                std::io::Error::last_os_error()
            );
        }
        buf.truncate(read);
        Ok(buf)
    }

    fn write_bytes(&self, addr: usize, data: &[u8]) -> Result<()> {
        let mut written: usize = 0;
        let ok = unsafe {
            WriteProcessMemory(
                self.handle,
                addr as *mut _,
                data.as_ptr() as *const _,
                data.len(),
                &mut written,
            )
        };
        if ok == 0 {
            bail!(
                "WriteProcessMemory(addr=0x{:x}, size={}) failed: {}",
                addr,
                data.len(),
                std::io::Error::last_os_error()
            );
        }
        if written != data.len() {
            bail!(
                "WriteProcessMemory partial: {} von {} bytes",
                written,
                data.len()
            );
        }
        Ok(())
    }
}
