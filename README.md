# pyzalea

Bindings of the [azalea-rs](https://github.com/azalea-rs/azalea) library for Python. This lets you create headless bots for the game Minecraft in Python. Unlike other Java-based systems that interact with Minecraft servers, Azalea is built from the ground up in Rust and is completely headless, meaning there is no overhead from graphics or launching the game. This allows it to run extremely fast and scalable, especially for simulations such as RL.

Not all Azalea methods are implemented as this was originally designed for a very narrow use-case (RL) but contributions are more than welcome.

[Join the Discord](https://discord.com/invite/JmqmBGKz7z) (for PvPBot, teaching AI to play Crystal PvP!)

## Installation

```bash
pip install git+https://github.com/bl791/pyzalea
```

Or build from source:

```bash
pip install maturin
maturin develop --release
```

## Quick Start

```python
import pyzalea

# Connect to a remote server
bot = pyzalea.connect("localhost", 25565, "MyBot")

while True:
    state = bot.get_state()

    bot.move_forward()
    bot.look_at(state.x + 10, state.y, state.z)

    if state.attack_cooldown >= 1.0:
        bot.attack()

    # Advance one tick
    state = bot.tick()
```

## State Information

The `GameState` object contains:

```python
state.x, state.y, state.z          # Position
state.yaw, state.pitch             # Look direction
state.velocity_x/y/z               # Velocity
state.health                       # Health (0-20)
state.food                         # Food level (0-20)
state.is_on_ground                 # Ground contact
state.is_sprinting                 # Sprint state
state.attack_cooldown              # 0.0-1.0, 1.0 = ready to attack
state.entities                     # List of nearby entities
state.tick                         # Current game tick
```

### Entity Information

```python
for entity in state.entities:
    print(entity.id)               # Entity ID
    print(entity.entity_type)      # "player", "zombie", etc.
    print(entity.x, entity.y, entity.z)
    print(entity.health)
    print(entity.distance_to(other_entity))
```

### Convenience Methods

```python
# Find nearest player
enemy = state.nearest_entity(entity_type="player", max_distance=32.0)

# Get all nearby players
players = state.nearby_players(max_distance=16.0)

# Convert to flat vector
obs = state.to_vector()  # Returns List[float]
```

## Multi-bot (Swarm)

```python
# Create multiple bots
bots = pyzalea.connect_swarm("localhost", 25565, ["Bot1", "Bot2", "Bot3"])

for bot in bots:
    state = bot.get_state()
    # ...
```

## License

GNU LGPL-2.1
