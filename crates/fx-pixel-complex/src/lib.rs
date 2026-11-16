//! Effect modules: pattern, wave, whirlpool. Generated tables live in `generated`.

pub mod generated;
pub mod pattern;
pub mod wave;
pub mod whirlpool;

use vixen_effects::framework::EffectDescriptor;

pub fn descriptors() -> Vec<&'static EffectDescriptor> {
    vec![&pattern::DESCRIPTOR, &wave::DESCRIPTOR, &whirlpool::DESCRIPTOR, ]
}
