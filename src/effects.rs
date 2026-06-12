//! Visual effect presets — particle configs and grid setup.
//!
//! Centralizes the look of each event (brick destroyed, paddle hit, ball
//! lost) so tuning happens in one place.

use engine_core::prelude::*;

use crate::chaos_theme::ChaosTheme;

/// 32×24-node grid sized to cover the playfield with some overscan.
pub(crate) fn build_grid(theme: &ChaosTheme) -> GridMesh {
    GridMesh::new(32, 24, 36.0, Vec2::ZERO)
        .with_color(theme.grid_color)
        .with_emissive(0.7)
        .with_stiffness(60.0)
        .with_damping(0.07)
}

/// Omnidirectional shatter when a brick dies, tinted to the brick's color.
pub(crate) fn brick_burst(color: Vec4, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (30.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.25, 0.6)
        .with_speed(100.0, 340.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(7.0, 0.5)
        .with_drag(2.2)
        .with_emissive(2.2)
        .with_texture(tex)
}

/// Upward spray when the ball bounces off the paddle.
pub(crate) fn paddle_hit_burst(color: Vec4, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (18.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.2, 0.45)
        .with_speed(120.0, 280.0)
        .with_direction(Vec2::Y, std::f32::consts::FRAC_PI_3) // ~60° half-cone
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(5.0, 0.5)
        .with_drag(2.5)
        .with_emissive(1.8)
        .with_texture(tex)
}

/// Large explosion when a ball falls past the paddle.
pub(crate) fn ball_lost_burst(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let color = Vec4::new(1.0, 0.35, 0.25, 1.0);
    let count = (70.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.4, 0.9)
        .with_speed(120.0, 480.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(9.0, 0.5)
        .with_drag(1.7)
        .with_emissive(2.8)
        .with_texture(tex)
}
