//! Visual effect presets — particle configs. Centralizes the look of each
//! event (hop, death, home fill) so tuning happens in one place.

use engine_core::prelude::*;
use glam::Vec4;

use crate::types::DeathCause;

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

/// Death blowout, tinted by what killed the frog (blue splash for water
/// deaths, accent flash for everything else).
pub(crate) fn death_burst(cause: DeathCause, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let color = match cause {
        DeathCause::Drown | DeathCause::Swept => Vec4::new(0.3, 0.55, 1.0, 1.0),
        _ => theme.accent_color,
    };
    let count = (50.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.3, 0.9)
        .with_speed(90.0, 380.0)
        .with_direction(Vec2::Y, std::f32::consts::PI)
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(7.0, 0.5)
        .with_drag(2.0)
        .with_emissive(2.6)
        .with_texture(tex)
}

/// Celebratory green shower when a home slot fills.
pub(crate) fn home_fill_burst(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let color = Vec4::new(0.45, 1.0, 0.5, 1.0);
    let count = (36.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.25, 0.8)
        .with_speed(60.0, 260.0)
        .with_direction(Vec2::Y, std::f32::consts::FRAC_PI_2)
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(5.0, 0.5)
        .with_drag(1.6)
        .with_emissive(2.4)
        .with_texture(tex)
}
