//! Effect modules: alternating, candle, chase, custom_value, dissolve, launcher, pulse, rds, set_level, spin, strobe, twinkle, wipe. Generated tables live in `generated`.

pub mod generated;
pub mod alternating;
pub mod candle;
pub mod chase;
pub mod common;
pub mod custom_value;
pub mod dissolve;
pub mod launcher;
pub mod pulse;
pub mod rds;
pub mod set_level;
pub mod spin;
pub mod strobe;
pub mod twinkle;
pub mod wipe;

use vixen_effects::framework::EffectDescriptor;

pub fn descriptors() -> Vec<&'static EffectDescriptor> {
    vec![&alternating::DESCRIPTOR, &candle::DESCRIPTOR, &chase::DESCRIPTOR, &custom_value::DESCRIPTOR, &dissolve::DESCRIPTOR, &launcher::DESCRIPTOR, &pulse::DESCRIPTOR, &rds::DESCRIPTOR, &set_level::DESCRIPTOR, &spin::DESCRIPTOR, &strobe::DESCRIPTOR, &twinkle::DESCRIPTOR, &wipe::DESCRIPTOR, ]
}
