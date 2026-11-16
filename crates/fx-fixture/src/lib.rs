//! Effect modules: fixture, fixture_strobe, frost, gobo, line_dance, prism, set_position, set_zoom, spin_color_wheel. Generated tables live in `generated`.

pub mod generated;
pub mod spec;
pub mod fixture_base;
pub mod visual;
pub mod fixture;
pub mod fixture_strobe;
pub mod frost;
pub mod gobo;
pub mod line_dance;
pub mod prism;
pub mod set_position;
pub mod set_zoom;
pub mod spin_color_wheel;

use vixen_effects::framework::EffectDescriptor;

pub fn descriptors() -> Vec<&'static EffectDescriptor> {
    vec![&fixture::DESCRIPTOR, &fixture_strobe::DESCRIPTOR, &frost::DESCRIPTOR, &gobo::DESCRIPTOR, &line_dance::DESCRIPTOR, &prism::DESCRIPTOR, &set_position::DESCRIPTOR, &set_zoom::DESCRIPTOR, &spin_color_wheel::DESCRIPTOR, ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_effect_is_ported_and_draws_its_own_thumbnail() {
        let all = super::descriptors();
        assert_eq!(all.len(), 9);
        for d in all {
            assert!(d.ported, "{} is not ported", d.name);
            assert!(d.force_visual && d.visual.is_some(), "{} has no thumbnail", d.name);
        }
    }
}
