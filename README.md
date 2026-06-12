# Insiculous Breakout

Neon Breakout built on the [insiculous_2d](../../insiculous_2d) engine — a
rainbow brick wall, bloom-heavy Geometry-Wars look, a spring-mass-deforming
grid background, achievements, and the engine's signature chaos modes.

## Running

The game depends on the engine by path (`../../insiculous_2d`), so keep both
checkouts side by side:

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's visual editor
```

Assets and saves resolve relative to the executable (falling back to the crate
directory), so `cargo run` works from any working directory. Achievements
persist to `saves/breakout_achievements.json`.

## How to Play

Clear all 60 bricks. You have 3 balls; one is lost each time every live ball
falls past the paddle. Top rows pay more points than bottom rows, and where
the ball strikes the paddle controls the bounce — center returns it straight
up, edges deflect it up to 60°.

| Input | Action |
|-------|--------|
| `←`/`→` or `A`/`D` | Move paddle |
| Mouse | Move paddle (takes over when the mouse moves) |
| `Space` / `Enter` / Left click | Launch ball |
| `Escape` | Back to title |
| `F1` | Toggle collider debug overlay |

**Menus** — `W`/`S` or `↑`/`↓` to navigate, `Enter`/`Space` to confirm,
`Escape` to go back.

## Chaos Modes

Pick one before each match:

| Mode | Effect |
|------|--------|
| Normal | Classic Breakout |
| Insane | Ball speeds up on every paddle hit |
| Ridiculous | Every serve launches two balls |
| Insiculous | Both at once |

## What This Game Demonstrates

The second entry in the engine's 20-games challenge (after Pong). New
patterns exercised here:

- **Dynamic entity despawning from collision events** — bricks are static
  bodies destroyed the frame a ball touches them, with score/combo payout.
- **Grid layout spawning** — the 10×6 brick wall is generated from
  constants, with per-row colors and point values.
- **Lives system** — bottom-of-screen sensor + escape safety net spends a
  life when the last live ball is gone.
- **Offset-based paddle control** — gameplay overrides the physical
  reflection so the player aims the ball by where it lands on the paddle.
- **Mouse + keyboard input on one control** — whichever moved last wins.

## Editor Mode

`cargo run --features editor` opens the exact same game inside the engine's
scene editor — hierarchy, inspector with undo/redo, play/pause/stop
(`F5`/`Ctrl+P`/`Ctrl+Shift+P`), and the collider outline overlay (`C`). All
gameplay tuning constants live in `src/constants.rs`; entities are spawned
from those values in `src/spawning.rs`.

## Project Layout

```
src/
├── main.rs        # Game trait impl, window/config setup, editor wiring
├── constants.rs   # All gameplay tuning values (sizes, speeds, layout)
├── types.rs       # BreakoutGame state, GameState, Brick
├── spawning.rs    # Entity creation (paddle, ball, walls, bricks, sensor)
├── gameplay.rs    # Match update loop, paddle control, bricks, lives
├── menu.rs        # Title / chaos / achievements screens
├── effects.rs     # Deforming grid background, particle bursts
├── chaos_theme.rs # Per-chaos-mode color themes
├── achievements.rs# Achievement definitions
└── drawing.rs     # UI/text drawing helpers
```
