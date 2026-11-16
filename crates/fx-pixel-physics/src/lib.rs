//! Effect modules: liquid, morph. Generated tables live in `generated`.

pub mod generated;
pub mod liquid;
pub mod morph;

#[cfg(test)]
mod test_util;

use vixen_effects::framework::EffectDescriptor;

pub fn descriptors() -> Vec<&'static EffectDescriptor> {
    vec![&liquid::DESCRIPTOR, &morph::DESCRIPTOR]
}
