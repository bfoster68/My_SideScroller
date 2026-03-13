# My Side-Scroller

A 2D side-scrolling platformer built with [Bevy 0.18](https://bevyengine.org/) (Rust game engine).

## How to Play

### Controls
| Key | Action |
|-----|--------|
| A / Left Arrow | Move left |
| D / Right Arrow | Move right |
| Space | Jump (press again mid-air for double jump) |
| Space (tap vs hold) | Short hop vs full jump (variable height) |
| S / Down Arrow | Slide (while running on the ground) |
| Escape | Pause / Unpause |
| Enter / Space | Start game / Restart after game over |
| R | Regenerate level |

### Objective
Navigate through procedurally generated platforms, collect coins, stomp enemies, and avoid hazards. Try to rack up the highest score before losing all your health!

## Features

### Sprint 1 — Foundation
- **Modular plugin architecture** — 14 separate modules (player, animation, camera, level, health, hud, state, audio, parallax, particles, enemies, collectibles, hazards, constants)
- **Game state machine** — Menu, Playing, Paused, and Game Over screens with transitions
- **Double jump** — Two jumps before needing to land
- **Variable jump height** — Tap Space for a short hop, hold for a full jump
- **Coyote time** — Brief grace period to jump after walking off a platform edge
- **Invincibility frames** — Flashing sprite after taking damage (1.5s protection)
- **HUD** — Displays health (red), score (white), and coins (gold)
- **Overlay screens** — Title screen, pause overlay, and game over screen

### Sprint 2 — Visuals & Audio
- **Animated player sprite** — Sprite sheet animations for idle (6 frames), run (6 frames), jump (7 frames), and fall states
- **Sprite flipping** — Character faces the direction of movement
- **Styled platforms** — Colored platforms with top highlights, bottom shadows, and edge shading
- **3-layer parallax background** — Stars/sky (far), hills (mid), foliage (near) scrolling at different speeds
- **Audio infrastructure** — Sound effect system with .ogg loading (jump, land, collect, hit)

### Sprint 3 — Enemies, Hazards & Collectibles
- **Enemies** — Red rectangles that patrol back and forth on platforms
  - Stomp from above to kill (+100 score, player bounces up)
  - Side/bottom contact deals 1 damage with knockback
- **Coins** — Gold squares that bob up and down and spin
  - Floating above platforms and mid-air between platforms
  - Collect on touch (+1 coin, +10 score, plays collect SFX)
- **Spikes** — Dark red hazards sitting on platforms
  - Contact deals 1 damage with upward knockback (respects invincibility)
- **Level population** — Procedural placement: 25% enemies, 15% spikes, 35% coins, 25% bare platforms

### Sprint 4 — Polish & Game Feel
- **Particle effects** — Dust bursts on jump and landing, trailing dust while running
- **Sound effects** — Placeholder .ogg SFX for jump, land, collect, and hit
- **Screen shake** — Camera shakes on taking damage (large) and stomping enemies (small), with intensity decay
- **Knockback** — Player is pushed away from enemies on contact and launched upward from spikes
- **Hurt animation** — 3-frame hurt sprite sheet plays on non-fatal damage (0.3s lock)
- **Death animation** — 7-frame faint sprite sheet plays before Game Over transition (0.7s)
- **Slide mechanic** — Press Down/S while running to slide forward at high speed (0.4s duration, 7-frame animation)

## Project Structure

```
src/
  main.rs          - App entry point, plugin registration
  constants.rs     - All tuning values (physics, sizes, speeds, spawn rates)
  state.rs         - Game state machine (Menu/Playing/Paused/GameOver)
  player.rs        - Player components, input, physics, slide, AABB collision
  animation.rs     - Sprite sheet animation system (idle, run, jump, fall, hurt, faint, slide)
  camera.rs        - Smooth camera follow with screen shake
  level.rs         - Procedural platform generation with entity spawning
  health.rs        - Health, damage/death events, invincibility, hurt/death timers
  hud.rs           - In-game HUD and overlay screens
  parallax.rs      - Multi-layer parallax background
  particles.rs     - Dust particle effects (jump, land, running)
  enemies.rs       - Enemy patrol AI, stomp/damage collision, knockback
  collectibles.rs  - Coin bobbing, spinning, collection, and collect SFX
  hazards.rs       - Spike damage and knockback on contact
  audio.rs         - Sound effect loading and playback

assets/
  sprites/         - PNG sprite sheets (idle, run, jump, hurt, faint, slide)
  audio/           - Placeholder .ogg sound effects (jump, land, collect, hit)
  *.gif            - Source GIF animations from itch.io
```

## Building & Running

Requires [Rust](https://rustup.rs/) (edition 2021).

```bash
cargo run
```

## What's Left to Do

### Sprint 5 — Level Design & Progression
- Checkpoints and level sections
- Increasing difficulty as the player progresses
- Moving platforms
- End-of-level goals or transitions

### Sprint 6 — Menus & UX
- Main menu with options
- Settings (volume, controls)
- High score tracking and persistence
- Screen transitions and fade effects

### Sprint 7 — Content & Variety
- Background music
- Enemy and collectible sprite art (currently colored shapes)
- More enemy types (flying, ranged)
- More hazard types (moving saws, lava)
- Power-ups (speed boost, extra jump, shield)

## Tech Stack

- **Engine**: Bevy 0.18
- **Language**: Rust (edition 2021)
- **Rendering**: Sprite-based 2D with texture atlas animations
- **Physics**: Custom AABB collision (separated horizontal/vertical passes)
- **Audio**: Bevy AudioPlayer with .ogg format
