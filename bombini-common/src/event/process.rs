//! Process event module

use bitflags::bitflags;

#[cfg(feature = "user")]
use serde::Serialize;

pub const MAX_FILENAME_SIZE: usize = 32;

pub const MAX_ARGS_SIZE: usize = 1024;

pub const MAX_FILE_PATH: usize = 512;

/// Process event
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ProcInfo {
    /// PID
    pub pid: u32,
    /// TID
    pub tid: u32,
    /// Parent PID
    pub ppid: u32,
    /// Task Creds
    pub creds: Cred,
    /// login UID
    pub auid: u32,
    /// if this event from clone
    pub clonned: bool,
    /// executable name
    pub filename: [u8; MAX_FILENAME_SIZE],
    /// full binary path
    pub binary_path: [u8; MAX_FILE_PATH],
    /// command line arguments without argv[0]
    pub args: [u8; MAX_ARGS_SIZE],
}

/// Creds
#[derive(Clone, Debug)]
#[repr(C)]
pub struct Cred {
    /// UID
    pub uid: u32,
    /// EUID
    pub euid: u32,
    pub cap_inheritable: u64,
    pub cap_permitted: u64,
    pub cap_effective: u64,
    pub secureexec: SecureExec,
}

bitflags! {
    #[derive(Clone, Debug, PartialEq)]
    #[cfg_attr(feature = "user", derive(Serialize))]
    #[repr(C)]
    pub struct SecureExec: u32 {
        const SETUID = 0b00000001;
        const SETGID = 0b00000010;
        const FILE_CAPS = 0b00000100;
    }
}
