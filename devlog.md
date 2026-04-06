# Devlog: Endless Difficulty, New Chunks, and Quality of Life

The game has received a major update focusing on late-game challenge, code quality, and usability. Here's what's new:

## Endless Difficulty Scaling

The biggest gameplay change: **difficulty no longer caps out.** Previously, the game stopped getting harder after 5,000 points. Now difficulty keeps climbing using a logarithmic curve — the game always has another gear.

What changes as you push past the old cap:
- Platform gaps get wider and platforms get narrower
- Enemies spawn more frequently and move faster
- Shooters fire faster with quicker projectiles
- More ground gaps, more crumbling platforms, more breakable blocks
- Boulders drop more often
- Power-up durations shrink so you can't rely on them as long
- Coin values increase to reward skilled play

The early game (0-5,000 points) plays exactly the same as before — the scaling only adds challenge beyond where the old system left off.

## 7 New Extreme Chunks

Hand-designed level sections now appear at high scores to keep things interesting:

- **Enemy Gauntlet** — Every platform has an enemy. No breathing room.
- **Narrow Run** — Tiny 2-3 tile platforms with huge gaps between them.
- **Crumble Gauntlet** — All crumbling platforms with flying enemies. Keep moving or fall.
- **Ice Shooters** — Ice platforms plus shooter enemies and falling boulders. Sliding into projectiles is a real risk.
- **Mixed Hell** — The longest chunk at 2000px. Conveyors, crumbling bridges over lava, ice shooters, springs with flying enemies, and charging enemies on breakable floors.
- **Lava Sprint** — Crumbling platforms over lava with boulders raining down.
- **Sky Assault** — Spring-powered vertical gauntlet with flying ranged enemies at every altitude and lava below.

The game now has **24 hand-designed chunks** total (up from 17).

## Mouse Support in Menus

All menus now support mouse interaction:
- **Hover** over any menu item to highlight it
- **Click** to select
- Keyboard and gamepad still work exactly as before

This applies to the Main Menu, Pause Menu, Settings, and Save/Load screens.

## Under the Hood

- **Spatial hashing** for collision detection — the game now uses a grid to skip distant entities instead of checking everything every frame
- **Debug overlay optimization** — entity counts update 4x per second instead of every frame
- **Code reorganization** — large files split into focused modules for easier future development

## Play It Now

Jump in and see how far past 5,000 points you can survive. The new extreme chunks start appearing around that point, and the difficulty never stops climbing. Good luck!
