//! The OS layer. Everything here that calls the OS directly is the only
//! `unsafe` in tTorrent; each call carries a SAFETY comment.

use std::path::{Path, PathBuf};

pub mod assoc;
pub mod tray;

/// Memory one process uses, in bytes, read three ways (docs/perf/README.md).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessMemory {
    /// Task Manager's "Memory" column: the private working set.
    pub private_working_set: u64,
    pub working_set: u64,
    /// Commit charge.
    pub private_bytes: u64,
}

/// The data folder: `%APPDATA%\tTorrent` on Windows, `~/Library/Application
/// Support/tTorrent` on macOS, `$XDG_DATA_HOME/ttorrent` elsewhere.
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("tTorrent"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support/tTorrent"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .map(|d| d.join("ttorrent"))
    }
}

/// The user's Downloads folder, the default save path.
pub fn downloads_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Downloads")
}

/// Open a folder in the file manager.
pub fn open_folder(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let cmd = "explorer";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = "xdg-open";
    std::process::Command::new(cmd)
        .arg(path)
        .spawn()
        .map(|_| ())
}

#[cfg(windows)]
mod win {
    use super::ProcessMemory;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::core::PCWSTR;

    pub fn process_memory(pid: Option<u32>) -> Option<ProcessMemory> {
        // SAFETY: OpenProcess/GetCurrentProcess return a handle or an error;
        // the counters struct is sized and zeroed as the API requires, and the
        // handle we opened is closed before returning.
        unsafe {
            let (handle, owned) = match pid {
                None => (GetCurrentProcess(), false),
                Some(p) => (
                    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, p).ok()?,
                    true,
                ),
            };
            let mut c = PROCESS_MEMORY_COUNTERS_EX2 {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32,
                ..Default::default()
            };
            let ok = GetProcessMemoryInfo(
                handle,
                (&mut c as *mut PROCESS_MEMORY_COUNTERS_EX2).cast::<PROCESS_MEMORY_COUNTERS>(),
                c.cb,
            );
            if owned {
                let _ = CloseHandle(handle);
            }
            ok.ok()?;
            Some(ProcessMemory {
                private_working_set: c.PrivateWorkingSetSize as u64,
                working_set: c.WorkingSetSize as u64,
                private_bytes: c.PrivateUsage as u64,
            })
        }
    }

    pub fn process_io(pid: u32) -> Option<super::ProcessIo> {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::{
            GetProcessIoCounters, GetProcessTimes, IO_COUNTERS,
        };
        let ft = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
        // SAFETY: the handle comes from OpenProcess and is closed before
        // returning; every out pointer is a valid, initialised local.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut io = IO_COUNTERS::default();
            let (mut c, mut e, mut k, mut u) = (
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
            );
            let a = GetProcessIoCounters(handle, &mut io);
            let b = GetProcessTimes(handle, &mut c, &mut e, &mut k, &mut u);
            let _ = CloseHandle(handle);
            a.ok()?;
            b.ok()?;
            Some(super::ProcessIo {
                read_bytes: io.ReadTransferCount,
                write_bytes: io.WriteTransferCount,
                read_ops: io.ReadOperationCount,
                // FILETIME counts 100 ns ticks.
                cpu_ms: (ft(k) + ft(u)) / 10_000,
            })
        }
    }

    pub fn free_space(path: &Path) -> Option<u64> {
        // The nearest existing ancestor: the save folder may not exist yet.
        let mut p = path;
        while !p.exists() {
            p = p.parent()?;
        }
        let wide: Vec<u16> = p.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut free = 0u64;
        // SAFETY: `wide` is NUL-terminated and outlives the call; the out
        // pointer is a valid u64.
        unsafe {
            GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free), None, None).ok()?;
        }
        Some(free)
    }
}

/// File I/O and CPU time a process has used since it started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessIo {
    /// Bytes the process asked to read. Includes reads the OS served from its
    /// file cache, so it is an upper bound on what came from the disk.
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    /// User plus kernel time, in milliseconds.
    pub cpu_ms: u64,
}

/// I/O and CPU counters of a process. `None` if they can't be read.
pub fn process_io(pid: u32) -> Option<ProcessIo> {
    #[cfg(windows)]
    {
        win::process_io(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        None
    }
}

/// Memory of a process (`None`: this one). `None` if it can't be read.
pub fn process_memory(pid: Option<u32>) -> Option<ProcessMemory> {
    #[cfg(windows)]
    {
        win::process_memory(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        None
    }
}

/// Free bytes on the volume holding `path`.
pub fn free_space(path: &Path) -> Option<u64> {
    #[cfg(windows)]
    {
        win::free_space(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn reads_own_memory() {
        let m = process_memory(None).unwrap();
        assert!(m.working_set > 0 && m.private_bytes > 0);
        assert!(m.private_working_set <= m.working_set);
    }

    #[cfg(windows)]
    #[test]
    fn free_space_of_temp() {
        assert!(free_space(&std::env::temp_dir().join("does/not/exist")).unwrap_or(0) > 0);
    }
}
