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
| Escape | Pause / Unpause |
| Enter / Space | Start game / Restart after game over |
| R | Regenerate level |

### Objective
Navigate through procedurally generated platforms, collect coins, stomp enemies, and avoid hazards. Try to rack up the highest score before losing all your health!

## Features

### Sprint 1 — Foundation
- **Modular plugin architecture** — 16 separate modules (player, animation, camera, level, health, hud, state, audio, parallax, enemies, collectibles, hazards, particles, powerups, highscore, transition)
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
- **Audio infrastructure** — Sound effect system ready for .ogg files (jump, land, collect, hit, music)

### Sprint 3 — Enemies, Hazards & Collectibles
- **Enemies** — Red rectangles that patrol back and forth on platforms
  - Stomp from above to kill (+100 score, player bounces up)
  - Side/bottom contact deals 1 damage
- **Coins** — Gold squares that bob up and down and spin
  - Floating above platforms and mid-air between platforms
  - Collect on touch (+1 coin, +10 score)
- **Spikes** — Dark red hazards sitting on platforms
  - Contact deals 1 damage (respects invincibility)
- **Level population** — Procedural placement: 25% enemies, 15% spikes, 35% coins, 25% bare platforms

### Sprint 4 — Level Design & Progression
- **Difficulty scaling** — Enemy chance, spike chance, ground gaps, and platform gaps scale with score
- **Moving platforms** — Platforms that oscillate vertically; occupants (enemies, spikes, coins) ride along as children
- **Ground gaps** — Procedural gaps in the ground that increase with difficulty

### Sprint 5 — Menus & UX
- **High score persistence** — Saves and displays best score across sessions
- **Screen transitions** — Fade-in/fade-out between game states
- **Smart respawn** — Instant respawn on safe platforms (avoids hazards), grants invincibility
- **Coin formations** — 5 Sonic-style patterns (arc, line, stack, diagonal, diamond) between platforms

### Sprint 6 — Polish
- **Particle effects** — Jump dust, landing dust, and running dust trails
- **Screen shake** — Camera trauma on damage with quadratic falloff
- **Death animation** — Faint sprite sheet plays before Game Over transition with fade-out
- **Flying enemies** — Purple enemies that oscillate on sine waves above platforms (stompable)
- **Shooter enemies** — Green enemies that fire homing projectiles at the player
- **Moving saws** — Spinning grey hazards that patrol platforms (always damages, no stomp)
- **Lava pools** — Orange-red pulsing hazards in ground gaps (instant kill)
- **Power-ups** — Random pickups that bob above platforms:
  - Speed Boost (blue) — +50% movement speed for 5 seconds
  - Triple Jump (green) — 3 jumps instead of 2 for 8 seconds
  - Shield (gold) — Absorbs one hit of damage

## Project Structure

```
src/
  main.rs          — App entry point, plugin registration
  constants.rs     — All tuning values (physics, sizes, speeds, spawn rates)
  state.rs         — Game state machine (Menu/Playing/Paused/GameOver)
  player.rs        — Player components, input, physics, AABB collision
  animation.rs     — Sprite sheet animation system (idle, run, jump, fall, death)
  camera.rs        — Smooth camera follow with lerp + screen shake
  level.rs         — Procedural platform generation with entity spawning
  health.rs        — Health, damage messages, invincibility, death animation
  hud.rs           — In-game HUD and overlay screens
  parallax.rs      — Multi-layer parallax background
  enemies.rs       — Walking, flying, and shooter enemies with AI and collision
  collectibles.rs  — Coin bobbing, spinning, and collection
  hazards.rs       — Spikes, moving saws, and lava pools
  powerups.rs      — Speed boost, triple jump, and shield pickups
  audio.rs         — Sound effect loading and playback
  particles.rs     — Particle effects (jump dust, landing dust, running dust)
  highscore.rs     — High score persistence
  transition.rs    — Screen fade transitions

assets/
  sprites/         — PNG sprite sheets (idle, run, jump, faint, hurt, slide)
  *.gif            — Source GIF animations from itch.io
```

## Building & Running

Requires [Rust](https://rustup.rs/) (edition 2021).

```bash
cargo run
```

## What's Left to Do

### Sprint 7 — Art & Audio
- Sound effects (.ogg files for jump, land, collect, hit, background music)
- Enemy and collectible sprite art (currently colored shapes)
- Power-up visual feedback (tinted sprites, outline effects)
- Lava and saw animated sprites

### Sprint 8 — Advanced Features
- Checkpoints and level sections
- End-of-level goals or transitions
- Main menu with options
- Settings (volume, controls)
- More enemy types (charging, flying ranged)
- More hazard types (falling boulders, timed traps)

## Tech Stack

- **Engine**: Bevy 0.18
- **Language**: Rust (edition 2021)
- **Rendering**: Sprite-based 2D with texture atlas animations
- **Physics**: Custom AABB collision (separated horizontal/vertical passes)
- **Audio**: Bevy AudioPlayer with .ogg format
