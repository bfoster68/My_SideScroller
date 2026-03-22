# Polish Plan — Round 2

## Phase 1: Bug Fixes

### 1. Fix checkpoint section banner operator precedence
- **File**: `src/checkpoint.rs` line 187
- **Bug**: `&&` binds tighter than `||`, causing incorrect section transition logic
- **Fix**: Add explicit parentheses to clarify intent

### 2. Reduce projectile lifetime
- **File**: `src/constants.rs`
- **Issue**: `PROJECTILE_LIFETIME = 4.0` lets projectiles travel 1120px — way past the screen
- **Fix**: Reduce to 2.0s

### 3. Add camera snap on respawn
- **File**: `src/camera.rs`
- **Issue**: Camera only snaps on `OnEnter(Playing)`, not on mid-game respawn
- **Fix**: Insert `NeedsCameraSnap` when `respawn_on_fall` teleports the player

## Phase 2: Gameplay Feel

### 4. Particle burst on projectile impact
- **File**: `src/enemies.rs` (projectile_player_collision)
- **Issue**: Projectile hits player with no visual feedback beyond screen shake
- **Fix**: Spawn a small red particle burst at impact point

### 5. Timed trap reactivation warning
- **File**: `src/hazards.rs` (timed_trap_cycle)
- **Issue**: Traps reappear without warning when reactivating
- **Fix**: Add a brief flash/pulse before the trap becomes dangerous again

### 6. Shield break visual effect
- **File**: `src/health.rs` (apply_damage, shield removal)
- **Issue**: Shield absorbs a hit and vanishes silently
- **Fix**: Spawn a gold particle burst when shield breaks

### 7. Scale powerup spawn chance with difficulty
- **File**: `src/level.rs`
- **Issue**: `POWERUP_SPAWN_CHANCE = 0.05` is flat across all difficulties
- **Fix**: Increase from 5% at d=0 to 10% at d=1 to reward harder gameplay

## Phase 3: Performance

### 8. Replace Vec allocation in respawn_on_fall
- **File**: `src/player.rs` lines 379-384
- **Issue**: Allocates a Vec of all hazard positions every frame
- **Fix**: Use iterator-based distance check instead of collecting

### 9. Optimize debug overlay
- **File**: `src/debug.rs`
- **Issue**: Fully reconstructs debug text string every frame
- **Fix**: Only regenerate when relevant values change (use change detection)

## Phase 4: Error Handling & Robustness

### 10. Validate save file on load
- **File**: `src/save.rs`
- **Issue**: Corrupted JSON silently returns None with no recovery
- **Fix**: Log warning and reset to defaults on deserialization failure

### 11. Warn on missing LDtk chunks
- **File**: `src/ldtk_chunks.rs`
- **Issue**: Missing .ldtk file silently disables chunk system
- **Fix**: Log a warning when the file is missing or fails to parse

## Phase 5: Sound Effects

### 12. Add missing sound effects
New SFX needed (can be procedurally generated or placeholder):
- Shield break sound
- Breakable block destruction
- Charging enemy wind-up growl
- Section transition fanfare
- Power-up expiring warning beep
- Low health heartbeat

## Phase 6: Code Cleanup

### 13. Remove dead code fields
- `breakable.rs`: Remove `group_id` field or implement falling-block physics
- `hazards.rs`: Remove `phase_offset` or use it for trap desync
- `audio.rs`: Remove `effective_sfx_volume()` or integrate it

### 14. Fix powerup spawn height inconsistency
- **File**: `src/powerups.rs` line 96
- **Issue**: Powerups spawn 10px higher than coins on same platform
- **Fix**: Use same `COIN_FLOAT_HEIGHT` formula or define `POWERUP_FLOAT_HEIGHT`
