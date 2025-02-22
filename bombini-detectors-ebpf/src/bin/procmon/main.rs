#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::BPF_ANY,
    helpers::{
        bpf_d_path, bpf_get_current_pid_tgid, bpf_get_current_task, bpf_ktime_get_ns,
        bpf_probe_read, bpf_probe_read_kernel_buf, bpf_probe_read_kernel_str_bytes,
        bpf_probe_read_user_buf, bpf_probe_read_user_str_bytes,
    },
    macros::{btf_tracepoint, fentry, lsm, map},
    maps::{array::Array, hash_map::HashMap, per_cpu_array::PerCpuArray},
    programs::{BtfTracePointContext, FEntryContext, LsmContext},
};

use bombini_detectors_ebpf::vmlinux::{
    cred, file, kernel_cap_t, kgid_t, kuid_t, linux_binprm, mm_struct, path, pid_t, qstr,
    task_struct,
};

use bombini_common::config::procmon::Config;
use bombini_common::event::process::{ProcInfo, SecureExec, MAX_ARGS_SIZE, MAX_FILE_PATH};
use bombini_common::event::{Event, MSG_PROCEXEC, MSG_PROCEXIT};

use bombini_detectors_ebpf::{event_capture, event_map::rb_event_init, util};

/// Extra info from bprm_committing_creds hook
struct CredSharedInfo {
    pub secureexec: SecureExec,
    /// full binary path
    pub binary_path: [u8; MAX_FILE_PATH],
}

#[map]
static PROC_MAP: HashMap<u32, ProcInfo> = HashMap::pinned(1024, 0);

#[map]
static CRED_SHARED_MAP: HashMap<u64, CredSharedInfo> = HashMap::pinned(1024, 0);

#[map]
static CRED_HEAP: PerCpuArray<CredSharedInfo> = PerCpuArray::with_max_entries(1, 0);

#[map]
static PROC_HEAP: PerCpuArray<ProcInfo> = PerCpuArray::with_max_entries(1, 0);

#[map]
static PROC_CONFIG: Array<Config> = Array::with_max_entries(1, 0);

#[inline(always)]
unsafe fn get_creds(proc: &mut ProcInfo, task: *const task_struct) -> Result<u32, u32> {
    let cred =
        bpf_probe_read::<*const cred>(&(*task).cred as *const *const _).map_err(|e| e as u32)?;
    let euid = bpf_probe_read::<kuid_t>(&(*cred).euid as *const _).map_err(|e| e as u32)?;
    let uid = bpf_probe_read::<kuid_t>(&(*cred).uid as *const _).map_err(|e| e as u32)?;
    let cap_e =
        bpf_probe_read::<kernel_cap_t>(&(*cred).cap_effective as *const _).map_err(|e| e as u32)?;
    let cap_i = bpf_probe_read::<kernel_cap_t>(&(*cred).cap_inheritable as *const _)
        .map_err(|e| e as u32)?;
    let cap_p =
        bpf_probe_read::<kernel_cap_t>(&(*cred).cap_permitted as *const _).map_err(|e| e as u32)?;

    proc.creds.uid = uid.val;
    proc.creds.euid = euid.val;
    proc.creds.cap_effective = cap_e.val;
    proc.creds.cap_inheritable = cap_i.val;
    proc.creds.cap_permitted = cap_p.val;

    Ok(0)
}

#[btf_tracepoint(function = "sched_process_exec")]
pub fn execve_capture(ctx: BtfTracePointContext) -> u32 {
    let Some(config_ptr) = PROC_CONFIG.get_ptr(0) else {
        return 0;
    };
    let config = unsafe { config_ptr.as_ref() };
    let Some(config) = config else {
        return 0;
    };
    event_capture!(ctx, MSG_PROCEXEC, try_execve, config.expose_events)
}

fn try_execve(_ctx: BtfTracePointContext, event: &mut Event, expose: bool) -> Result<u32, u32> {
    let Event::ProcExec(event) = event else {
        return Err(0);
    };
    let task = unsafe { bpf_get_current_task() as *const task_struct };
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let proc_ptr = unsafe { exec_map_get_init(pid) };
    let Some(proc_ptr) = proc_ptr else {
        return Err(0);
    };
    let proc = unsafe { proc_ptr.as_mut() };

    let Some(proc) = proc else {
        return Err(0);
    };
    let parent_proc = unsafe { execve_find_parent(task) };
    if let Some(parent_proc) = parent_proc {
        proc.ppid = unsafe { (*parent_proc).pid };
    }

    proc.ktime = unsafe { bpf_ktime_get_ns() };

    // We need to read real executable name and get arguments from stack.
    let (arg_start, arg_end) = unsafe {
        proc.pid = pid;
        proc.tid = pid_tgid as u32;

        let mm =
            bpf_probe_read::<*mut mm_struct>(&(*task).mm as *const *mut _).map_err(|e| e as u32)?;
        let mut arg_start = bpf_probe_read::<u64>(&(*mm).__bindgen_anon_1.arg_start as *const _)
            .map_err(|e| e as u32)?;

        let arg_end = bpf_probe_read::<u64>(&(*mm).__bindgen_anon_1.arg_end as *const _)
            .map_err(|e| e as u32)?;

        // Skip argv[0]
        let first_arg = bpf_probe_read_user_str_bytes(arg_start as *const u8, &mut proc.args)
            .map_err(|e| e as u32)?;

        arg_start += 1 + first_arg.len() as u64;

        let file = bpf_probe_read::<*mut file>(&(*mm).__bindgen_anon_1.exe_file as *const *mut _)
            .map_err(|e| e as u32)?;
        let path = bpf_probe_read::<path>(&(*file).f_path as *const _).map_err(|e| e as u32)?;

        let d_name =
            bpf_probe_read::<qstr>(&(*(path.dentry)).d_name as *const _).map_err(|e| e as u32)?;

        bpf_probe_read_kernel_str_bytes(d_name.name, &mut proc.filename).map_err(|e| e as u32)?;

        // Get cred
        get_creds(proc, task)?;

        let loginuid =
            bpf_probe_read::<kuid_t>(&(*task).loginuid as *const _).map_err(|e| e as u32)?;

        proc.auid = loginuid.val;

        (arg_start, arg_end)
    };
    let arg_size = (arg_end - arg_start) & (MAX_ARGS_SIZE - 1) as u64;
    unsafe {
        bpf_probe_read_user_buf(arg_start as *const u8, &mut proc.args[..arg_size as usize])
            .map_err(|e| e as u32)?;
    }

    if let Some(cred_info) = CRED_SHARED_MAP.get_ptr(&pid_tgid) {
        unsafe {
            let Some(cred_info) = cred_info.as_ref() else {
                return Err(0);
            };
            proc.creds.secureexec = cred_info.secureexec.clone();
            bpf_probe_read_kernel_str_bytes(cred_info.binary_path.as_ptr(), &mut proc.binary_path)
                .map_err(|e| e as u32)?;
        }
        CRED_SHARED_MAP.remove(&pid_tgid).unwrap();
    }

    // Copy process info to Rb
    if expose {
        util::copy_proc(proc, event);
    }

    Ok(0)
}

#[fentry(function = "acct_process")]
pub fn exit_capture(ctx: FEntryContext) -> u32 {
    let Some(config_ptr) = PROC_CONFIG.get_ptr(0) else {
        return 0;
    };
    let config = unsafe { config_ptr.as_ref() };
    let Some(config) = config else {
        return 0;
    };
    event_capture!(ctx, MSG_PROCEXIT, try_exit, config.expose_events)
}

fn try_exit(_ctx: FEntryContext, event: &mut Event, expose: bool) -> Result<u32, u32> {
    let Event::ProcExit(event) = event else {
        return Err(0);
    };
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    if expose {
        let proc = unsafe { PROC_MAP.get(&pid) };
        let Some(proc) = proc else {
            return Err(0);
        };
        util::copy_proc(proc, event);
    }
    PROC_MAP.remove(&pid).unwrap();
    Ok(0)
}

#[lsm(hook = "bprm_committing_creds")]
pub fn creds_capture(ctx: LsmContext) -> i32 {
    match try_committing_creds(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_committing_creds(ctx: LsmContext) -> Result<i32, i32> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let Some(cred_ptr) = CRED_HEAP.get_ptr_mut(0) else {
        return Err(0);
    };

    let creds_info = unsafe { cred_ptr.as_mut() };

    let Some(creds_info) = creds_info else {
        return Err(0);
    };
    unsafe {
        aya_ebpf::memset(cred_ptr as *mut u8, 0, core::mem::size_of_val(creds_info));
        let binprm: *const linux_binprm = ctx.arg(0);

        // Get cred
        let cred = (*binprm).cred;
        let euid = bpf_probe_read::<kuid_t>(&(*cred).euid as *const _)
            .map_err(|e| e as i32)?
            .val;
        let uid = bpf_probe_read::<kuid_t>(&(*cred).uid as *const _)
            .map_err(|e| e as i32)?
            .val;
        let egid = bpf_probe_read::<kgid_t>(&(*cred).egid as *const _)
            .map_err(|e| e as i32)?
            .val;
        let gid = bpf_probe_read::<kgid_t>(&(*cred).gid as *const _)
            .map_err(|e| e as i32)?
            .val;
        if euid != uid {
            creds_info.secureexec |= SecureExec::SETUID;
        }
        if egid != gid {
            creds_info.secureexec |= SecureExec::SETGID;
        }
        let file: *mut file = (*binprm).file;
        let _ = bpf_d_path(
            &(*file).f_path as *const _ as *mut aya_ebpf::bindings::path,
            creds_info.binary_path.as_mut_ptr() as *mut i8,
            creds_info.binary_path.len() as u32,
        );
        let _ = CRED_SHARED_MAP.insert(&pid_tgid, creds_info, BPF_ANY as u64);
    }
    Ok(0)
}

#[fentry(function = "wake_up_new_task")]
pub fn fork_capture(ctx: FEntryContext) -> u32 {
    match try_wake_up_new_task(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_wake_up_new_task(ctx: FEntryContext) -> Result<u32, u32> {
    let task: *const task_struct = unsafe { ctx.arg(0) };
    let tgid = unsafe { bpf_probe_read(&(*task).tgid as *const pid_t).map_err(|_| 0u32)? as u32 };

    let parent_proc = unsafe { execve_find_parent(task) };
    let Some(parent_proc) = parent_proc else {
        return Err(0);
    };
    let proc_ptr = unsafe { exec_map_get_init(tgid) };
    let Some(proc_ptr) = proc_ptr else {
        return Err(0);
    };
    let proc = unsafe { proc_ptr.as_mut() };

    let Some(proc) = proc else {
        return Err(0);
    };
    if proc.ktime != 0 {
        return Err(0);
    }
    proc.pid = tgid;
    unsafe {
        proc.ppid = (*parent_proc).pid;
        proc.ktime = bpf_ktime_get_ns();
        proc.tid = bpf_probe_read::<pid_t>(&(*task).pid as *const _).map_err(|e| e as u32)? as u32;
        bpf_probe_read_kernel_str_bytes(&(*parent_proc).filename as *const _, &mut proc.filename)
            .map_err(|e| e as u32)?;
        bpf_probe_read_kernel_str_bytes(
            &(*parent_proc).binary_path as *const _,
            &mut proc.binary_path,
        )
        .map_err(|e| e as u32)?;
        bpf_probe_read_kernel_buf(&(*parent_proc).args as *const _, &mut proc.args)
            .map_err(|e| e as u32)?;
        get_creds(proc, task)?;
    }
    Ok(0)
}
#[inline(always)]
unsafe fn execve_find_parent(task: *const task_struct) -> Option<*const ProcInfo> {
    let mut parent = task;
    for _i in 0..4 {
        let Ok(task) = bpf_probe_read::<*mut task_struct>(&(*parent).real_parent as *const _)
        else {
            return None;
        };
        parent = task;
        let Ok(pid) = bpf_probe_read(&(*parent).tgid as *const pid_t) else {
            return None;
        };
        if let Some(proc_ptr) = PROC_MAP.get_ptr(&(pid as u32)) {
            return Some(proc_ptr);
        }
    }
    None
}

#[inline(always)]
unsafe fn exec_map_get_init(pid: u32) -> Option<*mut ProcInfo> {
    if let Some(proc_ptr) = PROC_MAP.get_ptr_mut(&pid) {
        return Some(proc_ptr);
    }

    let proc_ptr = PROC_HEAP.get_ptr_mut(0)?;

    let proc = unsafe { proc_ptr.as_mut() };

    let proc = proc?;
    aya_ebpf::memset(proc_ptr as *mut u8, 0, core::mem::size_of_val(proc));
    if PROC_MAP.insert(&pid, proc, BPF_ANY as u64).is_err() {
        return None;
    }
    PROC_MAP.get_ptr_mut(&pid)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
