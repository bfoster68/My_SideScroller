# Building an Infinite Side-Scroller in Rust with Bevy

I've been building a 2D side-scrolling platformer from scratch using Rust and the Bevy game engine, and I wanted to share where it's at and what the tech looks like under the hood.

## The Game

It's an infinite runner-style platformer. You jump across procedurally generated platforms, stomp enemies, collect coins, dodge hazards, and try to survive as long as possible. Think classic side-scroller meets endless runner — the world builds itself as you go, and it gets harder the further you get.

There are six enemy types (walkers, flyers, shooters, chargers, flying ranged, and saws), powerups like speed boosts, triple jump, and shields, plus environmental hazards like lava pools, falling boulders, and timed spike traps. The level is broken into themed sections — Forest, Cave, Sky, Ruins — each with its own color palette and difficulty curve.

## Why Rust and Bevy

I went with Rust because I wanted something fast and correct. No garbage collector pauses, no runtime surprises. Bevy specifically because its ECS architecture makes it natural to compose game systems without everything turning into spaghetti. Each piece of the game — player physics, enemy AI, particles, audio — lives in its own plugin. Right now the project has 19 modules and they stay out of each other's way.

Bevy 0.18 has been solid. The ECS queries are expressive, the asset pipeline just works, and the community is active when you hit a wall.

## Procedural Generation + LDtk

The level generation is hybrid. Most of the world is procedural — platforms, ground segments, enemy placement, coin formations all get generated ahead of the camera and cleaned up behind it. Difficulty scales from 0.0 to 1.0 based on your score, affecting spawn rates, gap sizes, and enemy density.

But pure procedural can feel samey, so I integrated LDtk (Level Designer Toolkit) for hand-designed level chunks. About 30% of the time, instead of generating another random platform section, the system drops in a pre-built chunk from an LDtk project file. These chunks have their own collision geometry, enemy placements, and difficulty ratings. The procedural system stitches them in seamlessly — matching entry and exit heights so the player doesn't notice the transition.

I parse the LDtk JSON directly with serde rather than pulling in a heavy integration crate. It keeps the dependency tree lean and gives me full control over how chunks get spawned.

## Custom Physics

No physics engine. Everything runs on custom AABB collision with separated horizontal and vertical passes. It's simple, predictable, and fast. Variable jump height (tap vs hold), coyote time, double jumps, knockback — all hand-tuned in a constants file where every value is one tweak away.

## The Small Details

The stuff that makes it feel good: hit freeze that slows time to 2% on impacts, screen shake with quadratic trauma falloff, particle effects on jumps and landings, a combo system that multiplies score on consecutive stomps. A 3-layer parallax background. Fade transitions between screens. These are the things that take a game from "it works" to "it feels right."

## Deployment

The game compiles to WASM and runs in a browser. Trunk handles the build pipeline, and it's set up for itch.io hosting. Same codebase, native and web — Bevy's cross-platform story has been smooth here, with only a couple of platform-specific guards needed (like swapping `std::process::exit` for a WASM-safe alternative).

## What's Next

More LDtk chunks for level variety, boss encounters, and polish passes on the difficulty curve. The foundation is solid and adding new enemy types or hazards is mostly just defining a new component and writing a behavior system. That's the payoff of ECS done right.

If you want to try it, the WASM build is up on itch.io. Gamepad supported.
