//! Particle presets — the little that is still a particle now that deaths and
//! arrivals are drawn one-shots. Centralizes what is left so tuning happens in
//! one place.

use engine_core::prelude::*;

use crate::constants::SPLASH_COLOR;

/// Small dust puff under every hop.
pub(crate) fn hop_puff(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (6.0 * theme.particle_count_mult).round() as usize;
    let color = theme.structure_color;
    ParticleConfig::burst(count)
        .with_lifetime(0.1, 0.3)
        .with_speed(30.0, 90.0)
        .with_direction(Vec2::Y, std::f32::consts::PI)
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(3.0, 0.5)
        .with_drag(3.0)
        .with_emissive(1.4)
        .with_texture(tex)
}

/// The soup a drowning or a sweep throws up, under the feather poof that is
/// the death itself. The soup's own sauce reds rather than a generic blue, so
/// the splash reads as the river that took the chicken.
pub(crate) fn soup_splash(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let color = SPLASH_COLOR;
    let count = (30.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.25, 0.7)
        .with_speed(70.0, 300.0)
        .with_direction(Vec2::Y, std::f32::consts::PI)
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(6.0, 0.5)
        .with_drag(2.0)
        .with_emissive(2.2)
        .with_texture(tex)
}
