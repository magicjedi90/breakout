use engine_core::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    ChaosSelect { selection: u8 },
    Achievements,
    /// Ball rests on the paddle waiting for launch.
    Serving,
    Playing,
    GameOver { won: bool },
}

/// A live brick: its entity plus the score it pays out when destroyed.
pub(crate) struct Brick {
    pub(crate) entity: EntityId,
    pub(crate) value: u32,
    pub(crate) color: Vec4,
}

pub(crate) struct BreakoutGame {
    pub(crate) physics: PhysicsSystem,

    pub(crate) paddle: Option<EntityId>,
    pub(crate) ball: Option<EntityId>,
    pub(crate) extra_balls: Vec<EntityId>,
    pub(crate) bricks: Vec<Brick>,
    /// Parsed level scene (brick layout); re-instantiated every match.
    /// `None` means the scene file was missing — use the generated grid.
    pub(crate) level: Option<SceneData>,
    pub(crate) walls: Vec<EntityId>,
    pub(crate) bottom_sensor: Option<EntityId>,
    pub(crate) background: Option<EntityId>,
    /// White 1x1 texture for paddle, bricks, walls, background, particles.
    pub(crate) tex_id: u32,
    /// PNG texture for the circular ball sprite.
    pub(crate) ball_tex_id: u32,

    pub(crate) score: u32,
    pub(crate) lives: u32,
    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Global ball speed multiplier. Insane mode grows it on every paddle
    /// hit; reset on life loss and at match start.
    pub(crate) speed_mult: f32,
    /// Bricks destroyed since the ball last touched the paddle.
    pub(crate) combo: u32,

    /// Deforming spring-mass grid drawn under the gameplay sprites.
    pub(crate) grid: Option<GridMesh>,
    /// F1 toggles magenta collider outlines over the sprites.
    pub(crate) debug_colliders: bool,
}

impl Default for BreakoutGame {
    fn default() -> Self {
        Self {
            physics: PhysicsSystem::with_config(PhysicsConfig::top_down()),
            paddle: None,
            ball: None,
            extra_balls: Vec::new(),
            bricks: Vec::new(),
            level: None,
            walls: Vec::new(),
            bottom_sensor: None,
            background: None,
            tex_id: 0,
            ball_tex_id: 0,
            score: 0,
            lives: crate::constants::STARTING_LIVES,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            speed_mult: 1.0,
            combo: 0,
            grid: None,
            debug_colliders: false,
        }
    }
}
