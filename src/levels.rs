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
use crate::types::{Brick, PickupKind};

/// A selectable level: display name, scene file, and the chaos mode it
/// plays in. The layouts live in the scene files (editor-authorable); this
/// table is deliberately a Rust const — breakout has no serde/ron deps.
pub(crate) struct LevelDef {
    pub(crate) title: &'static str,
    pub(crate) scene_file: &'static str,
    pub(crate) mode: ChaosMode,
}

/// The level roster shown on the level-select screen, one per chaos mode.
pub(crate) const LEVELS: [LevelDef; 4] = [
    LevelDef { title: "CLASSIC", scene_file: "level1.scene.ron", mode: ChaosMode::Normal },
    LevelDef { title: "THE VAULT", scene_file: "level2.scene.ron", mode: ChaosMode::Insane },
    LevelDef { title: "PINATA", scene_file: "level3.scene.ron", mode: ChaosMode::Ridiculous },
    LevelDef { title: "THE GAUNTLET", scene_file: "level4.scene.ron", mode: ChaosMode::Insiculous },
];

/// One-line flavor text per level, shown under the roster.
pub(crate) fn level_hint(index: usize) -> &'static str {
    match index {
        0 => "The classic wall. A gentle taste of armor.",
        1 => "A fortress of steel. Ball speeds up per paddle hit.",
        2 => "Crack it open - it rains power-ups. Two-ball serves.",
        3 => "Armor, chaos, and everything at once.",
        _ => "",
    }
}

/// Absolute path of a level scene file.
///
/// `SceneLoader::load_from_file` takes raw filesystem paths (it does not go
/// through `GameConfig.asset_base_path`), so the path is anchored explicitly.
pub(crate) fn level_scene_path(scene_file: &str) -> PathBuf {
    engine_core::game_root!().join("assets/scenes").join(scene_file)
}

/// Parse a level's scene from disk. Returns `None` (with a console warning)
/// if the file is missing or malformed — the game then uses the generated
/// grid instead of failing to start.
pub(crate) fn load_level_data(index: usize) -> Option<SceneData> {
    let Some(def) = LEVELS.get(index) else {
        eprintln!("breakout: level index {index} out of range; using generated brick grid");
        return None;
    };
    let path = level_scene_path(def.scene_file);
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

/// What a brick's `EntityTag` says about it. Untagged bricks get the
/// defaults: one hit, no drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BrickSpec {
    pub(crate) hits: u32,
    pub(crate) drop: Option<PickupKind>,
}

impl Default for BrickSpec {
    fn default() -> Self {
        Self { hits: 1, drop: None }
    }
}

/// Parse a brick's `EntityTag` string: tokens joined by `+`.
///
/// - `armored{N}` — N total hits to destroy (2..=9)
/// - `drop_multiball` / `drop_wrecking` / `drop_insiculous` — pickup dropped
///
/// Unknown or malformed tokens warn and are skipped (graceful degradation,
/// same philosophy as `brick_value_from_name`); duplicates: last wins.
pub(crate) fn parse_brick_tag(tag: &str) -> BrickSpec {
    let mut spec = BrickSpec::default();
    for token in tag.split('+') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(n) = token.strip_prefix("armored") {
            match n.parse::<u32>() {
                Ok(hits) if (2..=9).contains(&hits) => spec.hits = hits,
                _ => eprintln!("breakout: ignoring malformed armor token '{token}'"),
            }
        } else {
            match token {
                "drop_multiball" => spec.drop = Some(PickupKind::Multiball),
                "drop_wrecking" => spec.drop = Some(PickupKind::Wrecking),
                "drop_insiculous" => spec.drop = Some(PickupKind::Insiculous),
                _ => eprintln!("breakout: ignoring unknown brick tag token '{token}'"),
            }
        }
    }
    spec
}

/// Build the game's `Brick` bookkeeping from a scene instance's named
/// entities: every entity named `brick*` becomes a brick. The particle-burst
/// color is read from the entity's live `Sprite`, so bricks retinted in the
/// editor keep matching effects; armor/drop behavior comes from the
/// entity's `EntityTag` (see `parse_brick_tag`).
pub(crate) fn bricks_from_names(
    named_entities: &HashMap<String, EntityId>,
    world: &World,
) -> Vec<Brick> {
    named_entities
        .iter()
        .filter(|(name, _)| name.starts_with("brick"))
        .map(|(name, &entity)| {
            let spec = world
                .get::<EntityTag>(entity)
                .map(|t| parse_brick_tag(&t.0))
                .unwrap_or_default();
            Brick {
                entity,
                value: brick_value_from_name(name),
                color: world
                    .get::<Sprite>(entity)
                    .map(|s| s.color)
                    .unwrap_or(Vec4::ONE),
                hits_left: spec.hits,
                drop: spec.drop,
            }
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
    use crate::constants::{BRICK_COLS, BRICK_GAP, BRICK_H, BRICK_W, PADDLE_Y, PLAYFIELD_HALF_W};
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

    /// Load a roster level via a CARGO_MANIFEST_DIR-anchored path (tests
    /// can't rely on exe-dir anchoring).
    fn load_roster_level(index: usize) -> SceneData {
        let def = &LEVELS[index];
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/scenes")
            .join(def.scene_file);
        SceneLoader::load_from_file(&path)
            .unwrap_or_else(|e| panic!("{} should parse: {e}", def.scene_file))
    }

    #[test]
    fn shipped_level_parses() {
        let scene = load_scene();
        assert_eq!(scene.name, "Breakout Level 1");
    }

    #[test]
    fn every_roster_level_parses_with_valid_bricks() {
        assert_eq!(LEVELS.len(), ChaosMode::ALL.len(), "one level per chaos mode");
        for (i, def) in LEVELS.iter().enumerate() {
            assert_eq!(def.mode, ChaosMode::ALL[i], "roster order follows ChaosMode::ALL");
            assert!(!level_hint(i).is_empty());

            let scene = load_roster_level(i);
            let brick_names: Vec<&str> = scene
                .entities
                .iter()
                .filter_map(|e| e.name.as_deref())
                .filter(|n| n.starts_with("brick"))
                .collect();
            assert!(!brick_names.is_empty(), "{} has no bricks", def.scene_file);
            for name in &brick_names {
                brick_row_from_name(name)
                    .unwrap_or_else(|| panic!("unparsable brick name {name} in {}", def.scene_file));
            }
        }
    }

    #[test]
    fn every_roster_level_fits_the_playfield() {
        for (i, def) in LEVELS.iter().enumerate() {
            let scene = load_roster_level(i);
            for entity in &scene.entities {
                let name = entity.name.as_deref().unwrap_or("<unnamed>");
                let (pos, scale) = merged_components(&scene, entity)
                    .into_iter()
                    .find_map(|c| match c {
                        ComponentData::Transform2D { position, scale, .. } => {
                            Some((position, scale))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{name} in {} has no Transform2D", def.scene_file));

                let half_w = scale.0 * RENDER_UNIT / 2.0;
                assert!(
                    pos.0.abs() + half_w < PLAYFIELD_HALF_W,
                    "{name} in {} pokes past a side wall",
                    def.scene_file
                );
                let bottom = pos.1 - scale.1 * RENDER_UNIT / 2.0;
                assert!(
                    bottom > PADDLE_Y + 100.0,
                    "{name} in {} sits too close to the paddle",
                    def.scene_file
                );
            }
        }
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
            let components = merged_components(&scene, entity);
            let emissive = components
                .iter()
                .find_map(|c| match c {
                    ComponentData::Sprite { emissive, .. } => Some(*emissive),
                    _ => None,
                })
                .expect("brick has a sprite");
            let tagged = components
                .iter()
                .any(|c| matches!(c, ComponentData::EntityTag { .. }));
            if tagged {
                // Special bricks style themselves (armor dims, drops glow).
                assert!(emissive > 0.0);
            } else {
                assert_eq!(emissive, 0.9);
            }
        }
    }

    #[test]
    fn parse_brick_tag_grammar_table() {
        let d = BrickSpec::default();
        assert_eq!(d, BrickSpec { hits: 1, drop: None });

        assert_eq!(parse_brick_tag("armored2"), BrickSpec { hits: 2, drop: None });
        assert_eq!(parse_brick_tag("armored9"), BrickSpec { hits: 9, drop: None });
        assert_eq!(
            parse_brick_tag("drop_multiball"),
            BrickSpec { hits: 1, drop: Some(PickupKind::Multiball) }
        );
        assert_eq!(
            parse_brick_tag("drop_wrecking"),
            BrickSpec { hits: 1, drop: Some(PickupKind::Wrecking) }
        );
        assert_eq!(
            parse_brick_tag("drop_insiculous"),
            BrickSpec { hits: 1, drop: Some(PickupKind::Insiculous) }
        );
        assert_eq!(
            parse_brick_tag("armored2+drop_wrecking"),
            BrickSpec { hits: 2, drop: Some(PickupKind::Wrecking) }
        );
        // Token order doesn't matter; whitespace tolerated
        assert_eq!(
            parse_brick_tag(" drop_wrecking + armored3 "),
            BrickSpec { hits: 3, drop: Some(PickupKind::Wrecking) }
        );
        // Duplicates: last wins
        assert_eq!(
            parse_brick_tag("armored2+armored3"),
            BrickSpec { hits: 3, drop: None }
        );
        // Unknown/malformed tokens degrade to defaults, never panic
        assert_eq!(parse_brick_tag(""), d);
        assert_eq!(parse_brick_tag("bogus"), d);
        assert_eq!(parse_brick_tag("armored1"), d, "1-hit armor is not armor");
        assert_eq!(parse_brick_tag("armored99"), d, "out-of-range armor rejected");
        assert_eq!(parse_brick_tag("armoredX"), d);
        assert_eq!(
            parse_brick_tag("bogus+drop_multiball"),
            BrickSpec { hits: 1, drop: Some(PickupKind::Multiball) },
            "unknown tokens are skipped, not fatal"
        );
    }

    /// Every EntityTag authored in a roster level must parse to a meaningful
    /// spec — a tag that parses to the plain-brick default is a typo.
    #[test]
    fn every_roster_level_tag_is_meaningful() {
        let mut tagged_total = 0;
        for (i, def) in LEVELS.iter().enumerate() {
            let scene = load_roster_level(i);
            for entity in &scene.entities {
                let name = entity.name.as_deref().unwrap_or("<unnamed>");
                for c in merged_components(&scene, entity) {
                    if let ComponentData::EntityTag { tag } = c {
                        assert_ne!(
                            parse_brick_tag(&tag),
                            BrickSpec::default(),
                            "tag '{tag}' on {name} in {} means nothing",
                            def.scene_file
                        );
                        tagged_total += 1;
                    }
                }
            }
        }
        assert!(tagged_total > 20, "expected plenty of special bricks, got {tagged_total}");
    }

    /// Levels 3 and 4 must actually rain power-ups (drop-brick counts > 0).
    #[test]
    fn prize_levels_have_drop_bricks() {
        for i in [2usize, 3] {
            let scene = load_roster_level(i);
            let drops = scene
                .entities
                .iter()
                .flat_map(|e| merged_components(&scene, e))
                .filter_map(|c| match c {
                    ComponentData::EntityTag { tag } => parse_brick_tag(&tag).drop,
                    _ => None,
                })
                .count();
            assert!(drops >= 4, "{} has only {drops} drop bricks", LEVELS[i].scene_file);
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
        // Untagged brick gets the plain defaults
        assert_eq!(bricks[0].hits_left, 1);
        assert_eq!(bricks[0].drop, None);
    }

    #[test]
    fn bricks_from_names_reads_entity_tags() {
        let mut world = World::new();
        let mut named = HashMap::new();

        let brick = world.create_entity();
        world.add_component(&brick, Sprite::new(0)).ok();
        world
            .add_component(&brick, EntityTag::new("armored3+drop_insiculous"))
            .ok();
        named.insert("brick_r1_c1".to_string(), brick);

        let bricks = bricks_from_names(&named, &world);
        assert_eq!(bricks.len(), 1);
        assert_eq!(bricks[0].hits_left, 3);
        assert_eq!(bricks[0].drop, Some(PickupKind::Insiculous));
    }
}
