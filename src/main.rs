mod achievements;
mod chaos_theme;
mod constants;
mod drawing;
mod effects;
mod gameplay;
#[cfg(test)]
mod gameplay_tests;
mod levels;
mod menu;
mod power_ups;
mod spawning;
mod types;

use engine_core::prelude::*;
use chaos_theme::theme_for;
use constants::*;
use spawning::*;
use types::*;

impl Game for BreakoutGame {
    fn init(&mut self, ctx: &mut GameContext) {
        let font_path = engine_core::game_root!().join("assets/fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        achievements::register_all(ctx.achievements);

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.tex_id = tex.id;
        // Relative paths resolve against the asset base path set in main().
        self.ball_tex_id = ctx.assets.load_texture("ball_8px.png")
            .expect("missing assets/ball_8px.png").id;

        let theme = theme_for(self.chaos_mode);
        self.background = Some(spawn_background(
            ctx.world, tex.id, theme.bg_color, Vec2::new(WIN_W, WIN_H)));

        self.paddle = Some(spawn_paddle(ctx.world, tex.id));

        // Top wall plus the two side walls; the bottom stays open over the
        // life-loss sensor.
        let top_y = WIN_H / 2.0 - WALL_THICKNESS / 2.0;
        let side_x = WIN_W / 2.0 - WALL_THICKNESS / 2.0;
        self.walls.push(spawn_wall(ctx.world, Vec2::new(0.0, top_y), WIN_W, WALL_THICKNESS, tex.id, theme.structure_color));
        self.walls.push(spawn_wall(ctx.world, Vec2::new(-side_x, 0.0), WALL_THICKNESS, WIN_H, tex.id, theme.structure_color));
        self.walls.push(spawn_wall(ctx.world, Vec2::new(side_x, 0.0), WALL_THICKNESS, WIN_H, tex.id, theme.structure_color));

        self.bottom_sensor = Some(spawn_bottom_sensor(ctx.world));

        // Brick layouts are authored in per-level scenes (editor-editable);
        // start_game() loads the selected level each match, falling back to
        // the generated grid if the file is missing.

        // Bricks and ball spawn fresh on every `start_game()`. Build the
        // deforming grid backdrop now so it exists before the first match.
        self.grid = Some(default_playfield_grid(&theme));
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frame_count = self.frame_count.wrapping_add(1);

        match self.state.clone() {
            GameState::TitleScreen { selection } => self.update_title_input(ctx, selection),
            GameState::LevelSelect { selection } => self.update_level_select_input(ctx, selection),
            GameState::Achievements => self.update_achievements_input(ctx),
            _ => self.update_gameplay(ctx),
        }

        self.update_entity_visibility(ctx);
        self.draw_ui(ctx);
    }
}

fn main() {
    // Anchor assets and saves to the game's directory so launching from any
    // working directory behaves the same.
    let root = engine_core::game_root!();
    let config = GameConfig::new("Insiculous Breakout")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        .with_asset_base_path(root.join("assets").to_string_lossy())
        .with_achievement_save_path(root.join("saves/breakout_achievements.json").to_string_lossy());

    // With `--features editor` the game runs inside the scene editor
    // (hierarchy, inspector, gizmos, play/pause/stop, collider overlay);
    // without it the game runs bare. Same game code either way.
    #[cfg(feature = "editor")]
    editor_integration::run_game_with_editor(BreakoutGame::default(), config).unwrap();
    #[cfg(not(feature = "editor"))]
    run_game(BreakoutGame::default(), config).unwrap();
}
