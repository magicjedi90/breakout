use engine_core::prelude::*;
use crate::chaos_theme::ChaosTheme;
use crate::constants::*;
use crate::spawning;
use crate::types::*;

fn menu_navigate(current: u8, count: u8, up: bool, down: bool) -> u8 {
    if up {
        if current == 0 { count - 1 } else { current - 1 }
    } else if down {
        (current + 1) % count
    } else {
        current
    }
}

fn nav_keys(ctx: &GameContext) -> (bool, bool, bool, bool) {
    let up = ctx.input.is_key_just_pressed(KeyCode::ArrowUp)
        || ctx.input.is_key_just_pressed(KeyCode::KeyW);
    let down = ctx.input.is_key_just_pressed(KeyCode::ArrowDown)
        || ctx.input.is_key_just_pressed(KeyCode::KeyS);
    let confirm = ctx.input.is_key_just_pressed(KeyCode::Space)
        || ctx.input.is_key_just_pressed(KeyCode::Enter);
    let back = ctx.input.is_key_just_pressed(KeyCode::Escape);
    (up, down, confirm, back)
}

impl BreakoutGame {
    pub(crate) fn update_title_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let (up, down, confirm, _) = nav_keys(ctx);
        let selection = menu_navigate(selection, 2, up, down);
        self.state = GameState::TitleScreen { selection };

        if confirm {
            match selection {
                0 => self.state = GameState::ChaosSelect { selection: 0 },
                _ => self.state = GameState::Achievements,
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        if ctx.input.is_key_just_pressed(KeyCode::Escape)
            || ctx.input.is_key_just_pressed(KeyCode::Space)
            || ctx.input.is_key_just_pressed(KeyCode::Enter)
        {
            self.state = GameState::TitleScreen { selection: 1 };
        }
    }

    pub(crate) fn update_chaos_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let (up, down, confirm, back) = nav_keys(ctx);
        let count = ChaosMode::ALL.len() as u8;
        let selection = menu_navigate(selection, count, up, down);
        self.state = GameState::ChaosSelect { selection };

        if back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if confirm {
            self.chaos_mode = ChaosMode::ALL[selection as usize];
            // Mirror the runtime selection into the engine context so any
            // code reading ctx.chaos_mode agrees with self.chaos_mode.
            ctx.chaos_mode = self.chaos_mode;
            self.start_game(ctx.world);
        }
    }

    /// Reset score/lives, rebuild the brick grid, and put a fresh ball on
    /// the paddle.
    pub(crate) fn start_game(&mut self, world: &mut World) {
        self.score = 0;
        self.lives = STARTING_LIVES;
        self.speed_mult = 1.0;
        self.combo = 0;

        self.destroy_all_balls(world);
        for brick in self.bricks.drain(..) {
            self.physics.destroy_entity(world, brick.entity);
        }
        self.bricks = spawning::spawn_bricks(world, self.tex_id);

        let ball = self.spawn_ball(world);
        self.ball = Some(ball);

        self.apply_theme(world);
        if let Some(paddle) = self.paddle {
            self.physics.set_kinematic_target(paddle, Vec2::new(0.0, PADDLE_Y), 0.0);
        }
        self.state = GameState::Serving;
    }

    /// Push the current `chaos_mode`'s look onto the live entities:
    /// background tint, wall color, ball color, and grid color.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(bg) = self.background {
            if let Some(s) = world.get_mut::<Sprite>(bg) { s.color = theme.bg_color; }
        }
        for &w in &self.walls {
            if let Some(s) = world.get_mut::<Sprite>(w) { s.color = theme.wall_color; }
        }
        for ball in self.ball.into_iter().chain(self.extra_balls.iter().copied()) {
            if let Some(s) = world.get_mut::<Sprite>(ball) { s.color = theme.ball_color; }
        }
        self.grid = Some(crate::effects::build_grid(&theme));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_navigate_wraps_both_directions() {
        assert_eq!(menu_navigate(0, 3, true, false), 2);
        assert_eq!(menu_navigate(2, 3, false, true), 0);
    }

    #[test]
    fn menu_navigate_holds_position_without_input() {
        assert_eq!(menu_navigate(1, 3, false, false), 1);
    }
}
