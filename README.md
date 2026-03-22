# My Side-Scroller

A 2D side-scrolling platformer built with [Bevy 0.18](https://bevyengine.org/) (Rust game engine). Features procedural infinite level generation, hand-designed LDtk level chunks, a GPU compute shader background with day/night cycle, and WASM deployment support.

## How to Play

### Controls
| Key / Button | Action |
|--------------|--------|
| A / Left Arrow / Left Stick | Move left |
| D / Right Arrow / Left Stick | Move right |
| Space / A Button (Gamepad) | Jump (press again mid-air for double jump) |
| Space (tap vs hold) | Short hop vs full jump (variable height) |
| Escape / Start Button | Pause / Unpause |
| Enter / Space / A Button | Select menu item / Restart after game over |
| Up/Down Arrows / D-Pad | Navigate menus |
| Left/Right Arrows / D-Pad | Adjust settings |
| ~ (Backtick) | Toggle debug overlay |

### Debug Controls (in-game)
| Key | Action |
|-----|--------|
| 0 | Toggle god mode (invincibility) |
| 9 | Refill health to max |

### Objective
Navigate through procedurally generated platforms, collect coins, stomp enemies, and avoid hazards. Try to rack up the highest score before losing all your health!

## Features

### Core Gameplay
- **Procedural infinite generation** — platforms, enemies, hazards, and collectibles spawn ahead of the player and despawn behind
- **Hand-designed LDtk chunks** — level editor integration stitches authored sections into the procedural stream
- **Difficulty scaling** — enemy frequency, hazard density, platform gaps, shooter fire rate, and projectile speed all scale with score
- **Double jump** with variable height — tap for short hop, hold for full jump
- **Coyote time** — brief grace period to jump after walking off a platform edge
- **Combo system** — consecutive stomps and block destructions multiply score (up to 16x)
- **Coin value scaling** — coins worth more at higher difficulty (10pts at easy, 50pts at hard)

### Enemies
- **Walking enemies** — patrol back and forth on platforms, stompable from above
- **Flying enemies** — hover on sine waves above platforms, stompable
- **Shooter enemies** — fire projectiles toward the player (range-gated, fire rate scales with difficulty)
- **Charging enemies** — detect the player and rush toward them after a wind-up
- **Flying ranged enemies** — aerial enemies that fire downward projectiles

### Hazards
- **Spikes** — static damage zones on platforms
- **Spinning saws** — patrol platforms, always damage (no stomp)
- **Lava pools** — fill ground gaps, instant kill on contact
- **Falling boulders** — drop from above with warning indicators
- **Timed traps** — spike traps that activate on a cycle with visual warnings

### Collectibles & Power-ups
- **Coins** — bob and spin above platforms, 5 formation patterns (arc, line, stack, diagonal, diamond)
- **Speed Boost** (blue) — +50% movement speed for 5 seconds
- **Triple Jump** (green) — 3 jumps instead of 2 for 8 seconds
- **Shield** (gold) — absorbs one hit of damage

### Breakable Platforms
- **Destructible blocks** — rows of blocks that take damage from stomps
- **Run wear** — blocks gradually degrade and change color as the player runs across them
- **Combo chain** — block destruction contributes to the score combo multiplier

### Visual Effects
- **GPU compute shader background** — WGSL shader renders parallax rolling hills with:
  - Day/night cycle (120s): midnight, dawn, noon, dusk transitions
  - Terrain detail: snow caps, rock outcrops, water in valleys, wildflowers
  - Atmospheric effects: drifting clouds, fog wisps, shooting stars, god rays
  - Fireflies (night only), tree silhouettes with wind sway
- **Sprite art** — player, enemies, collectibles, and hazards use PNG sprites with atlas animations
- **Particle effects** — jump dust, landing dust, running dust, debris explosions
- **Screen shake** — camera trauma on damage with quadratic falloff
- **Death animation** — faint sprite sheet plays before Game Over with fade-out
- **Invincibility flash** — player flickers during post-damage protection

### Audio
- **Sound effects** — jump, land, coin collect, damage, death, enemy stomp, projectile fire, power-up pickup
- **Background music** — looping chiptune track during gameplay
- **Volume controls** — master, SFX, and music sliders in settings

### Progression
- **Score-based difficulty** — 0% to 100% scaling based on score (caps at 5000)
- **Level sections** — themed color shifts with banner transitions (every 1000 points)
- **Checkpoints** — flag markers that save mid-run progress
- **High score persistence** — saves best score across sessions
- **Game over resets** — checkpoint data cleared on death, new games always start fresh

### Technical
- **Save/Load system** — JSON persistence for settings, high scores, and checkpoint data
- **WASM deployment** — builds for itch.io with canvas targeting and localStorage saves
- **Gamepad support** — full keyboard + controller input via unified abstraction layer
- **Screen resolution options** — 1280x720, 1920x1080, 2560x1440 with fullscreen toggle
- **Debug overlay** — FPS, entity counts, player state, HP, powerups, combo, difficulty, generation frontier

## Project Structure

```
src/
  main.rs          — App entry point, plugin registration
  constants.rs     — All tuning values (physics, sizes, speeds, spawn rates)
  state.rs         — Game state machine (Menu/Playing/Paused/Settings/GameOver)
  player.rs        — Player components, input, physics, AABB collision
  input.rs         — Unified keyboard + gamepad input abstraction
  animation.rs     — Sprite sheet animation system (idle, run, jump, fall, death)
  camera.rs        — Smooth camera follow with lerp + screen shake + hit freeze
  level.rs         — Procedural platform generation with difficulty scaling
  mountain_bg.rs   — GPU compute shader background plugin
  ldtk_chunks.rs   — LDtk level editor chunk parsing and spawning
  health.rs        — Health, damage messages, invincibility, death animation
  hud.rs           — In-game HUD, menus, and overlay screens
  enemies.rs       — All enemy types with AI, shooting, charging, and combos
  collectibles.rs  — Coin bobbing, spinning, collection, and value scaling
  hazards.rs       — Spikes, saws, lava, boulders, and timed traps
  breakable.rs     — Destructible block platforms with run wear
  powerups.rs      — Speed boost, triple jump, and shield pickups
  audio.rs         — Sound effect loading, playback, and game settings
  particles.rs     — Particle effects (jump dust, landing dust, running dust)
  highscore.rs     — High score tracking and checkpoint clear on game over
  checkpoint.rs    — Checkpoint flags, section transitions, and sky color
  save.rs          — JSON save/load system (native + WASM localStorage)
  sprites.rs       — Sprite and atlas asset loading
  transition.rs    — Screen fade transitions
  debug.rs         — Debug overlay, god mode, and cheat controls

assets/
  shaders/         — WGSL compute shader for mountain background
  sprites/         — PNG sprites (player, enemies, hazards, collectibles, HUD)
  audio/           — .ogg sound effects and background music
  levels/          — LDtk project file with hand-designed level chunks
```

## Building & Running

Requires [Rust](https://rustup.rs/) (edition 2021).

```bash
cargo run
```

### WASM Build (for itch.io)

```bash
trunk build --release --public-url ./
```

## Tech Stack

- **Engine**: Bevy 0.18 (ECS architecture)
- **Language**: Rust (edition 2021)
- **Background**: WGSL compute shader via wgpu
- **Level Design**: LDtk (JSON-parsed, no heavy crate dependency)
- **Rendering**: Sprite-based 2D with texture atlas animations
- **Physics**: Custom AABB collision (separated horizontal/vertical passes)
- **Audio**: Bevy AudioPlayer with .ogg format
- **Persistence**: serde/serde_json (native file + WASM localStorage)
- **Input**: Keyboard + gamepad via unified GameInput abstraction
- **WASM**: trunk build with web-sys for browser storage
