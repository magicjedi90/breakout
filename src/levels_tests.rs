//! Tests for the scene-driven level rosters (solo and co-op).
//!
//! Lives outside `levels.rs` so the roster tables and the test battery can
//! both grow without crowding the 600-line file budget.

use std::collections::HashMap;
use std::path::PathBuf;

use engine_core::prelude::*;

use crate::constants::{
    BRICK_COLS, BRICK_GAP, BRICK_H, BRICK_ROWS, BRICK_VALUE_STEP, BRICK_W, PADDLE_TOP_Y, PADDLE_Y,
    PLAYFIELD_HALF_W,
};
use crate::levels::*;
use crate::spawning::{brick_value, brick_x, brick_y};
use crate::types::{GameMode, PickupKind};

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

/// Load a roster level via a CARGO_MANIFEST_DIR-anchored path (tests can't
/// rely on exe-dir anchoring).
fn load_roster_level(def: &LevelDef) -> SceneData {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/scenes")
        .join(def.scene_file);
    SceneLoader::load_from_file(&path)
        .unwrap_or_else(|e| panic!("{} should parse: {e}", def.scene_file))
}

const BOTH_MODES: [GameMode; 2] = [GameMode::SinglePlayer, GameMode::TwoPlayerCoop];

#[test]
fn shipped_level_parses() {
    let scene = load_scene();
    assert_eq!(scene.name, "Breakout Level 1");
}

#[test]
fn every_roster_level_parses_with_valid_bricks() {
    for mode in BOTH_MODES {
        let levels = roster(mode);
        assert_eq!(levels.len(), ChaosMode::ALL.len(), "one level per chaos mode");
        for (i, def) in levels.iter().enumerate() {
            assert_eq!(def.mode, ChaosMode::ALL[i], "roster order follows ChaosMode::ALL");
            assert!(!level_hint(mode, i).is_empty());

            let scene = load_roster_level(def);
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
}

/// Solo levels stay above the bottom paddle; co-op levels must ALSO stay
/// below the top paddle (the both-sides analogue of the solo guarantee).
#[test]
fn every_roster_level_fits_the_playfield() {
    for mode in BOTH_MODES {
        for def in roster(mode) {
            let scene = load_roster_level(def);
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
                    "{name} in {} sits too close to the bottom paddle",
                    def.scene_file
                );
                if mode == GameMode::TwoPlayerCoop {
                    let top = pos.1 + scale.1 * RENDER_UNIT / 2.0;
                    assert!(
                        top < PADDLE_TOP_Y - 100.0,
                        "{name} in {} sits too close to the top paddle",
                        def.scene_file
                    );
                }
            }
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
/// Transform2D.scale, so collider half-extents must stay in sync with the
/// sprite's scale x RENDER_UNIT size — in EVERY roster level, both modes.
#[test]
fn every_level_colliders_match_brick_dimensions() {
    for mode in BOTH_MODES {
        for def in roster(mode) {
            let scene = load_roster_level(def);
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
                    .unwrap_or_else(|| panic!("{name} in {} has no box collider", def.scene_file));
                assert_eq!(
                    half_extents,
                    (BRICK_W / 2.0, BRICK_H / 2.0),
                    "collider of {name} in {}",
                    def.scene_file
                );
            }
        }
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
    for mode in BOTH_MODES {
        let mut tagged_total = 0;
        for def in roster(mode) {
            let scene = load_roster_level(def);
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
        assert!(
            tagged_total > 20,
            "expected plenty of special bricks in the {mode:?} roster, got {tagged_total}"
        );
    }
}

/// Prize levels (index 2 and 3) must actually rain power-ups, in both
/// rosters.
#[test]
fn prize_levels_have_drop_bricks() {
    for mode in BOTH_MODES {
        for i in [2usize, 3] {
            let def = &roster(mode)[i];
            let scene = load_roster_level(def);
            let drops = scene
                .entities
                .iter()
                .flat_map(|e| merged_components(&scene, e))
                .filter_map(|c| match c {
                    ComponentData::EntityTag { tag } => parse_brick_tag(&tag).drop,
                    _ => None,
                })
                .count();
            assert!(drops >= 4, "{} has only {drops} drop bricks", def.scene_file);
        }
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
