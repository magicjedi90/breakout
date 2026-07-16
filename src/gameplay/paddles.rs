//! Paddle control (per-player input, mouse takeover) and paddle bounces.

use engine_core::prelude::*;
use crate::chaos_theme::theme_for;
use crate::constants::*;
use crate::effects;
use crate::types::*;
use super::{entity_position, entity_x};

/// Direction a ball leaves the bottom paddle, from where it struck.
///
/// `offset_frac` is the hit position relative to the paddle center in
/// half-widths: 0 = dead center, ±1 = the very edge. Center hits return
/// the ball straight up; edge hits deflect up to `PADDLE_MAX_BOUNCE_ANGLE`.
pub(crate) fn paddle_bounce_direction(offset_frac: f32) -> Vec2 {
    let angle = offset_frac.clamp(-1.0, 1.0) * PADDLE_MAX_BOUNCE_ANGLE;
    Vec2::new(angle.sin(), angle.cos())
}

/// Side-aware bounce: a paddle always returns the ball toward the field —
/// up off the bottom paddle, DOWN off the top one. Without the top-side
/// flip, the top paddle would fire balls into itself/the top sensor.
pub(crate) fn paddle_bounce_direction_for(offset_frac: f32, side: PaddleSide) -> Vec2 {
    let dir = paddle_bounce_direction(offset_frac);
    match side {
        PaddleSide::Bottom => dir,
        PaddleSide::Top => Vec2::new(dir.x, -dir.y),
    }
}

impl BreakoutGame {
    /// Both paddles' worth of side/entity/player/color, for uniform loops.
    pub(super) fn paddle_roster(&self) -> [(Option<EntityId>, PaddleSide, Vec4); 2] {
        [
            (self.paddle, PaddleSide::Bottom, PADDLE_COLOR),
            (self.paddle_top, PaddleSide::Top, PADDLE2_COLOR),
        ]
    }

    pub(super) fn update_paddles(&mut self, ctx: &GameContext) {
        if let Some(paddle) = self.paddle {
            self.update_paddle(ctx, paddle, PlayerId::P1, PADDLE_Y);
        }
        if let Some(paddle) = self.paddle_top {
            self.update_paddle(ctx, paddle, PlayerId::P2, PADDLE_TOP_Y);
        }
    }

    /// Move one paddle from its player's bindings (keys, dpad, or stick) or
    /// the mouse. Mouse takes over whenever it moves and belongs to the
    /// bottom paddle only (the mouse is player 1's device); bound input
    /// takes over whenever it's active.
    fn update_paddle(&mut self, ctx: &GameContext, paddle: EntityId, player: PlayerId, y: f32) {
        let x = entity_x(ctx.world, paddle);

        // Solo: the lone paddle listens to both players' devices, so WASD,
        // arrows, and either pad all work.
        let axis = match self.mode {
            GameMode::SinglePlayer => (ctx.players.move_x(PlayerId::P1, ctx.input)
                + ctx.players.move_x(PlayerId::P2, ctx.input))
            .clamp(-1.0, 1.0),
            GameMode::TwoPlayerCoop => ctx.players.move_x(player, ctx.input),
        };

        let mouse_moved =
            player == PlayerId::P1 && ctx.input.mouse_movement_delta().0.abs() > 0.0;
        let new_x = if axis != 0.0 {
            x + axis * PADDLE_SPEED * ctx.delta_time
        } else if mouse_moved {
            // Window pixels (origin top-left) → world (origin center).
            ctx.input.mouse_position().x - ctx.window_size.x / 2.0
        } else {
            x
        };

        let new_x = new_x.clamp(-PADDLE_MAX_X, PADDLE_MAX_X);
        self.physics.set_kinematic_target(paddle, Vec2::new(new_x, y), 0.0);
    }

    /// Paddle bounces: aim the ball by hit offset (toward the field, per
    /// side), reset the combo, apply the Insane speed gain, spray particles.
    pub(super) fn check_paddle_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let theme = theme_for(self.chaos_mode);

        for (paddle, side, color) in self.paddle_roster() {
            let Some(paddle) = paddle else { continue };
            let paddle_x = entity_x(ctx.world, paddle);

            for &ball in &self.all_balls() {
                let hit = collisions.iter()
                    .any(|c| c.event.started && c.event.involves(ball, paddle));
                if !hit { continue; }

                self.combo = 0;
                if self.chaos_mode.is_insane() {
                    self.speed_mult *= INSANE_SPEED_GAIN;
                }

                let Some(pos) = entity_position(ctx.world, ball) else { continue };
                // Override the physical reflection with offset-based aim —
                // this is what makes Breakout controllable rather than
                // deterministic.
                let offset = (pos.x - paddle_x) / (PADDLE_W / 2.0);
                let dir = paddle_bounce_direction_for(offset, side);
                let speed = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
                self.physics.set_velocity(ball, dir * speed, 0.0);

                ctx.particles.spawn_burst(pos, &effects::paddle_hit_burst(color, &theme, self.tex_id));
                if let Some(grid) = self.grid.as_mut() {
                    grid.apply_impulse(&GridImpulse::Radial {
                        position: pos,
                        strength: GRID_IMPULSE_PADDLE_HIT_STRENGTH,
                        radius: GRID_IMPULSE_PADDLE_HIT_RADIUS,
                        attractive: false,
                    });
                }
            }
        }
    }
}
