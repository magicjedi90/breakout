//! Falling power-up pickups and the effects they grant.
//!
//! Special bricks (tagged `drop_*` in the level scenes) drop a glowing
//! capsule when destroyed; the player must catch it with the PADDLE.
//! Tracking/collection mechanics come from the engine's `Pickups` /
//! `EffectTimer`; this module owns what the pickups DO.

use engine_core::prelude::*;

use crate::chaos_theme::ChaosTheme;
use crate::constants::*;
use crate::effects;
use crate::gameplay::hash_f32;
use crate::types::*;

/// Capsule tint per pickup kind (mirrors the dropping brick's color).
pub(crate) fn pickup_color(kind: PickupKind) -> Vec4 {
    match kind {
        PickupKind::Multiball => MULTIBALL_PICKUP_COLOR,
        PickupKind::Wrecking => WRECKING_PICKUP_COLOR,
        PickupKind::Insiculous => INSICULOUS_PICKUP_COLOR,
    }
}

/// Editor-hierarchy display name for a spawned pickup entity.
fn pickup_entity_name(kind: PickupKind) -> &'static str {
    match kind {
        PickupKind::Multiball => "Pickup (Multiball)",
        PickupKind::Wrecking => "Pickup (Wrecking Ball)",
        PickupKind::Insiculous => "Pickup (Insiculous)",
    }
}

/// What catching a pickup grants: (extra balls spawned, wrecking refresh).
/// Insiculous is the trio's "both at once" — the pattern the whole engine
/// theme is named after.
pub(crate) fn pickup_effects(kind: PickupKind) -> (u32, bool) {
    match kind {
        PickupKind::Multiball => (1, false),
        PickupKind::Wrecking => (0, true),
        PickupKind::Insiculous => (1, true),
    }
}

/// Whether another extra ball may spawn given how many are live.
pub(crate) fn multiball_allowed(extra_count: usize) -> bool {
    extra_count < MAX_EXTRA_BALLS
}

impl BreakoutGame {
    pub(crate) fn wrecking_active(&self) -> bool {
        self.wrecking.active()
    }

    /// Drop a pickup capsule at a destroyed brick's position; it falls
    /// toward the paddle as a dynamic sensor (no ball interference).
    pub(crate) fn spawn_pickup(&mut self, world: &mut World, kind: PickupKind, pos: Vec2) {
        let entity = world
            .spawn()
            .with(Name::new(pickup_entity_name(kind)))
            .with(Transform2D::from_parts(pos, 0.0, Vec2::splat(PICKUP_SIZE / RENDER_UNIT)))
            .with(Sprite::new(self.tex_id).with_color(pickup_color(kind)).with_emissive(1.8))
            .with(
                RigidBody::new_dynamic()
                    .with_gravity_scale(0.0)
                    .with_rotation_locked(true),
            )
            .with(Collider::box_collider(PICKUP_SIZE, PICKUP_SIZE).as_sensor())
            .id();
        // Buffered-safe on the spawn frame; applied once the body syncs.
        self.physics.set_velocity(entity, Vec2::new(0.0, -PICKUP_FALL_SPEED), 0.0);
        self.pickups.track(entity, kind);
    }

    /// Resolve paddle catches from this frame's collision snapshot and grant
    /// the effects.
    pub(crate) fn check_pickup_catches(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
        paddle: EntityId,
    ) {
        let caught = self
            .pickups
            .collect(collisions, &[paddle], &mut self.physics, ctx.world);
        if caught.is_empty() {
            return;
        }

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        for (kind, _) in caught {
            let (extra_balls, wrecking) = pickup_effects(kind);
            for _ in 0..extra_balls {
                self.try_spawn_extra_ball(ctx);
            }
            if wrecking {
                // Catching a second one refreshes the clock — no stacking.
                self.wrecking.start(WRECKING_DURATION);
                self.apply_ball_visuals(ctx.world);
            }

            if let Some(pos) = ctx.world.get::<Transform2D>(paddle).map(|t| t.position) {
                ctx.particles.spawn_burst(
                    pos + Vec2::new(0.0, PADDLE_H),
                    &effects::pickup_catch_burst(pickup_color(kind), &theme, self.tex_id),
                );
            }
        }
    }

    /// Spawn a multiball-granted extra ball above the paddle, launched
    /// upward at a lightly randomized angle. Fizzles silently at the cap or
    /// outside active play.
    fn try_spawn_extra_ball(&mut self, ctx: &mut GameContext) {
        if !multiball_allowed(self.extra_balls.len()) || self.state != GameState::Playing {
            return;
        }
        let paddle_x = self
            .paddle
            .and_then(|p| ctx.world.get::<Transform2D>(p).map(|t| t.position.x))
            .unwrap_or(0.0);

        let ball = self.spawn_ball(ctx.world);
        // reset flushes before the velocity in the same physics update, so
        // the launch survives the reposition.
        self.physics
            .reset_body(ball, Vec2::new(paddle_x, PADDLE_Y + SERVE_OFFSET_Y));
        let angle = (hash_f32(self.frame_count.wrapping_add(7)) - 0.5) * 0.8;
        let dir = Vec2::new(angle.sin(), angle.cos());
        let speed = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
        self.physics.set_velocity(ball, dir * speed, 0.0);

        self.extra_balls.push(ball);
        // New ball adopts the current look (red-hot if wrecking is active).
        self.apply_ball_visuals(ctx.world);
    }

    /// Despawn pickups that fell past the paddle: bottom-sensor hit, or the
    /// y-threshold safety net in case a sensor event is ever missed.
    pub(crate) fn despawn_missed_pickups(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
    ) {
        if self.pickups.is_empty() {
            return;
        }
        let cutoff = -(WIN_H / 2.0 + 60.0);
        let doomed: Vec<EntityId> = self
            .pickups
            .entities()
            .filter(|&e| {
                let sensor_hit = self.bottom_sensor.is_some_and(|s| {
                    collisions.iter().any(|c| c.event.started && c.event.involves(e, s))
                });
                let fell = ctx
                    .world
                    .get::<Transform2D>(e)
                    .is_none_or(|t| !t.position.y.is_finite() || t.position.y < cutoff);
                sensor_hit || fell
            })
            .collect();
        if !doomed.is_empty() {
            self.pickups
                .remove_where(&mut self.physics, ctx.world, |p| doomed.contains(&p.entity));
        }
    }

    /// Tick the wrecking clock; when it expires, cool the balls back down.
    pub(crate) fn update_wrecking(&mut self, ctx: &mut GameContext) {
        if self.wrecking.tick(ctx.delta_time) {
            self.apply_ball_visuals(ctx.world);
        }
    }

    /// Push the current effect state onto every live ball's sprite:
    /// red-hot while wrecking, the chaos theme's ball look otherwise.
    /// Called on effect start/expiry AND on every ball spawn, so nothing
    /// can keep a stale look.
    pub(crate) fn apply_ball_visuals(&self, world: &mut World) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let (color, emissive) = if self.wrecking.active() {
            (WRECKING_BALL_COLOR, WRECKING_BALL_EMISSIVE)
        } else {
            (theme.ball_color, BALL_EMISSIVE)
        };
        for ball in self.ball.into_iter().chain(self.extra_balls.iter().copied()) {
            if let Some(s) = world.get_mut::<Sprite>(ball) {
                s.color = color;
                s.emissive = emissive;
            }
        }
    }

    /// Remove every in-flight pickup (match end / reset).
    pub(crate) fn destroy_all_pickups(&mut self, world: &mut World) {
        self.pickups.clear(&mut self.physics, world);
    }

    /// Make drop-bricks visibly pulse so players can spot the prizes.
    /// Owns the emissive channel for drop bricks only — armor damage owns
    /// the color channel, so the two never fight.
    pub(crate) fn pulse_drop_bricks(&self, world: &mut World) {
        let glow = 1.2 + 0.6 * (self.frame_count as f32 * 0.12).sin();
        for brick in self.bricks.iter().filter(|b| b.drop.is_some()) {
            if let Some(s) = world.get_mut::<Sprite>(brick.entity) {
                s.emissive = glow;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pickup_effects_form_the_insiculous_trio() {
        assert_eq!(pickup_effects(PickupKind::Multiball), (1, false));
        assert_eq!(pickup_effects(PickupKind::Wrecking), (0, true));
        // Insiculous = both base powers at once.
        let (balls, wrecking) = pickup_effects(PickupKind::Insiculous);
        assert_eq!(balls, pickup_effects(PickupKind::Multiball).0);
        assert!(wrecking);
    }

    #[test]
    fn multiball_cap_blocks_at_limit() {
        assert!(multiball_allowed(0));
        assert!(multiball_allowed(MAX_EXTRA_BALLS - 1));
        assert!(!multiball_allowed(MAX_EXTRA_BALLS));
        assert!(!multiball_allowed(MAX_EXTRA_BALLS + 1));
    }

    #[test]
    fn pickup_colors_match_brick_authoring_colors() {
        // The level scenes author drop-bricks in these exact colors so the
        // player can read what a brick drops before breaking it.
        assert_eq!(pickup_color(PickupKind::Multiball), MULTIBALL_PICKUP_COLOR);
        assert_eq!(pickup_color(PickupKind::Wrecking), WRECKING_PICKUP_COLOR);
        assert_eq!(pickup_color(PickupKind::Insiculous), INSICULOUS_PICKUP_COLOR);
    }
}
