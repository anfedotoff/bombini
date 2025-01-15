//! GTFOBins detector

use aya::maps::lpm_trie::{Key, LpmTrie};
use aya::programs::TracePoint;
use aya::{Ebpf, EbpfError};

use procfs::sys::kernel::Version;
use yaml_rust2::YamlLoader;

use bombini_common::config::gtfobins::{GTFOBinsKey, MAX_ARGS_SIZE, MAX_FILENAME_SIZE};

use std::path::Path;

use super::{load_ebpf_obj, Detector};

pub struct GTFOBinsDetector {
    ebpf: Ebpf,
    config: Option<GTFOBinsConfig>,
}

struct GTFOBinsConfig {
    /// Entry values for GTFOBins map
    gtfobins_entries: Vec<(GTFOBinsKey, u32)>,
}

impl Detector for GTFOBinsDetector {
    fn min_kenrel_verison(&self) -> Version {
        Version::new(5, 8, 0)
    }

    async fn new<U: AsRef<Path>>(
        obj_path: U,
        config_path: Option<U>,
    ) -> Result<Self, anyhow::Error> {
        let ebpf = load_ebpf_obj(obj_path).await?;
        if let Some(config_path) = config_path {
            // Get config
            let mut config = GTFOBinsConfig {
                gtfobins_entries: Vec::new(),
            };

            let s = std::fs::read_to_string(config_path.as_ref())?;
            let docs = YamlLoader::load_from_str(&s)?;
            let doc = &docs[0];

            // TODO: safe parsing
            if let Some(entries) = doc["maps"]["gtfobins"].as_vec() {
                for entry in entries {
                    let v = entry["value"].as_i64().unwrap() as u32;
                    let mut k: GTFOBinsKey = [0; MAX_FILENAME_SIZE + 1 + MAX_ARGS_SIZE];
                    let k_str = entry["key"].as_str().unwrap();
                    let mut idx = 0;
                    //TODO: deal with ', ", ` when parsing args.
                    for s in k_str.split(' ') {
                        let len = s.len();
                        if idx + len < MAX_FILENAME_SIZE + 1 + MAX_ARGS_SIZE {
                            k[idx..idx + len].clone_from_slice(s.as_bytes());
                            if idx == 0 {
                                idx += MAX_FILENAME_SIZE + 1;
                            } else {
                                idx += len + 1;
                            }
                        }
                    }
                    config.gtfobins_entries.push((k, v));
                }
            }
            Ok(GTFOBinsDetector {
                ebpf,
                config: Some(config),
            })
        } else {
            Ok(GTFOBinsDetector { ebpf, config: None })
        }
    }

    fn map_initialize(&mut self) -> Result<(), EbpfError> {
        if let Some(config) = &self.config {
            let mut file_names: LpmTrie<_, GTFOBinsKey, u32> =
                LpmTrie::try_from(self.ebpf.map_mut("GTFOBINS").unwrap())?;
            for (k, v) in config.gtfobins_entries.iter() {
                let key = Key::new(((MAX_FILENAME_SIZE + 1 + MAX_ARGS_SIZE) * 8) as u32, *k);
                file_names.insert(&key, v, 0)?;
            }
        }
        Ok(())
    }

    fn load_and_attach_programs(&mut self) -> Result<(), EbpfError> {
        let program: &mut TracePoint = self
            .ebpf
            .program_mut("gtfobins_detect")
            .unwrap()
            .try_into()?;
        program.load()?;
        program.attach("sched", "sched_process_exec")?;
        Ok(())
    }
}
