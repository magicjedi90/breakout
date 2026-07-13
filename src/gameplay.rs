use engine_core::prelude::*;
use crate::chaos_theme::theme_for;
use crate::constants::*;
use crate::effects;
use crate::types::*;

fn entity_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world.get::<Transform2D>(entity).map(|t| t.position)
}

fn entity_x(world: &World, entity: EntityId) -> f32 {
    world.get::<Transform2D>(entity).map(|t| t.position.x).unwrap_or(0.0)
}

/// Direction a ball leaves the paddle, from where it struck.
///
/// `offset_frac` is the hit position relative to the paddle center in
/// half-widths: 0 = dead center, ±1 = the very edge. Center hits return
/// the ball straight up; edge hits deflect up to `PADDLE_MAX_BOUNCE_ANGLE`.
pub(crate) fn paddle_bounce_direction(offset_frac: f32) -> Vec2 {
    let angle = offset_frac.clamp(-1.0, 1.0) * PADDLE_MAX_BOUNCE_ANGLE;
    Vec2::new(angle.sin(), angle.cos())
}

/// Velocity a ball should leave a destroyed brick with.
///
/// Bricks are destroyed the instant a contact starts, which cancels rapier's
/// contact impulse whenever the ball catches a brick corner or squeezes into
/// the gap between two bricks (the contact normal is horizontal there, so
/// nothing pushes the ball back) — the ball would plough straight through
/// the grid. So, like paddle bounces, the game supplies the reflection:
/// push the velocity *away from the brick* on the dominant contact axis.
/// Idempotent — if rapier already reflected the ball, this changes nothing.
pub(crate) fn brick_bounce_velocity(ball_pos: Vec2, vel: Vec2, brick_pos: Vec2) -> Vec2 {
    let d = ball_pos - brick_pos;
    let away_x = if d.x >= 0.0 { 1.0 } else { -1.0 };
    let away_y = if d.y >= 0.0 { 1.0 } else { -1.0 };
    // Normalize the offset by the brick's half-extents to find which face
    // the ball is closest to; ties go to vertical (breakout balls travel
    // mostly vertically).
    let mut v = vel;
    if d.y.abs() / (BRICK_H / 2.0) >= d.x.abs() / (BRICK_W / 2.0) {
        v.y = v.y.abs() * away_y;
    } else {
        v.x = v.x.abs() * away_x;
    }
    v
}

/// What a brick hit does, given the brick's remaining hits and whether the
/// wrecking ball is active (wrecking one-hit-kills anything).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrickHitOutcome {
    Destroyed,
    Damaged { hits_left: u32 },
}

pub(crate) fn brick_hit_outcome(hits_left: u32, wrecking_active: bool) -> BrickHitOutcome {
    if wrecking_active || hits_left <= 1 {
        BrickHitOutcome::Destroyed
    } else {
        BrickHitOutcome::Damaged { hits_left: hits_left - 1 }
    }
}

/// Re-aim `dir` if it is too horizontal, preserving its left/right and
/// up/down senses. Keeps the ball from shuttling between the side walls.
pub(crate) fn enforce_min_vertical(dir: Vec2) -> Vec2 {
    if dir.y.abs() >= MIN_VERTICAL_FRACTION {
        return dir;
    }
    let y_sign = if dir.y == 0.0 { 1.0 } else { dir.y.signum() };
    Vec2::new(
        dir.x.signum() * (1.0 - MIN_VERTICAL_FRACTION * MIN_VERTICAL_FRACTION).sqrt(),
        y_sign * MIN_VERTICAL_FRACTION,
    )
}

impl BreakoutGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        let Some(paddle) = self.paddle else { return };

        // F1 toggles the collider debug overlay. Magenta outlines render on
        // top of sprites so any sprite-vs-collider mismatch is obvious.
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        self.update_paddle(ctx, paddle);
        self.physics.update(ctx.world, ctx.delta_time);

        // Drain this frame's collision events once (take = the buffer is
        // consumed, not borrowed). Every consumer below shares this Vec, and
        // no borrow of `self.physics` is held while reacting.
        let collisions: Vec<CollisionData> = self.physics.take_collision_events();

        self.glue_serving_ball(ctx.world, paddle);
        self.handle_state_input(ctx);
        self.maintain_all_ball_velocities();
        self.check_paddle_hits(ctx, &collisions, paddle);
        self.check_brick_hits(ctx, &collisions);
        self.check_pickup_catches(ctx, &collisions, paddle);
        self.despawn_missed_pickups(ctx, &collisions);
        self.update_wrecking(ctx);
        self.pulse_drop_bricks(ctx.world);
        self.check_ball_loss(ctx, &collisions);
        self.check_win_condition(ctx);

        // Step + render the deforming grid after gameplay so it reacts to
        // this frame's collisions.
        self.step_and_emit_grid(ctx);
    }

    /// Advance the spring-mass grid and push its line vertices into the
    /// engine's per-frame line buffer. When the collider-debug overlay is
    /// enabled, the collider outlines are pushed on top.
    fn step_and_emit_grid(&mut self, ctx: &mut GameContext) {
        engine_core::grid::step_and_emit_grid(
            self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders,
        );
    }

    /// Move the paddle from keyboard or mouse. Mouse takes over whenever it
    /// moves; keys take over whenever they're pressed.
    fn update_paddle(&mut self, ctx: &GameContext, paddle: EntityId) {
        let x = entity_x(ctx.world, paddle);

        let left = ctx.input.is_key_pressed(KeyCode::ArrowLeft) || ctx.input.is_key_pressed(KeyCode::KeyA);
        let right = ctx.input.is_key_pressed(KeyCode::ArrowRight) || ctx.input.is_key_pressed(KeyCode::KeyD);
        let key_dx = match (left, right) {
            (true, false) => -PADDLE_SPEED,
            (false, true) => PADDLE_SPEED,
            _ => 0.0,
        };

        let mouse_moved = ctx.input.mouse_movement_delta().0.abs() > 0.0;
        let new_x = if key_dx != 0.0 {
            x + key_dx * ctx.delta_time
        } else if mouse_moved {
            // Window pixels (origin top-left) → world (origin center).
            ctx.input.mouse_position().x - ctx.window_size.x / 2.0
        } else {
            x
        };

        let new_x = new_x.clamp(-PADDLE_MAX_X, PADDLE_MAX_X);
        self.physics.set_kinematic_target(paddle, Vec2::new(new_x, PADDLE_Y), 0.0);
    }

    /// While serving, park the ball on top of the paddle every frame.
    /// Runs after `physics.update()`, so the paddle transform already
    /// reflects this frame's kinematic target.
    fn glue_serving_ball(&mut self, world: &mut World, paddle: EntityId) {
        if self.state != GameState::Serving { return; }
        let Some(ball) = self.ball else { return };
        let x = entity_x(world, paddle);
        // reset_body zeroes velocity too, so the ball just rides the paddle.
        self.physics.reset_body(ball, Vec2::new(x, PADDLE_Y + SERVE_OFFSET_Y));
    }

    fn handle_state_input(&mut self, ctx: &mut GameContext) {
        let launch = ctx.input.is_key_just_pressed(KeyCode::Space)
            || ctx.input.is_key_just_pressed(KeyCode::Enter)
            || ctx.input.is_mouse_button_just_pressed(MouseButton::Left);

        match &self.state {
            GameState::Serving => {
                if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                } else if launch {
                    self.launch_balls(ctx);
                }
            }
            GameState::GameOver { .. } => {
                if ctx.input.is_key_just_pressed(KeyCode::Space) {
                    self.start_game(ctx);
                } else if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                }
            }
            GameState::Playing => {
                if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                }
            }
            _ => {}
        }
    }

    /// Fire the served ball upward at a slightly random angle. Ridiculous
    /// mode launches a second ball mirrored the other way.
    fn launch_balls(&mut self, ctx: &mut GameContext) {
        let Some(ball) = self.ball else { return };

        let angle = (hash_f32(self.frame_count) - 0.5) * LAUNCH_ANGLE_SPREAD; // ±0.3 rad off vertical
        let dir = Vec2::new(angle.sin(), angle.cos());
        let speed = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
        self.physics.set_velocity(ball, dir * speed, 0.0);

        if self.chaos_mode.is_ridiculous() {
            let pos = entity_position(ctx.world, ball).unwrap_or(Vec2::new(0.0, PADDLE_Y + SERVE_OFFSET_Y));
            let extra = self.spawn_ball(ctx.world);
            let theme = theme_for(self.chaos_mode);
            if let Some(t) = ctx.world.get_mut::<Transform2D>(extra) {
                t.position = pos;
            }
            if let Some(s) = ctx.world.get_mut::<Sprite>(extra) {
                s.color = theme.accent_color;
            }
            let dir2 = Vec2::new(-angle.sin(), angle.cos());
            self.physics.set_velocity(extra, dir2 * speed, 0.0);
            self.extra_balls.push(extra);
        }

        self.state = GameState::Playing;
    }

    fn all_balls(&self) -> Vec<EntityId> {
        self.ball.into_iter().chain(self.extra_balls.iter().copied()).collect()
    }

    /// Hold every live ball at its target speed and keep it from going
    /// fully horizontal.
    fn maintain_all_ball_velocities(&mut self) {
        if self.state != GameState::Playing { return; }
        let target = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
        for ball in self.all_balls() {
            let Some((vel, _)) = self.physics.get_body_velocity(ball) else { continue };
            let speed = vel.length();
            if speed < 1.0 { continue; }
            let dir = enforce_min_vertical(vel / speed);
            let new_vel = dir * target;
            if (new_vel - vel).length() > 1.0 {
                self.physics.set_velocity(ball, new_vel, 0.0);
            }
        }
    }

    /// Paddle bounces: aim the ball by hit offset, reset the combo, apply
    /// the Insane speed gain, and spray particles.
    fn check_paddle_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData], paddle: EntityId) {
        let theme = theme_for(self.chaos_mode);
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
            // Override the physical reflection with offset-based aim — this
            // is what makes Breakout controllable rather than deterministic.
            let offset = (pos.x - paddle_x) / (PADDLE_W / 2.0);
            let dir = paddle_bounce_direction(offset);
            let speed = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
            self.physics.set_velocity(ball, dir * speed, 0.0);

            ctx.particles.spawn_burst(pos, &effects::paddle_hit_burst(PADDLE_COLOR, &theme, self.tex_id));
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

    /// Resolve brick hits: armored bricks take damage (rapier reflects the
    /// ball naturally — the brick survives, so its contact impulse lands);
    /// destroyed bricks score, grow the combo, and may drop a pickup.
    fn check_brick_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let all_balls = self.all_balls();

        let mut hit: Vec<usize> = Vec::new();
        for collision in collisions {
            if !collision.event.started { continue; }
            for (i, brick) in self.bricks.iter().enumerate() {
                if all_balls.iter().any(|&b| collision.event.involves(b, brick.entity)) {
                    hit.push(i);
                }
            }
        }
        // A brick can appear once per ball that touched it this frame —
        // dedup so it only takes one hit. Ascending order for reverse removal.
        hit.sort_unstable();
        hit.dedup();
        if hit.is_empty() { return; }

        let wrecking_active = self.wrecking_active();
        let theme = theme_for(self.chaos_mode);
        // Descending order: removals at index i never shift lower indices.
        for &i in hit.iter().rev() {
            match brick_hit_outcome(self.bricks[i].hits_left, wrecking_active) {
                BrickHitOutcome::Damaged { hits_left } => {
                    self.bricks[i].hits_left = hits_left;
                    let entity = self.bricks[i].entity;
                    // Visible battle damage: dim the tint and the glow.
                    if let Some(s) = ctx.world.get_mut::<Sprite>(entity) {
                        s.color.x *= BRICK_DAMAGE_COLOR_FACTOR;
                        s.color.y *= BRICK_DAMAGE_COLOR_FACTOR;
                        s.color.z *= BRICK_DAMAGE_COLOR_FACTOR;
                        s.emissive *= BRICK_DAMAGE_EMISSIVE_FACTOR;
                    }
                    if let Some(pos) = entity_position(ctx.world, entity) {
                        ctx.particles.spawn_burst(
                            pos,
                            &effects::armor_hit_burst(self.bricks[i].color, &theme, self.tex_id),
                        );
                    }
                }
                BrickHitOutcome::Destroyed => {
                    let brick = self.bricks.remove(i);
                    self.score += brick.value;
                    self.combo += 1;
                    if self.combo == COMBO_TARGET {
                        ctx.achievements.unlock(crate::achievements::COMBO_VOLLEY);
                    }
                    self.destroy_brick_entity(ctx, collisions, &all_balls, &brick, &theme);
                }
            }
        }
    }

    /// Tear down a destroyed brick: reflect the balls that hit it, burst
    /// particles, kick the grid, drop its pickup, and remove the entity.
    fn destroy_brick_entity(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
        all_balls: &[EntityId],
        brick: &Brick,
        theme: &ChaosTheme,
    ) {
        if let Some(pos) = entity_position(ctx.world, brick.entity) {
            // Destroying the brick cancels rapier's contact impulse, so
            // reflect every ball that hit it ourselves (see
            // brick_bounce_velocity for why corner/gap hits need this).
            for &ball in all_balls {
                let hit_this = collisions.iter()
                    .any(|c| c.event.started && c.event.involves(ball, brick.entity));
                if !hit_this { continue; }
                if let (Some(ball_pos), Some((vel, _))) =
                    (entity_position(ctx.world, ball), self.physics.get_body_velocity(ball))
                {
                    let new_vel = brick_bounce_velocity(ball_pos, vel, pos);
                    if new_vel != vel {
                        self.physics.set_velocity(ball, new_vel, 0.0);
                    }
                }
            }

            ctx.particles.spawn_burst(pos, &effects::brick_burst(brick.color, theme, self.tex_id));
            if let Some(grid) = self.grid.as_mut() {
                grid.apply_impulse(&GridImpulse::Radial {
                    position: pos,
                    strength: GRID_IMPULSE_BRICK_DESTROY_STRENGTH,
                    radius: GRID_IMPULSE_BRICK_DESTROY_RADIUS,
                    attractive: false,
                });
            }

            if let Some(kind) = brick.drop {
                self.spawn_pickup(ctx.world, kind, pos);
            }
        }
        self.physics.destroy_entity(ctx.world, brick.entity);
    }

    /// Remove balls that fell past the paddle (sensor hit or escaped the
    /// playfield entirely). When none remain, spend a life.
    fn check_ball_loss(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        if self.state != GameState::Playing { return; }
        let Some(sensor) = self.bottom_sensor else { return };

        let bound_x = WIN_W / 2.0 + BALL_LOST_BOUNDS_PAD;
        let bound_y = WIN_H / 2.0 + BALL_LOST_BOUNDS_PAD;
        let mut lost: Vec<EntityId> = Vec::new();
        for &ball in &self.all_balls() {
            let sensor_hit = collisions.iter()
                .any(|c| c.event.started && c.event.involves(ball, sensor));
            // Safety net: a CCD miss or NaN position also counts as lost.
            let escaped = entity_position(ctx.world, ball).is_none_or(|p| {
                !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > bound_x || p.y.abs() > bound_y
            });
            if sensor_hit || escaped {
                lost.push(ball);
            }
        }
        if lost.is_empty() { return; }

        let theme = theme_for(self.chaos_mode);
        for ball in lost {
            let pos = entity_position(ctx.world, ball).unwrap_or(Vec2::new(0.0, -WIN_H / 2.0));
            ctx.particles.spawn_burst(pos, &effects::ball_lost_burst(&theme, self.tex_id));
            if let Some(grid) = self.grid.as_mut() {
                grid.apply_impulse(&GridImpulse::Radial {
                    position: pos,
                    strength: GRID_IMPULSE_BALL_LOST_STRENGTH,
                    radius: GRID_IMPULSE_BALL_LOST_RADIUS,
                    attractive: false,
                });
            }

            if Some(ball) == self.ball {
                self.ball = self.extra_balls.pop();
            } else {
                self.extra_balls.retain(|&e| e != ball);
            }
            self.physics.destroy_entity(ctx.world, ball);
        }

        if self.ball.is_some() { return; }

        // All balls gone — spend a life. Wrecking dies with the volley
        // (consistent with the speed_mult reset below).
        self.lives = self.lives.saturating_sub(1);
        self.combo = 0;
        self.speed_mult = 1.0;
        self.wrecking.stop();
        if self.lives == 0 {
            self.destroy_all_pickups(ctx.world);
            self.state = GameState::GameOver { won: false };
            return;
        }

        let fresh = self.spawn_ball(ctx.world);
        let theme = theme_for(self.chaos_mode);
        if let Some(s) = ctx.world.get_mut::<Sprite>(fresh) {
            s.color = theme.accent_color;
        }
        self.ball = Some(fresh);
        self.state = GameState::Serving;
    }

    fn check_win_condition(&mut self, ctx: &mut GameContext) {
        if !matches!(self.state, GameState::Playing | GameState::Serving) { return; }
        if !self.bricks.is_empty() { return; }

        self.destroy_all_balls(ctx.world);
        self.destroy_all_pickups(ctx.world);
        self.wrecking.stop();
        self.unlock_win_achievements(ctx);
        self.state = GameState::GameOver { won: true };
    }

    pub(crate) fn destroy_all_balls(&mut self, world: &mut World) {
        if let Some(ball) = self.ball.take() {
            self.physics.destroy_entity(world, ball);
        }
        for ball in self.extra_balls.drain(..) {
            self.physics.destroy_entity(world, ball);
        }
    }

    fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_all_balls(world);
        self.destroy_all_pickups(world);
        self.wrecking.stop();
        self.state = GameState::TitleScreen { selection: 0 };
    }

    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = !matches!(
            self.state,
            GameState::TitleScreen { .. } | GameState::LevelSelect { .. } | GameState::Achievements
        );
        let entities = [self.paddle, self.ball].into_iter().flatten()
            .chain(self.extra_balls.iter().copied())
            .chain(self.walls.iter().copied())
            .chain(self.bricks.iter().map(|b| b.entity))
            .chain(self.pickups.entities().collect::<Vec<_>>());
        for entity in entities {
            if let Some(sprite) = ctx.world.get_mut::<Sprite>(entity) {
                sprite.visible = visible;
            }
        }
    }
}
