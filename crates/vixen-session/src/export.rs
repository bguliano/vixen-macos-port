//! Exports a sequence: `SequenceIntervalGenerator` over this port's schedule and output graph.

use crate::app::AppCore;
use crate::playback::{ShowPlayback, Transport};
use crate::sequence::SequenceSession;
use crate::show::{build_output_graph, StoreLookup};
use crate::SessionError;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::{CombinedState, TimeSpan};
use vixen_engine::schedule::{EvalScratch, Schedule};
use vixen_export::fpp_config::{active_in_order, ControllerInfo, NetworkConfig, ProtocolType, TransmissionMethod, UniverseConfig};
use vixen_export::run::{run_export, ExportJob, ExportOutcome, IntervalSource};
use vixen_export::{file_name_of, ExportFormat};
use vixen_io::sequence::TimedSequence;
use vixen_io::{DataPaths, SystemData};
use vixen_output::modules::{self as output_modules, ControllerConfig, DdpModule, E131Module};
use vixen_output::{ModuleDataLookup, OutputAddress, OutputGraph, OutputModule};

/// The Audio media module, `AudioDescriptor`. The only media type there is.
pub const AUDIO_MEDIA_TYPE: Uuid = uuid::uuid!("fe460392-3fab-4c63-99dd-d76c48354150");

/// What the export dialog and wizard choose.
#[derive(Clone, Debug)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub compress: bool,
    pub out_file_name: PathBuf,
    pub update_interval_ms: i32,
    /// `Export.AudioFilename`: the full path, or empty for no media header.
    pub audio_filename: String,
    pub match_audio_name: bool,
    /// The controllers and their order. Empty means all of them, in system order, as a new export starts.
    pub controllers: Vec<ControllerInfo>,
}

/// `Export.CreateControllerInfo`: every controller in system order, active. Network details are the export dialog's to add.
pub fn default_controller_info(system: &SystemData) -> Vec<ControllerInfo> {
    system
        .controllers
        .iter()
        .enumerate()
        .map(|(index, c)| ControllerInfo {
            id: c.id,
            name: c.name.clone(),
            index: index as i32,
            is_active: true,
            channels: c.outputs.len() as i32,
            network: None,
        })
        .collect()
}

/// `Export.CreateControllerInfo(true, ...)`: [`default_controller_info`], plus the network configuration of each controller whose module
/// `SupportsNetwork` and whose configuration `SupportsUniverses`: E1.31 and DDP. An E1.31 controller unicasting to a host name looks the
/// name up, as `GetNetworkConfiguration` does upstream, so call this off the main thread.
pub fn controller_info_with_network(system: &SystemData) -> Vec<ControllerInfo> {
    let store = vixen_io::system::effective_store(system);
    let lookup = StoreLookup::new(&store);
    let mut list = default_controller_info(system);
    for (info, controller) in list.iter_mut().zip(&system.controllers) {
        info.network = network_config(controller, &lookup);
    }
    list
}

fn network_config(controller: &vixen_io::system::Controller, lookup: &StoreLookup<'_>) -> Option<NetworkConfig> {
    let config = match lookup.module_data(&controller.hardware_type_id, &controller.hardware_instance_id) {
        Some(data) => ControllerConfig::from_xml(&controller.hardware_type_id, data),
        None => ControllerConfig::new_default(&controller.hardware_type_id, controller.hardware_instance_id),
    }?;
    let network = match config {
        ControllerConfig::E131(config) => E131Module::new(config).network_configuration(),
        ControllerConfig::Ddp(config) => {
            let mut module = DdpModule::new(config);
            module.set_output_count(controller.outputs.len());
            module.network_configuration()
        }
        _ => return None,
    };
    if !network.supports_universes {
        return None;
    }
    Some(NetworkConfig {
        protocol: match network.protocol {
            output_modules::NetworkProtocol::E131 => ProtocolType::Sacn,
            output_modules::NetworkProtocol::Ddp => ProtocolType::Ddp,
        },
        transmission: match network.transmission {
            output_modules::TransmissionMethod::Multicast => TransmissionMethod::Multicast,
            output_modules::TransmissionMethod::Unicast => TransmissionMethod::Unicast,
        },
        ip_address: network.ip_address.map(|ip| ip.to_string()).unwrap_or_default(),
        throttling_enabled: network.throttling_enabled,
        universes: network
            .universes
            .iter()
            .map(|u| UniverseConfig { active: u.active, universe_number: u.universe_number, size: u.size })
            .collect(),
    })
}

/// `ExportDialog`'s `_audioFileName`, and the wizard's `AudioFilename`: the first audio media with a file, as a full path, or empty. A
/// sequence saved on Windows names a Windows path, so the file is looked for in this profile's `Media` folder by name.
pub fn audio_file_name(document: &TimedSequence, paths: &DataPaths) -> String {
    document
        .media
        .iter()
        .filter(|m| m.type_id == AUDIO_MEDIA_TYPE && !m.file_name.is_empty())
        .find_map(|m| resolve_audio_path(&m.file_name, paths))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// F32: a media path as stored, if that file exists, otherwise `<Media>/<file name>`, splitting on both separators. `None` for an
/// empty name. A file that exists in neither place still gives the `Media` path, because only its name reaches the FSEQ header.
pub fn resolve_audio_path(media_path: &str, paths: &DataPaths) -> Option<PathBuf> {
    if media_path.is_empty() {
        return None;
    }
    let as_given = Path::new(media_path);
    if as_given.is_absolute() && as_given.is_file() {
        return Some(as_given.to_path_buf());
    }
    let name = file_name_of(media_path);
    (!name.is_empty()).then(|| paths.media_dir().join(name))
}

/// F12 steps 2 and 6: what's playing pauses for the length of an export, and carries on afterwards on every path out, a panic included.
/// GUARD: upstream skips the resume when Stop lands between `BeginGeneration` and the run, which leaves everything paused until a restart.
pub struct ExportPause {
    playback: Arc<ShowPlayback>,
    resume: bool,
}

impl ExportPause {
    pub fn begin(core: &AppCore) -> ExportPause {
        let playback = Arc::clone(&core.playback);
        let resume = playback.transport() == Transport::Playing;
        if resume {
            playback.pause();
        }
        ExportPause { playback, resume }
    }
}

impl Drop for ExportPause {
    fn drop(&mut self) {
        // Only what this pause paused, and only if nobody stopped or restarted it meanwhile.
        if self.resume && self.playback.transport() == Transport::Paused {
            self.playback.resume();
        }
    }
}

/// An export ready to run, holding what it needs so that the session isn't locked while it runs.
pub struct PreparedExport {
    job: ExportJob,
    format: ExportFormat,
    compress: bool,
    source: ScheduleSource,
}

impl PreparedExport {
    /// `Export.DoExport`, up to the point of writing: waits for every effect to render, then builds the row layout.
    pub fn prepare(session: &mut SequenceSession, system: &SystemData, request: &ExportRequest) -> Result<PreparedExport, SessionError> {
        if request.update_interval_ms <= 0 {
            return Err(SessionError::Other("The update interval must be above zero.".to_string()));
        }
        session.wait_for_renders();
        let schedule = session.schedule();
        let graph = build_output_graph(system, &schedule.elements().ids);

        let defaults;
        let infos: &[ControllerInfo] = if request.controllers.is_empty() {
            defaults = default_controller_info(system);
            &defaults
        } else {
            &request.controllers
        };
        let mut controllers = Vec::new();
        let mut channel_names = Vec::new();
        let tree = session.tree();
        let element_name = |element: &uuid::Uuid| {
            tree.find_by_element(element).map(|n| tree.node(n).name.clone()).unwrap_or_default()
        };
        for info in active_in_order(infos) {
            let Some(index) = graph.controller_index(&info.id) else { continue };
            controllers.push(index);
            for output in 0..graph.controllers()[index].output_count() {
                let address = OutputAddress { controller: index, output };
                channel_names.push(graph.channel_name(address, &element_name).unwrap_or_default());
            }
        }

        let job = ExportJob {
            sequence_name: session.name(),
            sequence_length: session.length(),
            out_file_name: request.out_file_name.clone(),
            update_interval_ms: request.update_interval_ms,
            channel_names,
            audio_filename: request.audio_filename.clone(),
            match_audio_name: request.match_audio_name,
        };
        Ok(PreparedExport {
            job,
            format: request.format,
            compress: request.compress,
            source: ScheduleSource::new(schedule, graph, controllers),
        })
    }

    /// `ProcessExport`. `progress` gets a percentage and a message before each interval.
    pub fn run(mut self, cancel: &AtomicBool, progress: &mut dyn FnMut(i32, &str)) -> Result<ExportOutcome, SessionError> {
        let mut writer = self.format.writer(self.compress);
        Ok(run_export(&self.job, writer.as_mut(), &mut self.source, cancel, progress)?)
    }
}

/// Samples a rendered sequence through the output graph, one export row at a time.
pub struct ScheduleSource {
    schedule: Arc<Schedule>,
    graph: OutputGraph,
    /// The controllers in the row, in export order.
    controllers: Vec<usize>,
    scratch: EvalScratch,
    states: Vec<CombinedState>,
}

impl ScheduleSource {
    /// The graph must have been built for this schedule's element order.
    pub fn new(schedule: Arc<Schedule>, graph: OutputGraph, controllers: Vec<usize>) -> ScheduleSource {
        ScheduleSource { schedule, graph, controllers, scratch: EvalScratch::default(), states: Vec::new() }
    }

    /// All controllers in system order, as a new export starts.
    pub fn all_controllers(schedule: Arc<Schedule>, graph: OutputGraph) -> ScheduleSource {
        let controllers = (0..graph.controllers().len()).collect();
        ScheduleSource::new(schedule, graph, controllers)
    }

    pub fn graph(&self) -> &OutputGraph {
        &self.graph
    }

    /// The states the last `sample` left, in schedule element order.
    pub fn states(&self) -> &[CombinedState] {
        &self.states
    }
}

impl IntervalSource for ScheduleSource {
    fn sample(&mut self, time: TimeSpan, row: &mut [u8]) {
        self.schedule.evaluate(time, &mut self.scratch, &mut self.states);
        self.graph.evaluate(&self.states);
        self.graph.fill_row(&self.controllers, row);
    }
}
