use engine_core::prelude::*;

pub(crate) const WIN_W: f32 = 800.0;
pub(crate) const WIN_H: f32 = 600.0;


pub(crate) const WALL_THICKNESS: f32 = 20.0;
/// Inner edge of the side walls — the playfield half-width.
pub(crate) const PLAYFIELD_HALF_W: f32 = WIN_W / 2.0 - WALL_THICKNESS;

pub(crate) const PADDLE_W: f32 = 110.0;
pub(crate) const PADDLE_H: f32 = 16.0;
pub(crate) const PADDLE_SCALE: Vec2 = Vec2::new(PADDLE_W / RENDER_UNIT, PADDLE_H / RENDER_UNIT);
pub(crate) const PADDLE_Y: f32 = -260.0;
/// Player 2's paddle in co-op guards the top edge, mirroring the bottom one.
pub(crate) const PADDLE_TOP_Y: f32 = -PADDLE_Y;
pub(crate) const PADDLE_MAX_X: f32 = PLAYFIELD_HALF_W - PADDLE_W / 2.0;
pub(crate) const PADDLE_SPEED: f32 = 520.0;
/// Maximum bounce deflection off the paddle, in radians from straight up.
/// Hitting the paddle dead center returns the ball vertically; the very
/// edge sends it out at this angle.
pub(crate) const PADDLE_MAX_BOUNCE_ANGLE: f32 = std::f32::consts::FRAC_PI_3; // 60 degrees

pub(crate) const BALL_SIZE: f32 = 16.0;
pub(crate) const BALL_SCALE: f32 = BALL_SIZE / RENDER_UNIT;
pub(crate) const BALL_RADIUS: f32 = BALL_SIZE / 2.0;
pub(crate) const BALL_SPEED: f32 = 360.0;
pub(crate) const BALL_MAX_SPEED: f32 = 760.0;
/// Minimum fraction of the ball's speed that must be vertical. Prevents the
/// ball ping-ponging horizontally between the side walls forever.
pub(crate) const MIN_VERTICAL_FRACTION: f32 = 0.25;
/// Insane mode: ball speed multiplier gained on every paddle hit.
pub(crate) const INSANE_SPEED_GAIN: f32 = 1.15;

/// Resting offset of a served ball above the paddle center.
pub(crate) const SERVE_OFFSET_Y: f32 = PADDLE_H / 2.0 + BALL_RADIUS + 2.0;
/// Full width of the random launch-angle spread, in radians. A served ball
/// leaves within ±half this off vertical.
pub(crate) const LAUNCH_ANGLE_SPREAD: f32 = 0.6;

pub(crate) const BRICK_COLS: usize = 10;
pub(crate) const BRICK_ROWS: usize = 6;
pub(crate) const BRICK_W: f32 = 70.0;
pub(crate) const BRICK_H: f32 = 24.0;
pub(crate) const BRICK_GAP: f32 = 4.0;
/// Y position of the center of the top brick row.
pub(crate) const BRICK_TOP_Y: f32 = 240.0;
/// Co-op: top brick row of the middle band. Six rows at the standard pitch
/// span +70..-70, clear of both paddles' reaction zones.
pub(crate) const BRICK_TOP_Y_2P: f32 = 70.0;
/// Points awarded per brick = (rows from the bottom of the grid) * this.
pub(crate) const BRICK_VALUE_STEP: u32 = 10;
/// Battle-damage dimming applied to each color channel of an armored brick
/// on every non-fatal hit.
pub(crate) const BRICK_DAMAGE_COLOR_FACTOR: f32 = 0.65;
/// Battle-damage dimming applied to an armored brick's glow on every
/// non-fatal hit.
pub(crate) const BRICK_DAMAGE_EMISSIVE_FACTOR: f32 = 0.5;

// Falling power-up pickups dropped by special bricks.
pub(crate) const PICKUP_SIZE: f32 = 18.0;
pub(crate) const PICKUP_FALL_SPEED: f32 = 180.0;
/// Wrecking-ball effect length; catching another pickup refreshes it.
pub(crate) const WRECKING_DURATION: f32 = 10.0;
/// Cap on simultaneous extra balls (multiball grants beyond it fizzle).
pub(crate) const MAX_EXTRA_BALLS: usize = 6;

/// Normal ball glow (spawn + wrecking-revert use the same value).
pub(crate) const BALL_EMISSIVE: f32 = 2.5;
/// Red-hot look while the wrecking ball is active.
pub(crate) const WRECKING_BALL_COLOR: Vec4 = Vec4::new(1.0, 0.45, 0.3, 1.0);
pub(crate) const WRECKING_BALL_EMISSIVE: f32 = 3.5;

// Pickup capsule tints (mirror the brick colors so players learn the map).
pub(crate) const MULTIBALL_PICKUP_COLOR: Vec4 = Vec4::new(0.35, 0.9, 1.0, 1.0);
pub(crate) const WRECKING_PICKUP_COLOR: Vec4 = Vec4::new(1.0, 0.45, 0.2, 1.0);
pub(crate) const INSICULOUS_PICKUP_COLOR: Vec4 = Vec4::new(0.6, 1.0, 0.35, 1.0);

pub(crate) const STARTING_LIVES: u32 = 3;
/// Bricks destroyed in one volley (without touching the paddle) to unlock
/// the combo achievement.
pub(crate) const COMBO_TARGET: u32 = 5;

/// Extra margin past the window edge before an off-screen ball counts as
/// lost (safety net for CCD misses / NaN positions).
pub(crate) const BALL_LOST_BOUNDS_PAD: f32 = 60.0;

// Radial impulses kicked into the spring-mass background grid, one strength
// and radius per event that disturbs it.
pub(crate) const GRID_IMPULSE_PADDLE_HIT_STRENGTH: f32 = 200.0;
pub(crate) const GRID_IMPULSE_PADDLE_HIT_RADIUS: f32 = 70.0;
pub(crate) const GRID_IMPULSE_BRICK_DESTROY_STRENGTH: f32 = 260.0;
pub(crate) const GRID_IMPULSE_BRICK_DESTROY_RADIUS: f32 = 90.0;
pub(crate) const GRID_IMPULSE_BALL_LOST_STRENGTH: f32 = 700.0;
pub(crate) const GRID_IMPULSE_BALL_LOST_RADIUS: f32 = 160.0;

pub(crate) const PADDLE_COLOR: Vec4 = Vec4::new(0.4, 0.85, 1.0, 1.0);
/// Player 2's top paddle in co-op — warm pink against P1's cool cyan.
pub(crate) const PADDLE2_COLOR: Vec4 = Vec4::new(1.0, 0.5, 0.75, 1.0);
/// Classic rainbow rows, top to bottom.
pub(crate) const BRICK_ROW_COLORS: [Vec4; BRICK_ROWS] = [
    Vec4::new(1.0, 0.30, 0.30, 1.0), // red
    Vec4::new(1.0, 0.60, 0.20, 1.0), // orange
    Vec4::new(1.0, 0.90, 0.25, 1.0), // yellow
    Vec4::new(0.35, 0.95, 0.40, 1.0), // green
    Vec4::new(0.30, 0.55, 1.0, 1.0), // blue
    Vec4::new(0.75, 0.40, 1.0, 1.0), // purple
];
