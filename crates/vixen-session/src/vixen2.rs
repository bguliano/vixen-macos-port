//! Vixen 2 channel-major sequence import. The converter never mutates a saved map.
use crate::SessionError;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use uuid::{uuid, Uuid};
use vixen_core::{Color, ColorGradient, Curve, ElementTree, TimeSpan};
use vixen_effects::framework::{ParamValue, Params};
use vixen_io::{EffectNode, TimedSequence};
use vixen_xml::XElement;

pub mod maps;
pub const MODULE_TYPE: Uuid = uuid!("92bbd2cb-b750-437f-8a88-49864d569ab4");
pub const FLATTENED: &str = "Sequence has been flattened no profile is available";
fn error(e: impl std::fmt::Display) -> SessionError {
    SessionError::Other(e.to_string())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelMapping {
    pub name: String,
    pub number: String,
    pub output: String,
    pub color: Color,
    pub destination: Option<Uuid>,
    pub import_color: Color,
    pub mixing: bool,
}
#[derive(Clone, Debug)]
pub struct Vixen2File {
    pub path: PathBuf,
    pub audio_name: String,
    pub profile_name: String,
    pub audio: Option<PathBuf>,
    pub profile: Option<PathBuf>,
    pub length_ms: u32,
    pub period_ms: u32,
    pub bytes: Vec<u8>,
    pub channels: Vec<ChannelMapping>,
    pub events_per_channel: usize,
    pub warnings: Vec<String>,
}
impl Vixen2File {
    /// First pass before Swift asks for named audio/profile paths.
    pub fn read(path: &Path) -> Result<Self, SessionError> {
        let root = XElement::from_file(path).map_err(error)?;
        let integer = |key: &str| -> Result<u32, SessionError> {
            root.child_text(key)
                .ok_or_else(|| error(format!("Missing {key}.")))?
                .trim()
                .parse()
                .map_err(|_| error(format!("Invalid {key}.")))
        };
        let length_ms = integer("Time")?;
        let period_ms = integer("EventPeriodInMilliseconds")?;
        if period_ms == 0 || length_ms / period_ms == 0 {
            return Err(error(
                "The sequence must contain at least one event with a positive event period.",
            ));
        }
        let text = root
            .child_text("EventValues")
            .ok_or_else(|| error("Missing EventValues."))?;
        let encoded: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        let plain = encoded.trim_end_matches('=');
        if encoded.len() % 4 != 0
            || encoded.len() - plain.len() > 2
            || !plain
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
        {
            return Err(error("Invalid base64 EventValues."));
        }
        let bytes = vixen_xml::nbfx::base64_decode(&encoded)
            .ok_or_else(|| error("Invalid base64 EventValues."))?;
        if bytes.is_empty() {
            return Err(error("The sequence contains no channel data."));
        }
        let channels = read_channels(root.child("Channels"))?;
        // A named profile is ignored even when the Channels children are malformed.
        let flattened = root
            .child("Channels")
            .is_some_and(|c| c.elements().next().is_some());
        Ok(Self {
            path: path.into(),
            audio_name: root
                .child("Audio")
                .and_then(|e| e.attr("filename"))
                .unwrap_or_default()
                .to_owned(),
            profile_name: if flattened {
                String::new()
            } else {
                root.child_text("Profile").unwrap_or_default()
            },
            audio: None,
            profile: None,
            length_ms,
            period_ms,
            bytes,
            channels,
            events_per_channel: 0,
            warnings: Vec::new(),
        })
    }
    pub fn finish(
        mut self,
        audio: Option<&Path>,
        profile: Option<&Path>,
    ) -> Result<Self, SessionError> {
        self.audio = audio.map(Path::to_path_buf);
        if !self.profile_name.is_empty() {
            if let Some(path) = profile {
                let root = XElement::from_file(path).map_err(error)?;
                self.channels
                    .extend(read_channels(root.child("ChannelObjects"))?);
                self.profile = Some(path.into());
            }
        } else {
            self.profile_name = FLATTENED.into();
        }
        let events = (self.length_ms / self.period_ms) as usize;
        let calculated = self.bytes.len() / events;
        if calculated == 0 || self.channels.is_empty() {
            return Err(error(
                "The sequence contains no complete channel or the profile was not supplied.",
            ));
        }
        self.events_per_channel = self.bytes.len() / calculated;
        if self.channels.len() != calculated {
            self.warnings.push(format!("ParseFile: Actual mappings ({}) and calculated mappings ({calculated}) do not match. Using the Actual mappings value.", self.channels.len()));
        }
        if self
            .channels
            .len()
            .checked_mul(self.events_per_channel)
            .is_none_or(|n| n > self.bytes.len())
        {
            return Err(error(
                "The channel definitions require more event data than the file contains.",
            ));
        }
        Ok(self)
    }
}
fn read_channels(parent: Option<&XElement>) -> Result<Vec<ChannelMapping>, SessionError> {
    let mut channels = Vec::new();
    for c in parent
        .into_iter()
        .flat_map(XElement::elements)
        .filter(|c| c.local == "Channel")
    {
        let name = if let Some(name) = c.attr("name") {
            name.to_owned()
        } else if c.attr("color").is_some() {
            c.text()
        } else {
            continue;
        };
        let output = c
            .attr("output")
            .ok_or_else(|| error("Channel is missing its output."))?
            .to_owned();
        let color: i32 = c
            .attr("color")
            .ok_or_else(|| error("Channel is missing its color."))?
            .parse()
            .map_err(|_| error("Channel color is not a signed ARGB integer."))?;
        channels.push(ChannelMapping {
            name: if name.trim().is_empty() {
                format!("Unnamed Output {output}")
            } else {
                name
            },
            number: (channels.len() + 1).to_string(),
            output,
            color: Color::from_argb_u32(color as u32),
            destination: None,
            import_color: Color::EMPTY,
            mixing: false,
        });
    }
    Ok(channels)
}

/// Assign tree leaves in display order, retaining repeated leaves as the Windows importer does.
pub fn apply_drop(
    rows: &mut [ChannelMapping],
    start: usize,
    nodes: &[Uuid],
    repeat: usize,
    mixing: bool,
    order: &str,
    tree: &ElementTree,
) -> Result<(), SessionError> {
    let repeat = if mixing { 3 } else { repeat.clamp(1, 20) };
    if mixing && !["RGB", "RBG", "BRG", "BGR", "GRB", "GBR"].contains(&order) {
        return Err(error("Invalid color order."));
    }
    let mut row = start;
    for node in nodes.iter().filter_map(|id| tree.find(id)) {
        for leaf in tree.leaves(node) {
            for n in 0..repeat {
                let Some(mapping) = rows.get_mut(row) else {
                    return Ok(());
                };
                mapping.destination = Some(tree.node(leaf).id);
                mapping.mixing = mixing;
                mapping.import_color = if mixing {
                    match order.as_bytes()[n] {
                        b'R' => Color::RED,
                        b'G' => Color::GREEN,
                        _ => Color::BLUE,
                    }
                } else {
                    mapping.color
                };
                row += 1;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedEffect {
    pub target: Uuid,
    pub color: Color,
    pub start_ms: u64,
    pub duration_ms: u64,
    pub ramp: bool,
    pub start_level: u8,
    pub end_level: u8,
}
#[derive(Copy, Clone, PartialEq)]
enum State {
    Waiting,
    Holding,
    Single,
    Up,
    Down,
}
/// F99's state transitions; the first event after emission is reprocessed.
pub fn channel_effects(
    target: Uuid,
    period: u32,
    values: impl IntoIterator<Item = (Color, u8)>,
) -> Vec<ImportedEffect> {
    let mut output = Vec::new();
    let (mut state, mut color, mut start, mut end, mut first, mut current) =
        (State::Waiting, Color::EMPTY, 0, 0, 0, 0);
    let emit = |output: &mut Vec<ImportedEffect>, state, color, start, end, first, current| {
        output.push(ImportedEffect {
            target,
            color,
            start_ms: period as u64 * start,
            duration_ms: period as u64 * (end - start + 1),
            ramp: matches!(state, State::Up | State::Down),
            start_level: first,
            end_level: current,
        });
    };
    for (pos, (next_color, intensity)) in values.into_iter().enumerate() {
        loop {
            if state == State::Waiting {
                color = next_color;
                first = intensity;
                current = intensity;
                start = pos as u64;
                end = start;
                if intensity >= 5 {
                    state = State::Holding;
                }
                break;
            }
            let same = next_color == color;
            let next = match state {
                State::Holding if same && intensity == current => Some(State::Single),
                State::Holding if same && intensity > current => Some(State::Up),
                State::Holding if same && intensity < current && intensity > 5 => Some(State::Down),
                State::Single if same && intensity == current => Some(State::Single),
                State::Up if same && intensity > current => Some(State::Up),
                State::Down if same && intensity < current && intensity > 5 => Some(State::Down),
                _ => None,
            };
            if let Some(next) = next {
                state = next;
                end = pos as u64;
                current = intensity;
                break;
            }
            emit(&mut output, state, color, start, end, first, current);
            state = State::Waiting;
        }
    }
    if state != State::Waiting {
        emit(&mut output, state, color, start, end, first, current);
    }
    output
}

pub fn convert(
    file: &Vixen2File,
    mappings: &[ChannelMapping],
    tree: &ElementTree,
    mut progress: impl FnMut(usize),
) -> Result<(TimedSequence, Vec<String>), SessionError> {
    let mut warnings = file.warnings.clone();
    let mut by_target: HashMap<Uuid, Vec<&ChannelMapping>> = HashMap::new();
    for map in mappings {
        if let Some(id) = map.destination.filter(|id| tree.find(id).is_some()) {
            by_target.entry(id).or_default().push(map);
        }
    }
    let mut sequence = vixen_io::new_sequence();
    sequence.length = TimeSpan::from_ms(file.length_ms as f64);
    for (channel, map) in mappings.iter().take(file.channels.len()).enumerate() {
        progress(channel);
        let Some(target) = map.destination.filter(|id| tree.find(id).is_some()) else {
            continue;
        };
        let linked = &by_target[&target];
        let mixing = map.mixing && linked.len() == 3;
        if map.mixing && !mixing {
            warnings.push(format!(
                "Expected 3 RGB channels for '{}', found {}; converting without color mixing.",
                map.name,
                linked.len()
            ));
        }
        if mixing && map.import_color != Color::RED {
            continue;
        }
        if mixing {
            for other in linked {
                if ![Color::RED, Color::GREEN, Color::BLUE].contains(&other.import_color) {
                    warnings.push(format!("Skipping unexpected import color for '{}'; RGB channels require Red, Green or Blue.",other.name));
                }
            }
        }
        let value = |channel: usize, event: usize| -> Result<u8, SessionError> {
            channel
                .checked_mul(file.events_per_channel)
                .and_then(|n| n.checked_add(event))
                .and_then(|n| file.bytes.get(n).copied())
                .ok_or_else(|| error("Mapping channel number is outside the sequence event data."))
        };
        let mut events = Vec::with_capacity(file.events_per_channel);
        for event in 0..file.events_per_channel {
            if mixing {
                let mut rgb = [0u8; 3];
                for other in linked {
                    let component = match other.import_color {
                        Color::RED => 0,
                        Color::GREEN => 1,
                        Color::BLUE => 2,
                        _ => continue,
                    };
                    let n = other
                        .number
                        .parse::<usize>()
                        .ok()
                        .and_then(|n| n.checked_sub(1))
                        .ok_or_else(|| error("Invalid mapping channel number."))?;
                    rgb[component] = rgb[component].max(value(n, event)?);
                }
                let intensity = *rgb.iter().max().unwrap_or(&0);
                if intensity > 0 {
                    for component in &mut rgb {
                        *component = (*component as f64 * (255.0 / intensity as f64))
                            .round_ties_even()
                            .min(255.0) as u8;
                    }
                }
                events.push((Color::rgb(rgb[0], rgb[1], rgb[2]), intensity));
            } else {
                events.push((map.import_color, value(channel, event)?));
            }
        }
        for effect in channel_effects(target, file.period_ms, events) {
            let type_id = if effect.ramp {
                uuid!("cbd76d3b-c924-40ff-bad6-d1437b3dbdc0")
            } else {
                uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0")
            };
            let descriptor = vixen_effect_registry::by_type_id(&type_id)
                .ok_or_else(|| error("The basic effect module is unavailable."))?;
            let mut params = Params::defaults(descriptor.params);
            if effect.ramp {
                params.set(
                    "ColorGradient",
                    ParamValue::Gradient(ColorGradient::solid(effect.color)),
                );
                params.set(
                    "LevelCurve",
                    ParamValue::Curve(Curve::from_points(vec![
                        (0.0, effect.start_level as f64 / 2.55),
                        (100.0, effect.end_level as f64 / 2.55),
                    ])),
                );
            } else {
                params.set(
                    "Color",
                    ParamValue::Rgb(vixen_core::color::Rgb::from_color(effect.color)),
                );
                params.set(
                    "IntensityLevel",
                    ParamValue::Double(effect.end_level as f64 / 255.0),
                );
            }
            sequence.effects.push(EffectNode {
                start: TimeSpan::from_ms(effect.start_ms as f64),
                time_span: TimeSpan::from_ms(effect.duration_ms as f64),
                type_id,
                instance_id: Uuid::new_v4(),
                targets: vec![(target, tree.node(tree.find(&target).unwrap()).name.clone())],
                params,
            });
        }
    }
    progress(file.channels.len());
    Ok((sequence, warnings))
}

/// Preserve an existing profile media file: a conflicting imported track gets a unique name.
pub fn associate_audio(
    sequence: &mut TimedSequence,
    path: &Path,
    media_dir: &Path,
) -> Result<(), SessionError> {
    std::fs::create_dir_all(media_dir).map_err(error)?;
    let name = path
        .file_name()
        .ok_or_else(|| error("The audio file has no filename."))?;
    let mut target = media_dir.join(name);
    if path != target && target.exists() {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let mut n = 1;
        while target.exists() {
            target = media_dir.join(format!("{stem} ({n}).{ext}"));
            n += 1;
        }
    }
    if path != target {
        std::fs::copy(path, &target).map_err(error)?;
    }
    let instance = Uuid::new_v4();
    let type_id = uuid!("fe460392-3fab-4c63-99dd-d76c48354150");
    const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Media.Audio";
    let mut data = XElement::new(vixen_xml::ARRAYS_NS, "anyType");
    data.xsi_type = Some((NS.into(), "AudioData".into()));
    data.push(XElement::with_text(
        "",
        "ModuleInstanceId",
        instance.to_string(),
    ));
    data.push(XElement::with_text("", "ModuleTypeId", type_id.to_string()));
    data.push(XElement::with_text(
        NS,
        "FilePath",
        target.to_string_lossy(),
    ));
    sequence.media.push(vixen_io::MediaEntry {
        type_id,
        instance_id: instance,
        file_name: target.file_name().unwrap().to_string_lossy().into(),
        data: Some(data),
    });
    Ok(())
}

#[cfg(test)]
mod tests;
