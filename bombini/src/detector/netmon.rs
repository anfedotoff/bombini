//! Network monitor detector

use aya::maps::{
    hash_map::HashMap,
    lpm_trie::{Key, LpmTrie},
    Array,
};
use aya::programs::FExit;
use aya::{Btf, Ebpf, EbpfError, EbpfLoader};

use std::path::Path;

use bombini_common::{
    config::network::Config,
    config::procmon::ProcessFilterMask,
    constants::{MAX_FILENAME_SIZE, MAX_FILE_PATH, MAX_FILE_PREFIX},
};

use crate::{
    config::{CONFIG, EVENT_MAP_NAME, PROCMON_PROC_MAP_NAME},
    init_process_filter_maps,
    proto::config::NetMonConfig,
    resize_process_filter_maps,
};

use super::Detector;

pub struct NetMon {
    ebpf: Ebpf,
    config: NetMonConfig,
}
impl Detector for NetMon {
    async fn new<P, U>(obj_path: P, yaml_config: Option<U>) -> Result<Self, anyhow::Error>
    where
        U: AsRef<str>,
        P: AsRef<Path>,
    {
        let Some(yaml_config) = yaml_config else {
            anyhow::bail!("Config for io_uringmon must be provided");
        };

        let config: NetMonConfig = serde_yml::from_str(yaml_config.as_ref())?;
        let config_opts = CONFIG.read().await;
        let mut ebpf_loader = EbpfLoader::new();
        let mut ebpf_loader_ref = ebpf_loader
            .map_pin_path(config_opts.maps_pin_path.as_ref().unwrap())
            .set_max_entries(EVENT_MAP_NAME, config_opts.event_map_size.unwrap())
            .set_max_entries(
                PROCMON_PROC_MAP_NAME,
                config_opts.procmon_proc_map_size.unwrap(),
            );
        if let Some(filter) = &config.process_filter {
            resize_process_filter_maps!(filter, ebpf_loader_ref);
        }
        let ebpf = ebpf_loader_ref.load_file(obj_path.as_ref())?;
        Ok(NetMon { ebpf, config })
    }

    fn map_initialize(&mut self) -> Result<(), EbpfError> {
        let mut config = Config {
            filter_mask: ProcessFilterMask::empty(),
            deny_list: false,
        };
        if let Some(filter) = &self.config.process_filter {
            config.filter_mask = init_process_filter_maps!(filter, &mut self.ebpf);
            config.deny_list = filter.deny_list;
        }
        let mut config_map: Array<_, Config> =
            Array::try_from(self.ebpf.map_mut("NETMON_CONFIG").unwrap())?;
        let _ = config_map.set(0, config, 0);
        Ok(())
    }

    fn load_and_attach_programs(&mut self) -> Result<(), EbpfError> {
        let btf = Btf::from_sys_fs()?;
        let tcp_v4_connect: &mut FExit = self
            .ebpf
            .program_mut("tcp_v4_connect_capture")
            .unwrap()
            .try_into()?;
        tcp_v4_connect.load("tcp_v4_connect", &btf)?;
        tcp_v4_connect.attach()?;
        let tcp_v6_connect: &mut FExit = self
            .ebpf
            .program_mut("tcp_v6_connect_capture")
            .unwrap()
            .try_into()?;
        tcp_v6_connect.load("tcp_v6_connect", &btf)?;
        tcp_v6_connect.attach()?;
        let tcp_close: &mut FExit = self
            .ebpf
            .program_mut("tcp_close_capture")
            .unwrap()
            .try_into()?;
        tcp_close.load("tcp_close", &btf)?;
        tcp_close.attach()?;
        let tcp_accept: &mut FExit = self
            .ebpf
            .program_mut("inet_csk_accept_capture")
            .unwrap()
            .try_into()?;
        tcp_accept.load("inet_csk_accept", &btf)?;
        tcp_accept.attach()?;
        Ok(())
    }
}

/// IOUringMon Filter map names
const FILTER_UID_MAP_NAME: &str = "NETMON_FILTER_UID_MAP";

const FILTER_EUID_MAP_NAME: &str = "NETMON_FILTER_EUID_MAP";

const FILTER_AUID_MAP_NAME: &str = "NETMON_FILTER_AUID_MAP";

const FILTER_BINPATH_MAP_NAME: &str = "NETMON_FILTER_BINPATH_MAP";

const FILTER_BINNAME_MAP_NAME: &str = "NETMON_FILTER_BINNAME_MAP";

const FILTER_BINPREFIX_MAP_NAME: &str = "NETMON_FILTER_BINPREFIX_MAP";
