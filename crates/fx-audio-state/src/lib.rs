//! Effect modules: lip_sync, state, vertical_meter, vu_meter, waveform. Generated tables live in `generated`.

pub mod audio_helper;
pub mod generated;
pub mod lip_sync;
pub mod property_data;
pub mod state;
pub mod vertical_meter;
pub mod vu_meter;
pub mod visual;
pub mod waveform;

use vixen_effects::framework::EffectDescriptor;

pub fn descriptors() -> Vec<&'static EffectDescriptor> {
    vec![&lip_sync::DESCRIPTOR, &state::DESCRIPTOR, &vertical_meter::DESCRIPTOR, &vu_meter::DESCRIPTOR, &waveform::DESCRIPTOR, ]
}
