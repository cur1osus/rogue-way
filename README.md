# Merchant's Menagerie

2D roguelike bullet heaven game built with Bevy 0.18 (Rust). Play as a merchant who commands combat pets to survive endless enemy waves.

## Genre

Bullet heaven auto-battler with meta-progression - combining elements from Vampire Survivors with pet collection mechanics.

## Features

- 🐾 **Pet Combat System** - Command autonomous combat pets with unique abilities
- 🌊 **Endless Waves** - Face increasingly challenging enemy hordes
- ⬆️ **Level-Up System** - Choose upgrades during gameplay
- 💰 **Meta-Progression** - Persistent shop upgrades between runs
- 🎨 **Sprite Animations** - Animated character and enemy sprites

## Building

### Prerequisites

- Rust 1.70 or later
- Cargo

### Commands

```bash
# Run in development mode
cargo run

# Run with optimizations (recommended for gameplay)
cargo run --release

# Build without running
cargo build --release

# Run tests
cargo test
```

## Tech Stack

- **Engine**: Bevy 0.18
- **Language**: Rust
- **Architecture**: ECS (Entity Component System)

## Development

The project uses optimized dev profile settings for fast iteration:
- `opt-level = 1` for your code (faster compilation)
- `opt-level = 3` for dependencies (better runtime performance)

See `CLAUDE.md` for detailed architecture documentation and development guidelines.

## Project Status

Currently in Phase 2-3 of development:
- ✅ Core gameplay mechanics
- ✅ Pet system
- ✅ Enemy spawning
- 🚧 Meta-progression system
- 📋 Planned: More pet types, bosses, achievements

## License

This project is a game development exercise.
