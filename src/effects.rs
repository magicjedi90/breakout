//! Visual effect presets — particle configs.
//!
//! Centralizes the look of each event (brick destroyed, paddle hit, ball
//! lost) so tuning happens in one place. The deforming grid uses the
//! engine's `default_playfield_grid` preset directly.

use engine_core::prelude::*;

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

/// Small metallic spark when an armored brick takes a non-final hit.
pub(crate) fn armor_hit_burst(color: Vec4, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (10.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.15, 0.35)
        .with_speed(80.0, 220.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(4.0, 0.5)
        .with_drag(2.8)
        .with_emissive(1.6)
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

/// Celebratory pop when the paddle catches a pickup, tinted to its kind.
pub(crate) fn pickup_catch_burst(color: Vec4, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (24.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.25, 0.55)
        .with_speed(120.0, 300.0)
        .with_direction(Vec2::Y, std::f32::consts::FRAC_PI_2) // upward fan
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(6.0, 0.5)
        .with_drag(2.4)
        .with_emissive(2.4)
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
