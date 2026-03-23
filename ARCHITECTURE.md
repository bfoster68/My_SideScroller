# Game Architecture Document

## Overview

A 2D side-scrolling platformer built with **Bevy 0.18** (Rust). The game features procedural infinite level generation with optional hand-designed LDtk chunks, multiple enemy types, collectibles, power-ups, hazards, a checkpoint/section system, and a GPU compute shader background.

**Total codebase**: ~7,570 lines across 25 modules.

---

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Engine | Bevy 0.18 (ECS) |
| Language | Rust (2021 edition) |
| Background | WGSL compute shader via wgpu |
| Level Design | LDtk (JSON-parsed, no heavy crate) |
| Audio | Bevy AudioPlayer (OGG format) |
| Serialization | serde + serde_json |
| RNG | rand 0.8 |
| WASM Target | trunk build, web-sys for storage |

---

## State Machine

```
   MainMenu
      |  \
      v   v
   Playing  SaveMenu (Load)
      |  \
      v   v
   Paused  GameOver
     |  \       |
     v   v      v
  Settings SaveMenu (Save)  MainMenu
```

- `GameState` enum: `Menu`, `Playing`, `Paused`, `GameOver`, `Settings`, `SaveMenu`
- `SaveMenuMode` resource: `Load` or `Save` — determines save menu behavior
- `PreviousGameState` resource tracks the prior state to distinguish pause/unpause from fresh game start
- `OnEnter(Playing)` systems have guards to skip level reset when returning from Paused/Settings
- `ScreenTransition` resource manages fade-in/fade-out between states
- `CheckpointResetSet` system set ensures checkpoint data resets before level generation

---

## Plugin Registration Order (main.rs)

```
 1. InputPlugin          (PreUpdate: reads keyboard/gamepad -> GameInput resource)
 2. SavePlugin           (Startup: loads saved settings, high scores)
 3. StatePlugin          (Update: state transitions, menu/pause UI)
 4. CameraPlugin         (Update: camera follow, screen shake, hit freeze)
 5. PlayerPlugin         (Update: movement, physics, collision, respawn)
 6. LdtkChunksPlugin     (Startup: loads LDtk project, builds ChunkPool)
 7. LevelPlugin          (Update: procedural generation, difficulty scaling)
 8. BreakablePlugin      (Update: destructible block mechanics)
 9. HealthPlugin         (Update: damage application, death handling)
10. HudPlugin            (OnEnter/OnExit: score, health, powerup bars)
11. AnimationPlugin       (Update: sprite sheet cycling, state-based anims)
12. MountainBgPlugin     (Update: GPU shader background, camera tracking)
13. ParticlesPlugin      (Update: dust, debris, burst effects)
14. EnemiesPlugin        (Update: AI, patrol, shooting, stomping, combos)
15. CollectiblesPlugin   (Update: coin animation, collection)
16. HazardsPlugin        (Update: spikes, saws, lava, boulders, traps)
17. PowerupsPlugin       (Update: speed boost, triple jump, shield)
18. SpritesPlugin        (Startup: loads all sprite assets -> GameSprites)
19. GameAudioPlugin      (OnEnter Playing + Update: SFX, music)
20. HighScorePlugin      (OnEnter GameOver: save/display high score)
21. TransitionPlugin     (Update: screen fade effects)
22. CheckpointPlugin     (Update: section tracking, banners, colors)
23. DebugPlugin          (Update: debug overlay, god mode, cheats)
```

---

## System Execution Order (Playing State)

### PreUpdate
- `read_input` -> `GameInput` resource

### Update (chained within each plugin)

**Player systems** (`PlayerMovementSet`):
1. `player_input` (reads GameInput, applies movement/jump)
2. `apply_gravity` (gravity on velocity, skips during death)
3. `apply_velocity` (position update + platform collision)
4. `respawn_on_fall` (teleport to safe platform if falling)

**Camera** (after player):
1. `manage_hit_freeze` (time scale management)
2. `damage_triggers_shake` (trauma from damage events)
3. `try_snap_camera` (one-shot snap on play start)
4. `camera_follow` (smooth lerp + shake offset)

**Level generation** (after `PlayerMovementSet`):
1. `update_difficulty` (score -> difficulty 0.0-1.0)
2. `generate_chunks` (ground, platforms, enemies, coins)
3. `despawn_behind_camera` (cleanup)

**Enemy systems** (after `PlayerMovementSet`):
1. `enemy_patrol` (walking AI)
2. `flying_enemy_movement` (sine wave hover)
3. `charging_enemy_update` (detect, charge, recover)
4. `shooter_fire` (range-gated, difficulty-scaled)
5. `flying_ranged_fire` (downward projectiles)
6. `move_projectiles` (velocity + lifetime)
7. `enemy_player_collision` (stomp/damage)
8. `projectile_player_collision` (projectile hits)
9. `update_score_popups` (floating score text)
10. `reset_combo_on_land` (combo tracker)

**Hazards** (after `PlayerMovementSet`):
1. `spike_player_collision`
2. `saw_player_collision` + `saw_animate`
3. `lava_player_collision` + `lava_animate`
4. `timed_trap_cycle` + `timed_trap_collision`
5. `boulder_spawner_tick` + `boulder_fall` + `boulder_collision`

**Health** (after collisions):
1. `apply_damage` (reads DamageEvent, applies knockback/invincibility/death)

---

## Key Resources

| Resource | Module | Purpose |
|----------|--------|---------|
| `GameInput` | input.rs | Unified keyboard/gamepad input state |
| `Score` | player.rs | Current score (u32) |
| `Coins` | player.rs | Coin count |
| `Difficulty` | level.rs | 0.0-1.0 from score, drives all scaling |
| `ChunkTracker` | level.rs | Tracks generation frontier positions |
| `ChunkPool` | ldtk_chunks.rs | Parsed LDtk level templates |
| `ComboTracker` | enemies.rs | Stomp combo count + display timer |
| `ScreenShake` | camera.rs | Trauma value for shake |
| `HitFreeze` | camera.rs | Virtual time slowdown for impacts |
| `GameSettings` | audio.rs | Volume, resolution, fullscreen |
| `HighScore` | highscore.rs | Persistent best score |
| `CheckpointData` | checkpoint.rs | Section, score, position at last checkpoint |
| `ActiveSlot` | save.rs | Currently played save slot ID |
| `SaveMenuMode` | state.rs | Load or Save mode for save menu |
| `SaveMenuSelection` | state.rs | Selected slot index + delete confirmation |
| `AudioHandles` | audio.rs | Loaded audio asset handles |
| `GameSprites` | sprites.rs | All sprite/atlas handles |
| `SpriteSheets` | animation.rs | Player animation atlas data |
| `MountainParams` | mountain_bg.rs | GPU uniform: camera pos + time |
| `PreviousGameState` | state.rs | Distinguishes pause from fresh start |
| `ScreenTransition` | transition.rs | Fade-in/out state |
| `SectionBanner` | checkpoint.rs | Milestone display timer |

---

## Key Components

### Player
| Component | Purpose |
|-----------|---------|
| `Player` | Marker |
| `Velocity(Vec2)` | Physics velocity |
| `Grounded { on_ground, coyote_timer }` | Ground detection |
| `JumpCounter { jumps_remaining }` | Double/triple jump tracking |
| `JumpHeld(bool)` | Variable jump height |
| `FacingDirection` | Sprite flip |
| `PlayerAnimState` | Current animation state |
| `Health { current, max }` | Hit points |
| `DeathTimer` | Death animation countdown |
| `Invincible` | Post-damage invulnerability timer |
| `Knockback(Vec2)` | Impact pushback |
| `Shield` | Absorbs hits |
| `SpeedBoost` | Temporary speed multiplier |
| `TripleJump` | Extra jump count |

### Platforms
| Component | Purpose |
|-----------|---------|
| `Platform` | Marker for collision |
| `PlatformSize(Vec2)` | Collision dimensions |
| `MovingPlatform { base_y, speed, range }` | Oscillation data |
| `PlatformVelocity(Vec2)` | Per-frame delta for carrying player |
| `BreakableBlock { health, max_health, wear }` | Destructible (wear degrades while running) |

### Enemies
| Component | Purpose |
|-----------|---------|
| `Enemy` | Shared marker for all enemy types |
| `PatrolRange { left_bound, right_bound }` | Walking AI bounds |
| `FlyingEnemy { base_y, amplitude, frequency }` | Hover data |
| `ShooterEnemy` | Marker for shooters |
| `FlyingRangedEnemy` | Marker for flying ranged |
| `ChargingEnemy` | Marker + charge state machine |
| `ShootTimer` | Fire interval timer |
| `Projectile { velocity, lifetime }` | Fired projectile |

### Hazards
| Component | Purpose |
|-----------|---------|
| `Spike` | Static damage zone |
| `Saw` | Animated spinning hazard |
| `Lava` | Instant kill in ground gaps |
| `LavaHitbox { size, y_offset }` | Extended collision zone |
| `TimedTrap` | Cycling on/off spike |
| `BoulderSpawner` | Periodic falling boulder source |
| `FallingBoulder` | Active falling hazard |

### Collectibles & Powerups
| Component | Purpose |
|-----------|---------|
| `Coin` | Collectible marker |
| `CoinBob { base_y, phase }` | Bobbing animation |
| `PowerupPickup(PowerupKind)` | Uncollected powerup |
| `PowerupKind` | Speed, TripleJump, or Shield |

---

## Coordinate System

- **Y-up**: Bevy standard (positive Y = up)
- **Camera**: Orthographic, `FixedVertical { viewport_height: 720.0 }`
- **Window**: 1280x720, WASM canvas-aware
- **Ground level**: `GROUND_Y = -300.0`
- **Player spawn**: `(-400.0, -200.0)`
- **Fall limit**: `Y < -420.0` triggers respawn
- **Generation**: extends `GENERATE_AHEAD = 2400.0` units right of camera
- **Cleanup**: despawns `DESPAWN_BEHIND = 1200.0` units left of camera

---

## Difficulty System

Score-based scaling from 0.0 to 1.0:
```
difficulty = (score / DIFFICULTY_SCORE_MAX).clamp(0.0, 1.0)
```
Where `DIFFICULTY_SCORE_MAX = 5000`.

**Scales with difficulty:**
- Platform gap distances (wider)
- Ground gap frequency (more holes)
- Enemy spawn chance (more enemies)
- Spike/saw spawn chance (more hazards)
- Moving platform frequency (fewer safe platforms)
- Shooter fire rate (2.5s→1.2s), projectile speed (160→280px/s), range-gated (600px)
- Charging enemy detect range (220px), wind-up time (0.45s)
- Coin value (10pts at d=0 → 50pts at d=1)
- Powerup spawn chance (5%→12%)
- LDtk chunk selection (filtered by difficulty range)

**Platform reachability:** Max upward rise scales from 100% of `MAX_JUMP_HEIGHT` at small gaps to 60% at large gaps. Every 8th platform forced near ground level as a safety net.

---

## GPU Background (mountain_bg.rs + mountain_bg.wgsl)

A compute shader renders a fullscreen texture every frame:
- 4 parallax hill layers with procedural noise terrain
- Day/night cycle (120s): midnight -> dawn -> noon -> dusk
- Terrain detail: snow caps, rock outcrops, water, wildflowers, trees
- Atmospheric: clouds, fog wisps, shooting stars, fireflies, god rays
- Camera X position fed as uniform for parallax scrolling
- Background sprite follows camera at z=-50

---

## LDtk Integration (ldtk_chunks.rs)

Hand-designed level chunks parsed from `assets/levels/chunks.ldtk`:
- Raw JSON parsing with serde_json (no bevy_ecs_ldtk dependency)
- `ChunkPool` resource contains `ChunkTemplate` entries
- Each template stores collision tiles, entities, entry/exit Y, difficulty range
- `select_chunk()` filters by difficulty and entry height reachability
- `spawn_chunk()` uses existing spawn functions (platforms, enemies, coins)
- 30% chance per generation cycle when spacing allows

---

## Save System (save.rs)

**Unlimited save slots** with separate global settings:

- `SettingsData` — volumes, resolution, fullscreen, high score (`settings.json` / `settings` localStorage key)
- `SlotData` — score, checkpoint position, section, timestamp (per-slot files)
- `SlotIndex` — quick metadata for menu display without loading every slot

**Storage layout:**
- Native: `settings.json` + `saves/index.json` + `saves/slot_{id}.json`
- WASM: `my_sidescroller_settings` + `my_sidescroller_slots` + `my_sidescroller_slot_{id}` localStorage keys

**API:** `list_slots()`, `load_slot(id)`, `save_slot(id, data)`, `delete_slot(id)`, `create_slot(data)`, `update_slot(id, data)`

**Auto-save:** At each checkpoint, saves to `ActiveSlot`. If no slot exists yet, creates one automatically.

**Migration:** Legacy `save.json` format auto-migrated to `settings.json` + slot 0 on first load.

---

## Audio System (audio.rs)

- 8 SFX: jump, land, collect, hit, powerup, death, shoot, music
- OGG format loaded at startup with missing-file warnings
- Volume controlled by `GameSettings` (master + sfx + music)
- Music loops during Playing state, stops on exit
- SFX spawned as one-shot `AudioPlayer` entities

---

## File Structure

```
src/
  main.rs          (67 lines)   - App setup, plugin registration
  constants.rs     (256 lines)  - All tuning values
  state.rs         (400 lines)  - GameState, menus, pause, save menu, transitions
  input.rs         (185 lines)  - Keyboard/gamepad -> GameInput
  player.rs        (535 lines)  - Movement, physics, collision, respawn
  level.rs         (742 lines)  - Procedural generation, difficulty
  enemies.rs       (697 lines)  - All enemy types, AI, combat
  hazards.rs       (598 lines)  - Spikes, saws, lava, boulders, traps
  hud.rs           (760 lines)  - Score, health, powerup bars, banners, save menu
  camera.rs        (152 lines)  - Follow, shake, hit freeze
  animation.rs     (255 lines)  - Sprite sheet cycling
  mountain_bg.rs   (287 lines)  - GPU compute shader plugin
  ldtk_chunks.rs   (635 lines)  - LDtk level parsing
  audio.rs         (228 lines)  - SFX and music
  particles.rs     (206 lines)  - Dust, debris, bursts
  collectibles.rs  (134 lines)  - Coins
  powerups.rs      (239 lines)  - Speed, jump, shield
  breakable.rs     (381 lines)  - Destructible blocks + run wear
  checkpoint.rs    (265 lines)  - Sections, banners, colors
  health.rs        (217 lines)  - Damage, death, invincibility
  save.rs          (370 lines)  - Save slot system, settings, migration
  sprites.rs       (52 lines)   - Asset loading
  highscore.rs     (56 lines)   - High score + checkpoint clear on game over
  transition.rs    (97 lines)   - Screen fades
  debug.rs         (220 lines)  - Overlay, god mode, cheats

assets/
  shaders/mountain_bg.wgsl      - GPU background shader (909 lines)
  audio/*.ogg                   - 8 sound effects + music
  sprites/*.png                 - 25 sprite images
  levels/chunks.ldtk            - LDtk project (3 chunks)
  *.gif, *.png                  - Player sprite sheets
```
