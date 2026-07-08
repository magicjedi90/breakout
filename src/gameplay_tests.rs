//! Tests for the gameplay module (split out to keep gameplay.rs under
//! the 600-line limit).

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::{brick_bounce_velocity, enforce_min_vertical, paddle_bounce_direction};

#[test]
fn paddle_bounce_center_hit_goes_straight_up() {
    let dir = paddle_bounce_direction(0.0);
    assert!(dir.x.abs() < 0.0001);
    assert!((dir.y - 1.0).abs() < 0.0001);
}

#[test]
fn paddle_bounce_edge_hits_deflect_sideways_but_upward() {
    let right = paddle_bounce_direction(1.0);
    assert!(right.x > 0.8, "edge hit should deflect hard right: {right:?}");
    assert!(right.y > 0.0, "ball must still travel upward: {right:?}");

    let left = paddle_bounce_direction(-1.0);
    assert!((left.x + right.x).abs() < 0.0001, "bounce should be symmetric");
    assert_eq!(left.y, right.y);
}

#[test]
fn paddle_bounce_clamps_past_edge_overshoot() {
    assert_eq!(paddle_bounce_direction(5.0), paddle_bounce_direction(1.0));
    assert_eq!(paddle_bounce_direction(-5.0), paddle_bounce_direction(-1.0));
}

#[test]
fn paddle_bounce_directions_are_unit_length() {
    for offset in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let dir = paddle_bounce_direction(offset);
        assert!((dir.length() - 1.0).abs() < 0.0001);
    }
}

#[test]
fn brick_bounce_reflects_upward_hit_downward() {
    // Ball below the brick moving up — must leave moving down.
    let v = brick_bounce_velocity(Vec2::new(0.0, 80.0), Vec2::new(50.0, 300.0), Vec2::new(0.0, 100.0));
    assert_eq!(v, Vec2::new(50.0, -300.0));
}

#[test]
fn brick_bounce_is_idempotent_when_already_reflected() {
    // Rapier already reflected the ball — correction must not undo it.
    let v = brick_bounce_velocity(Vec2::new(0.0, 80.0), Vec2::new(50.0, -300.0), Vec2::new(0.0, 100.0));
    assert_eq!(v, Vec2::new(50.0, -300.0));
}

#[test]
fn brick_bounce_side_hits_reflect_horizontally() {
    // Ball level with the brick, to its left, moving right → pushed left.
    let v = brick_bounce_velocity(Vec2::new(-45.0, 100.0), Vec2::new(300.0, 50.0), Vec2::new(0.0, 100.0));
    assert_eq!(v, Vec2::new(-300.0, 50.0));
}

#[test]
fn brick_bounce_gap_squeeze_still_reflects_vertically() {
    // Ball wedged in the 4px gap under two bricks: it sits mostly BELOW
    // the brick (large normalized dy) even though it is off to the side.
    // The dominant axis must be vertical, sending it back down — this is
    // the exact "ploughs through the grid" scenario.
    let brick = Vec2::new(37.0, 100.0); // one of the two gap neighbors
    let ball = Vec2::new(0.0, 80.0);    // in the gap, below the row
    let v = brick_bounce_velocity(ball, Vec2::new(20.0, 300.0), brick);
    assert_eq!(v, Vec2::new(20.0, -300.0));
}

/// End-to-end regression for the "ball ploughs through bricks" bug:
/// simulate the real frame loop (step physics, hold ball speed, destroy
/// hit bricks, apply brick_bounce_velocity) against the full grid and
/// assert the ball reflects off every brick it destroys.
#[test]
fn ball_reflects_off_every_brick_it_destroys() {
    let mut world = World::new();
    let mut physics = PhysicsSystem::with_config(PhysicsConfig::top_down());

    // Playfield walls like init(): top + sides.
    let spawn_wall = |world: &mut World, pos: Vec2, w: f32, h: f32| {
        world.spawn()
            .with(Transform2D::new(pos))
            .with(RigidBody::new_static())
            .with(Collider::box_collider(w, h).with_friction(0.0).with_restitution(1.0))
            .id()
    };
    let top_y = WIN_H / 2.0 - WALL_THICKNESS / 2.0;
    let side_x = WIN_W / 2.0 - WALL_THICKNESS / 2.0;
    spawn_wall(&mut world, Vec2::new(0.0, top_y), WIN_W, WALL_THICKNESS);
    spawn_wall(&mut world, Vec2::new(-side_x, 0.0), WALL_THICKNESS, WIN_H);
    spawn_wall(&mut world, Vec2::new(side_x, 0.0), WALL_THICKNESS, WIN_H);
    // Bottom wall stands in for the paddle so the rally never ends.
    spawn_wall(&mut world, Vec2::new(0.0, -top_y), WIN_W, WALL_THICKNESS);

    let mut bricks = crate::spawning::spawn_bricks(&mut world, 0);

    // Ball exactly as spawn_ball builds it.
    let ball = world.spawn()
        .with(Transform2D::new(Vec2::new(0.0, PADDLE_Y + SERVE_OFFSET_Y)))
        .with(RigidBody::new_dynamic()
            .with_gravity_scale(0.0)
            .with_rotation_locked(true)
            .with_linear_damping(0.0)
            .with_angular_damping(0.0)
            .with_ccd(true))
        .with(Collider::circle_collider(BALL_RADIUS).with_friction(0.0).with_restitution(1.0))
        .id();
    let launch = Vec2::new(0.15f32.sin(), 0.15f32.cos()) * BALL_SPEED;
    physics.set_velocity(ball, launch, 0.0);

    let mut destroyed = 0usize;
    let mut ploughs = 0usize;

    for _frame in 0..3600 {
        physics.update(&mut world, 1.0 / 60.0);
        let collisions: Vec<CollisionData> = physics.collision_events().to_vec();

        // maintain_all_ball_velocities equivalent
        if let Some((vel, _)) = physics.get_body_velocity(ball) {
            let speed = vel.length();
            if speed >= 1.0 {
                let new_vel = enforce_min_vertical(vel / speed) * BALL_SPEED;
                if (new_vel - vel).length() > 1.0 {
                    physics.set_velocity(ball, new_vel, 0.0);
                }
            }
        }

        // check_brick_hits equivalent (destroy + brick_bounce_velocity)
        bricks.retain(|brick| {
            let hit = collisions.iter()
                .any(|c| c.event.started && c.event.involves(ball, brick.entity));
            if hit {
                destroyed += 1;
                let brick_pos = world.get::<Transform2D>(brick.entity)
                    .map(|t| t.position)
                    .expect("brick has a transform");
                if let (Some(ball_pos), Some((vel, _))) = (
                    world.get::<Transform2D>(ball).map(|t| t.position),
                    physics.get_body_velocity(ball),
                ) {
                    let new_vel = brick_bounce_velocity(ball_pos, vel, brick_pos);
                    if new_vel != vel {
                        physics.set_velocity(ball, new_vel, 0.0);
                    }
                    // The bug: ball hit the brick from below and is
                    // STILL climbing afterwards → it ploughed through.
                    let was_below = ball_pos.y < brick_pos.y - BRICK_H / 2.0;
                    if was_below && new_vel.y > 1.0 {
                        ploughs += 1;
                    }
                }
                physics.destroy_entity(&mut world, brick.entity);
            }
            !hit
        });

        if bricks.is_empty() {
            break;
        }
    }

    assert!(destroyed > 5, "expected a real rally, only {destroyed} bricks destroyed");
    assert_eq!(
        ploughs, 0,
        "ball kept climbing after {ploughs} of {destroyed} brick hits — ploughed through the grid"
    );
}

#[test]
fn min_vertical_leaves_steep_directions_alone() {
    let dir = Vec2::new(0.6, 0.8);
    assert_eq!(enforce_min_vertical(dir), dir);
}

#[test]
fn min_vertical_reaims_shallow_directions_preserving_signs() {
    let fixed = enforce_min_vertical(Vec2::new(-0.999, -0.04));
    assert!(fixed.x < 0.0);
    assert!((fixed.y + MIN_VERTICAL_FRACTION).abs() < 0.0001);
    assert!((fixed.length() - 1.0).abs() < 0.0001, "must stay unit length");
}

#[test]
fn min_vertical_pushes_pure_horizontal_upward() {
    let fixed = enforce_min_vertical(Vec2::new(1.0, 0.0));
    assert!(fixed.y > 0.0);
    assert!((fixed.length() - 1.0).abs() < 0.0001);
}
