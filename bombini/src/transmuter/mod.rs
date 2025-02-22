//! Transmuter provides an interface to transmute raw eBPF events into
//! serialized formats

use bombini_common::event::Event;

use gtfobins::GTFOBinsEvent;
use histfile::HistFileEvent;
use process::ProcessExec;
use process::ProcessExit;

mod gtfobins;
mod histfile;
mod process;

/// Transmutes eBPF events from low representation into serialized formats
pub struct Transmuter;

impl Transmuter {
    /// Transmutes bombini_common::Event into serialized formats
    pub async fn transmute(&self, event: Event) -> Result<Vec<u8>, anyhow::Error> {
        match event {
            Event::ProcExec(s) => Ok(ProcessExec::new(s).to_json()?.into_bytes()),
            Event::ProcExit(s) => Ok(ProcessExit::new(s).to_json()?.into_bytes()),
            Event::GTFOBins(s) => Ok(GTFOBinsEvent::new(s).to_json()?.into_bytes()),
            Event::HistFile(s) => Ok(HistFileEvent::new(s).to_json()?.into_bytes()),
        }
    }
}
