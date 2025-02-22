//! Detector for Process executions and exits

use aya::maps::Array;
use aya::programs::{BtfTracePoint, FEntry, Lsm};
use aya::{Btf, Ebpf, EbpfError};

use procfs::sys::kernel::Version;
use yaml_rust2::YamlLoader;

use std::path::Path;

use bombini_common::config::procmon::Config;

use super::{load_ebpf_obj, Detector};

pub struct ProcMon {
    ebpf: Ebpf,
    config: Option<Config>,
}

impl Detector for ProcMon {
    fn min_kenrel_verison(&self) -> Version {
        Version::new(5, 10, 0)
    }

    async fn new<U: AsRef<Path>>(
        obj_path: U,
        config_path: Option<U>,
    ) -> Result<Self, anyhow::Error> {
        let ebpf = load_ebpf_obj(obj_path).await?;

        if let Some(config_path) = config_path {
            let mut config = Config {
                expose_events: false,
            };

            let s = std::fs::read_to_string(config_path.as_ref())?;
            let docs = YamlLoader::load_from_str(&s)?;
            let doc = &docs[0];

            config.expose_events = doc["expose-events"].as_bool().unwrap_or(false);

            Ok(ProcMon {
                ebpf,
                config: Some(config),
            })
        } else {
            Ok(ProcMon { ebpf, config: None })
        }
    }

    fn map_initialize(&mut self) -> Result<(), EbpfError> {
        if let Some(config) = self.config {
            let mut config_map: Array<_, Config> =
                Array::try_from(self.ebpf.map_mut("PROC_CONFIG").unwrap())?;
            let _ = config_map.set(0, config, 0);
        }
        Ok(())
    }

    fn load_and_attach_programs(&mut self) -> Result<(), EbpfError> {
        let btf = Btf::from_sys_fs()?;
        let exec: &mut BtfTracePoint = self
            .ebpf
            .program_mut("execve_capture")
            .unwrap()
            .try_into()?;
        exec.load("sched_process_exec", &btf)?;
        exec.attach()?;

        let fork: &mut FEntry = self.ebpf.program_mut("fork_capture").unwrap().try_into()?;
        fork.load("wake_up_new_task", &btf)?;
        fork.attach()?;

        let exit: &mut FEntry = self.ebpf.program_mut("exit_capture").unwrap().try_into()?;
        exit.load("acct_process", &btf)?;
        exit.attach()?;

        let program: &mut Lsm = self.ebpf.program_mut("creds_capture").unwrap().try_into()?;
        program.load("bprm_committing_creds", &btf)?;
        program.attach()?;
        Ok(())
    }
}
