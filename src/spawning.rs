use engine_core::prelude::*;
use crate::constants::*;
use crate::types::{BreakoutGame, Brick};

/// Spawn the player paddle near the bottom of the playfield.
///
/// Physics uses a horizontal capsule — the flat top returns balls
/// predictably, while the rounded left/right caps deflect edge hits
/// outward (on top of the offset-based bounce control in gameplay).
pub(crate) fn spawn_paddle(world: &mut World, tex: u32) -> EntityId {
    world.spawn()
        .with(Transform2D::from_parts(Vec2::new(0.0, PADDLE_Y), 0.0, PADDLE_SCALE))
        // The paddle glows strongly — the signature neon treatment for
        // player-controlled objects.
        .with(Sprite::new(tex).with_color(PADDLE_COLOR).with_emissive(1.5))
        .with(RigidBody::new_kinematic().with_rotation_locked(true))
        .with(Collider::new(ColliderShape::capsule_x(PADDLE_W, PADDLE_H * 0.5))
            .with_friction(0.0)
            .with_restitution(1.0))
        .id()
}

pub(crate) fn spawn_wall(world: &mut World, pos: Vec2, w: f32, h: f32, tex: u32, color: Vec4) -> EntityId {
    world.spawn()
        .with(Transform2D::from_parts(pos, 0.0, Vec2::new(w / RENDER_UNIT, h / RENDER_UNIT)))
        // Walls glow gently — they outline the playfield without dominating.
        .with(Sprite::new(tex).with_color(color).with_depth(-1.0).with_emissive(0.6))
        .with(RigidBody::new_static())
        .with(Collider::box_collider(w, h).with_friction(0.0).with_restitution(1.0))
        .id()
}

/// Sensor strip below the visible playfield — touching it costs a ball.
pub(crate) fn spawn_bottom_sensor(world: &mut World) -> EntityId {
    world.spawn()
        .with(Transform2D::new(Vec2::new(0.0, -(WIN_H / 2.0 + 30.0))))
        .with(RigidBody::new_static())
        .with(Collider::box_collider(WIN_W + 200.0, 20.0).as_sensor())
        .id()
}

/// Center X position of brick `col` (0-based, left to right).
pub(crate) fn brick_x(col: usize) -> f32 {
    let total = BRICK_COLS as f32 * BRICK_W + (BRICK_COLS as f32 - 1.0) * BRICK_GAP;
    -(total - BRICK_W) / 2.0 + col as f32 * (BRICK_W + BRICK_GAP)
}

/// Center Y position of brick `row` (0-based, top to bottom).
pub(crate) fn brick_y(row: usize) -> f32 {
    BRICK_TOP_Y - row as f32 * (BRICK_H + BRICK_GAP)
}

/// Score paid out by a brick in `row` — top rows are worth the most.
pub(crate) fn brick_value(row: usize) -> u32 {
    (BRICK_ROWS - row) as u32 * BRICK_VALUE_STEP
}

/// Spawn the full brick grid: `BRICK_ROWS` rainbow rows of `BRICK_COLS`
/// static box colliders.
pub(crate) fn spawn_bricks(world: &mut World, tex: u32) -> Vec<Brick> {
    let mut bricks = Vec::with_capacity(BRICK_ROWS * BRICK_COLS);
    for (row, &color) in BRICK_ROW_COLORS.iter().enumerate() {
        for col in 0..BRICK_COLS {
            let pos = Vec2::new(brick_x(col), brick_y(row));
            let entity = world.spawn()
                .with(Transform2D::from_parts(pos, 0.0, Vec2::new(BRICK_W / RENDER_UNIT, BRICK_H / RENDER_UNIT)))
                .with(Sprite::new(tex).with_color(color).with_emissive(0.9))
                .with(RigidBody::new_static())
                .with(Collider::box_collider(BRICK_W, BRICK_H)
                    .with_friction(0.0)
                    .with_restitution(1.0))
                .id();
            bricks.push(Brick { entity, value: brick_value(row), color, hits_left: 1, drop: None });
        }
    }
    bricks
}

impl BreakoutGame {
    /// Spawn a ball entity using the loaded ball PNG texture. The collider
    /// stays a true circle so reflections match what the player sees.
    pub(crate) fn spawn_ball(&self, world: &mut World) -> EntityId {
        world.spawn()
            .with(Transform2D::from_parts(Vec2::new(0.0, PADDLE_Y + SERVE_OFFSET_Y), 0.0, Vec2::splat(BALL_SCALE)))
            // Ball is the brightest object on screen — high emissive value
            // gives it a strong neon core that smears with motion via bloom.
            .with(Sprite::new(self.ball_tex_id).with_emissive(2.5))
            .with(RigidBody::new_dynamic()
                .with_gravity_scale(0.0)
                .with_rotation_locked(true)
                .with_linear_damping(0.0)
                .with_angular_damping(0.0)
                .with_ccd(true))
            .with(Collider::circle_collider(BALL_RADIUS)
                .with_friction(0.0)
                .with_restitution(1.0))
            .id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brick_grid_spawns_full_rainbow_grid() {
        let mut world = World::new();
        let bricks = spawn_bricks(&mut world, 0);
        assert_eq!(bricks.len(), BRICK_ROWS * BRICK_COLS);
    }

    #[test]
    fn brick_grid_fits_inside_playfield_walls() {
        let left_edge = brick_x(0) - BRICK_W / 2.0;
        let right_edge = brick_x(BRICK_COLS - 1) + BRICK_W / 2.0;
        assert!(left_edge > -PLAYFIELD_HALF_W, "grid pokes past left wall: {left_edge}");
        assert!(right_edge < PLAYFIELD_HALF_W, "grid pokes past right wall: {right_edge}");
        // Symmetric layout
        assert!((left_edge + right_edge).abs() < 0.001);
    }

    #[test]
    fn brick_rows_descend_and_stay_above_paddle() {
        assert!(brick_y(0) > brick_y(BRICK_ROWS - 1));
        let lowest = brick_y(BRICK_ROWS - 1) - BRICK_H / 2.0;
        assert!(lowest > PADDLE_Y + 100.0, "bricks too close to paddle: {lowest}");
    }

    #[test]
    fn top_row_bricks_pay_the_most() {
        assert_eq!(brick_value(0), BRICK_ROWS as u32 * BRICK_VALUE_STEP);
        assert_eq!(brick_value(BRICK_ROWS - 1), BRICK_VALUE_STEP);
        for row in 1..BRICK_ROWS {
            assert!(brick_value(row - 1) > brick_value(row));
        }
    }
}
