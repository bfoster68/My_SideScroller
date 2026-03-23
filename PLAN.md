# Polish Plan — Round 2

## All Completed

- [x] **Breakable block run wear** — blocks degrade visually and break when player runs across them
- [x] **Game over clears checkpoint** — new games always start at section 1
- [x] **Combo system extended** — breakable block destruction contributes to combo chain
- [x] **Shooter enemy scaling** — fire rate, projectile speed scale with difficulty; range-gated at 600px
- [x] **Charging enemy tuned** — detect range 300→220px, wind-up 0.3→0.45s
- [x] **Coin value scaling** — 10pts at d=0 → 50pts at d=1
- [x] **Flying ranged projectile fix** — direction vector no longer double-scaled
- [x] **Lava hitbox fix** — extends downward only, not above visual
- [x] **Particle burst cap** — MAX_PARTICLES/4 limit on burst spawning
- [x] **Audio load warnings** — logs missing audio files at startup
- [x] **Fix checkpoint section banner operator precedence** — added parentheses
- [x] **Reduce projectile lifetime** — 4.0s → 2.0s
- [x] **Camera snap on respawn** — NeedsCameraSnap inserted when player teleports
- [x] **Projectile impact particles** — red burst at hit point
- [x] **Shield break visual effect** — gold burst when shield absorbs last hit
- [x] **Powerup spawn scaling** — 5% at d=0 → 12% at d=1
- [x] **Respawn optimization** — replaced Vec allocation with iterator-based hazard check
- [x] **Save file validation** — corrupted JSON logs warning, resets to defaults
- [x] **Dead code cleanup** — removed group_id, phase_offset, effective_sfx_volume
- [x] **Powerup spawn height** — removed +10px offset, matches coin height
- [x] **System ordering fix** — CheckpointResetSet runs before LevelResetSet
- [x] **Unlimited save slots** — load, save, delete from menus with auto-save at checkpoints
- [x] **Quit Game option** — added to pause menu
- [x] **Platform reachability** — constrain rise by gap distance, every 8th platform near ground

## Remaining Ideas (Future Work)

### Sound Effects
- Shield break sound
- Breakable block destruction sound
- Charging enemy wind-up growl
- Section transition fanfare
- Power-up expiring warning beep
- Low health heartbeat

### Debug Overlay Optimization
- Only regenerate text when relevant values change (change detection)

### Additional Polish
- Port compute shader background to main game (currently integrated)
- Design more LDtk chunks for variety (currently 3 starter chunks)
- Add touch controls for mobile WASM testing
