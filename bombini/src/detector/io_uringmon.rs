//! IOUring detector

use aya::programs::BtfTracePoint;
use aya::{Btf, Ebpf, EbpfError};

use std::path::Path;

use super::{load_ebpf_obj, Detector};

pub struct IOUringMon {
    ebpf: Ebpf,
}

impl Detector for IOUringMon {
    async fn new<P, U>(obj_path: P, _yaml_config: Option<U>) -> Result<Self, anyhow::Error>
    where
        U: AsRef<str>,
        P: AsRef<Path>,
    {
        let ebpf = load_ebpf_obj(obj_path).await?;
        Ok(IOUringMon { ebpf })
    }

    fn map_initialize(&mut self) -> Result<(), EbpfError> {
        Ok(())
    }

    fn load_and_attach_programs(&mut self) -> Result<(), EbpfError> {
        let btf = Btf::from_sys_fs()?;
        let submit: &mut BtfTracePoint = self
            .ebpf
            .program_mut("io_uring_submit_req_capture")
            .unwrap()
            .try_into()?;
        submit.load("io_uring_submit_req", &btf)?;
        submit.attach()?;
        Ok(())
    }
}
