//! Scene-driven brick layouts.
//!
//! The brick grid is authored in `assets/scenes/level1.scene.ron` (editable
//! in the engine editor) and instantiated at every match start. Bricks are
//! identified by the `brick_r{row}_c{col}` naming convention; the row digit
//! drives the score payout, which stays a game rule in Rust
//! (`spawning::brick_value`). If the scene is missing or yields no bricks,
//! the caller falls back to the generated grid in `spawning::spawn_bricks`.

use std::collections::HashMap;
use std::path::PathBuf;

use engine_core::prelude::*;

use crate::constants::{BRICK_ROWS, BRICK_VALUE_STEP};
use crate::spawning::brick_value;
use crate::types::Brick;

/// Directory that holds the game's `assets/` and `saves/` folders.
///
/// Prefers the executable's directory (shipped layout: assets next to the
/// binary), falling back to the crate directory so `cargo run` works from
/// any current working directory.
pub(crate) fn game_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("assets").is_dir() {
                return dir.to_path_buf();
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Absolute path of the level scene file.
///
/// `SceneLoader::load_from_file` takes raw filesystem paths (it does not go
/// through `GameConfig.asset_base_path`), so the path is anchored explicitly.
pub(crate) fn level_scene_path() -> PathBuf {
    game_root().join("assets/scenes/level1.scene.ron")
}

/// Parse the level scene from disk. Returns `None` (with a console warning)
/// if the file is missing or malformed — the game then uses the generated
/// grid instead of failing to start.
pub(crate) fn load_level_data() -> Option<SceneData> {
    let path = level_scene_path();
    match SceneLoader::load_from_file(&path) {
        Ok(data) => Some(data),
        Err(e) => {
            eprintln!(
                "breakout: could not load level scene {}: {e}; using generated brick grid",
                path.display()
            );
            None
        }
    }
}

/// Extract the row index from a `brick_r{row}_c{col}` entity name.
pub(crate) fn brick_row_from_name(name: &str) -> Option<usize> {
    let rest = name.strip_prefix("brick_r")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Score paid out by a brick with the given entity name. Bricks whose name
/// doesn't yield an in-range row still score the minimum payout, so renamed
/// bricks degrade gracefully instead of breaking the game.
fn brick_value_from_name(name: &str) -> u32 {
    match brick_row_from_name(name) {
        Some(row) if row < BRICK_ROWS => brick_value(row),
        _ => BRICK_VALUE_STEP,
    }
}

/// Build the game's `Brick` bookkeeping from a scene instance's named
/// entities: every entity named `brick*` becomes a brick. The particle-burst
/// color is read from the entity's live `Sprite`, so bricks retinted in the
/// editor keep matching effects.
pub(crate) fn bricks_from_names(
    named_entities: &HashMap<String, EntityId>,
    world: &World,
) -> Vec<Brick> {
    named_entities
        .iter()
        .filter(|(name, _)| name.starts_with("brick"))
        .map(|(name, &entity)| Brick {
            entity,
            value: brick_value_from_name(name),
            color: world
                .get::<Sprite>(entity)
                .map(|s| s.color)
                .unwrap_or(Vec4::ONE),
        })
        .collect()
}

/// Instantiate the level scene into the world and return the brick list.
pub(crate) fn spawn_bricks_from_scene(
    data: &SceneData,
    world: &mut World,
    assets: &mut AssetManager,
) -> Result<Vec<Brick>, SceneLoadError> {
    let instance = SceneLoader::instantiate(data, world, assets)?;
    Ok(bricks_from_names(&instance.named_entities, world))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{BRICK_COLS, BRICK_GAP, BRICK_H, BRICK_W, RENDER_UNIT};
    use crate::spawning::{brick_x, brick_y};
    use engine_core::prelude::ComponentData;

    fn manifest_scene_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/scenes/level1.scene.ron")
    }

    fn load_scene() -> SceneData {
        SceneLoader::load_from_file(manifest_scene_path()).expect("level1.scene.ron should parse")
    }

    /// Effective (merged prefab + override) components of an entity.
    fn merged_components(scene: &SceneData, entity: &EntityData) -> Vec<ComponentData> {
        let mut result: Vec<ComponentData> = entity
            .prefab
            .as_ref()
            .and_then(|p| scene.prefabs.get(p))
            .map(|p| p.components.clone())
            .unwrap_or_default();
        for over in &entity.overrides {
            let kind = std::mem::discriminant(over);
            if let Some(pos) = result.iter().position(|c| std::mem::discriminant(c) == kind) {
                result[pos] = over.clone();
            } else {
                result.push(over.clone());
            }
        }
        result
    }

    #[test]
    fn shipped_level_parses() {
        let scene = load_scene();
        assert_eq!(scene.name, "Breakout Level 1");
    }

    #[test]
    fn shipped_level_has_full_brick_grid() {
        let scene = load_scene();
        let brick_names: Vec<&str> = scene
            .entities
            .iter()
            .filter_map(|e| e.name.as_deref())
            .filter(|n| n.starts_with("brick"))
            .collect();
        assert_eq!(brick_names.len(), BRICK_ROWS * BRICK_COLS);
        for name in &brick_names {
            let row = brick_row_from_name(name).expect("brick name should parse");
            assert!(row < BRICK_ROWS, "row out of range in {name}");
        }
    }

    #[test]
    fn shipped_level_positions_match_generated_grid() {
        let scene = load_scene();
        for entity in &scene.entities {
            let name = entity.name.as_deref().expect("all level entities are named");
            let row = brick_row_from_name(name).expect("brick name should parse");
            let col: usize = name
                .split("_c")
                .nth(1)
                .and_then(|s| s.parse().ok())
                .expect("brick name has a column");

            let transform = merged_components(&scene, entity)
                .into_iter()
                .find_map(|c| match c {
                    ComponentData::Transform2D { position, scale, .. } => Some((position, scale)),
                    _ => None,
                })
                .expect("brick has a Transform2D");

            assert_eq!(transform.0, (brick_x(col), brick_y(row)), "position of {name}");
            assert_eq!(
                transform.1,
                (BRICK_W / RENDER_UNIT, BRICK_H / RENDER_UNIT),
                "scale of {name}"
            );
        }
    }

    /// Guards the sprite/collider size footgun: physics ignores
    /// Transform2D.scale, so collider half-extents must stay in sync with
    /// the sprite's scale x RENDER_UNIT size.
    #[test]
    fn shipped_level_colliders_match_brick_dimensions() {
        let scene = load_scene();
        for entity in &scene.entities {
            let name = entity.name.as_deref().expect("all level entities are named");
            let half_extents = merged_components(&scene, entity)
                .into_iter()
                .find_map(|c| match c {
                    ComponentData::Collider {
                        shape: engine_core::scene_data::ColliderShapeData::Box { half_extents },
                        ..
                    } => Some(half_extents),
                    _ => None,
                })
                .expect("brick has a box collider");
            assert_eq!(half_extents, (BRICK_W / 2.0, BRICK_H / 2.0), "collider of {name}");
        }
    }

    #[test]
    fn shipped_level_bricks_glow_and_fit_playfield() {
        let scene = load_scene();
        // Emissive must survive the schema (bricks lose their neon look
        // silently otherwise), and the grid must stay inside the walls.
        let total_width = BRICK_COLS as f32 * BRICK_W + (BRICK_COLS as f32 - 1.0) * BRICK_GAP;
        assert!(total_width < crate::constants::WIN_W - 2.0 * crate::constants::WALL_THICKNESS);
        for entity in &scene.entities {
            let emissive = merged_components(&scene, entity)
                .into_iter()
                .find_map(|c| match c {
                    ComponentData::Sprite { emissive, .. } => Some(emissive),
                    _ => None,
                })
                .expect("brick has a sprite");
            assert_eq!(emissive, 0.9);
        }
    }

    #[test]
    fn brick_row_from_name_parses_valid_and_rejects_invalid() {
        assert_eq!(brick_row_from_name("brick_r0_c0"), Some(0));
        assert_eq!(brick_row_from_name("brick_r5_c9"), Some(5));
        assert_eq!(brick_row_from_name("brick_r12_c3"), Some(12));
        assert_eq!(brick_row_from_name("brick_rX_c0"), None);
        assert_eq!(brick_row_from_name("brick"), None);
        assert_eq!(brick_row_from_name("paddle"), None);
    }

    #[test]
    fn brick_value_from_name_maps_rows_and_defaults_minimum() {
        assert_eq!(brick_value_from_name("brick_r0_c0"), brick_value(0));
        assert_eq!(brick_value_from_name("brick_r5_c9"), brick_value(5));
        // Renamed or out-of-range bricks score the minimum instead of panicking
        assert_eq!(brick_value_from_name("brick_r99_c0"), BRICK_VALUE_STEP);
        assert_eq!(brick_value_from_name("brick_custom"), BRICK_VALUE_STEP);
    }

    #[test]
    fn bricks_from_names_builds_bookkeeping_from_world() {
        let mut world = World::new();
        let mut named = HashMap::new();

        let red = Vec4::new(1.0, 0.3, 0.3, 1.0);
        let brick = world.create_entity();
        world.add_component(&brick, Sprite::new(0).with_color(red)).ok();
        named.insert("brick_r0_c0".to_string(), brick);

        // Non-brick entities are ignored
        let paddle = world.create_entity();
        named.insert("paddle".to_string(), paddle);

        let bricks = bricks_from_names(&named, &world);
        assert_eq!(bricks.len(), 1);
        assert_eq!(bricks[0].entity, brick);
        assert_eq!(bricks[0].value, brick_value(0));
        assert_eq!(bricks[0].color, red);
    }
}
