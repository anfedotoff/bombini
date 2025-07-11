//! Transmutes IOUringEvent to serialized format

use bombini_common::event::io_uring::IOUringMsg;

use serde::Serialize;

use super::process::Process;
use super::{transmute_ktime, Transmute};

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
#[repr(u8)]
enum IOUringOp {
    IORING_OP_NOP,
    IORING_OP_READV,
    IORING_OP_WRITEV,
    IORING_OP_FSYNC,
    IORING_OP_READ_FIXED,
    IORING_OP_WRITE_FIXED,
    IORING_OP_POLL_ADD,
    IORING_OP_POLL_REMOVE,
    IORING_OP_SYNC_FILE_RANGE,
    IORING_OP_SENDMSG,
    IORING_OP_RECVMSG,
    IORING_OP_TIMEOUT,
    IORING_OP_TIMEOUT_REMOVE,
    IORING_OP_ACCEPT,
    IORING_OP_ASYNC_CANCEL,
    IORING_OP_LINK_TIMEOUT,
    IORING_OP_CONNECT,
    IORING_OP_FALLOCATE,
    IORING_OP_OPENAT,
    IORING_OP_CLOSE,
    IORING_OP_FILES_UPDATE,
    IORING_OP_STATX,
    IORING_OP_READ,
    IORING_OP_WRITE,
    IORING_OP_FADVISE,
    IORING_OP_MADVISE,
    IORING_OP_SEND,
    IORING_OP_RECV,
    IORING_OP_OPENAT2,
    IORING_OP_EPOLL_CTL,
    IORING_OP_SPLICE,
    IORING_OP_PROVIDE_BUFFERS,
    IORING_OP_REMOVE_BUFFERS,
    IORING_OP_TEE,
    IORING_OP_SHUTDOWN,
    IORING_OP_RENAMEAT,
    IORING_OP_UNLINKAT,
    IORING_OP_MKDIRAT,
    IORING_OP_SYMLINKAT,
    IORING_OP_LINKAT,
    IORING_OP_MSG_RING,
    IORING_OP_FSETXATTR,
    IORING_OP_SETXATTR,
    IORING_OP_FGETXATTR,
    IORING_OP_GETXATTR,
    IORING_OP_SOCKET,
    IORING_OP_URING_CMD,
    IORING_OP_SEND_ZC,
    IORING_OP_SENDMSG_ZC,
    IORING_OP_READ_MULTISHOT,
    IORING_OP_WAITID,
    IORING_OP_FUTEX_WAIT,
    IORING_OP_FUTEX_WAKE,
    IORING_OP_FUTEX_WAITV,
    IORING_OP_FIXED_FD_INSTALL,
    IORING_OP_FTRUNCATE,
    IORING_OP_BIND,
    IORING_OP_LISTEN,
    IORING_OP_RECV_ZC,
    IORING_OP_EPOLL_WAIT,
    IORING_OP_READV_FIXED,
    IORING_OP_WRITEV_FIXED,

    /* this goes last, obviously */
    IORING_OP_LAST,
}

/// High-level event representation
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub struct IOUringEvent {
    /// Process Infro
    process: Process,
    /// io_uring_ops
    opcode: IOUringOp,
    flags: u64,
    /// Event's date and time
    timestamp: String,
}

impl IOUringEvent {
    /// Constructs High level event representation from low eBPF message
    pub fn new(event: IOUringMsg, ktime: u64) -> Self {
        Self {
            process: Process::new(event.process),
            opcode: unsafe { std::mem::transmute::<u8, IOUringOp>(event.opcode) },
            flags: event.flags,
            timestamp: transmute_ktime(ktime),
        }
    }
}

impl Transmute for IOUringEvent {}
