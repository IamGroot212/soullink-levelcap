use anyhow::{bail, Context, Result};
use std::io;

use super::ProcessMemory;

pub struct LinuxProcessMemory {
    pid: libc::pid_t,
}

impl LinuxProcessMemory {
    pub fn open(pid: u32) -> Result<Self> {
        // process_vm_readv/writev brauchen keinen separaten open() — wir merken uns nur die PID
        // und verifizieren, dass /proc/<pid> existiert. Permission-Probleme zeigen sich erst beim
        // ersten Read; siehe troubleshooting/ptrace_scope-Doku.
        let proc_path = format!("/proc/{}", pid);
        if !std::path::Path::new(&proc_path).exists() {
            bail!("Prozess {} existiert nicht in /proc", pid);
        }
        Ok(Self {
            pid: pid as libc::pid_t,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid as u32
    }

    /// Liefert die Memory-Map-Einträge des Zielprozesses (für FCRAM-Discovery).
    /// Jeder Eintrag: (start, end, perms, ist_anonym).
    pub fn maps(&self) -> Result<Vec<MapEntry>> {
        let path = format!("/proc/{}/maps", self.pid);
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("kann {} nicht lesen", path))?;
        Ok(content.lines().filter_map(MapEntry::parse).collect())
    }
}

#[derive(Debug, Clone)]
pub struct MapEntry {
    pub start: usize,
    pub end: usize,
    pub perms: String,
    pub pathname: String,
}

impl MapEntry {
    fn parse(line: &str) -> Option<Self> {
        // Format: "7f1234567000-7f1234600000 rw-p 00000000 00:00 0  [heap]"
        let mut parts = line.split_whitespace();
        let range = parts.next()?;
        let perms = parts.next()?.to_string();
        let (start_str, end_str) = range.split_once('-')?;
        let start = usize::from_str_radix(start_str, 16).ok()?;
        let end = usize::from_str_radix(end_str, 16).ok()?;
        // pathname ist optional und kann Whitespace enthalten → Rest des Strings
        // Skip: offset, dev, inode
        let _ = parts.next()?;
        let _ = parts.next()?;
        let _ = parts.next()?;
        let pathname = parts.collect::<Vec<_>>().join(" ");
        Some(Self {
            start,
            end,
            perms,
            pathname,
        })
    }

    pub fn size(&self) -> usize {
        self.end - self.start
    }

    pub fn is_anonymous_rw(&self) -> bool {
        self.pathname.is_empty() && self.perms.contains('r') && self.perms.contains('w')
    }
}

impl ProcessMemory for LinuxProcessMemory {
    fn read_bytes(&self, addr: usize, size: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; size];
        let local = libc::iovec {
            iov_base: buf.as_mut_ptr() as *mut libc::c_void,
            iov_len: size,
        };
        let remote = libc::iovec {
            iov_base: addr as *mut libc::c_void,
            iov_len: size,
        };

        let n = unsafe { libc::process_vm_readv(self.pid, &local, 1, &remote, 1, 0) };
        if n < 0 {
            let err = io::Error::last_os_error();
            bail!(
                "process_vm_readv(pid={}, addr=0x{:x}, size={}) failed: {} \
                 (ggf. ptrace_scope-Problem — siehe docs/TROUBLESHOOTING.md)",
                self.pid,
                addr,
                size,
                err
            );
        }
        buf.truncate(n as usize);
        Ok(buf)
    }

    fn write_bytes(&self, addr: usize, data: &[u8]) -> Result<()> {
        let local = libc::iovec {
            iov_base: data.as_ptr() as *mut libc::c_void,
            iov_len: data.len(),
        };
        let remote = libc::iovec {
            iov_base: addr as *mut libc::c_void,
            iov_len: data.len(),
        };

        let n = unsafe { libc::process_vm_writev(self.pid, &local, 1, &remote, 1, 0) };
        if n < 0 {
            let err = io::Error::last_os_error();
            bail!(
                "process_vm_writev(pid={}, addr=0x{:x}, size={}) failed: {}",
                self.pid,
                addr,
                data.len(),
                err
            );
        }
        if (n as usize) != data.len() {
            bail!(
                "process_vm_writev partial write: {} von {} bytes",
                n,
                data.len()
            );
        }
        Ok(())
    }
}
