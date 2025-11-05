# SolvraScript Game Standard Library

**Author:** Zachariah Obie
**License:** Apache License 2.0
**Status:** Phase 1 - Design & Specification
**Last Updated:** 2025-11-04

## Overview

The Game standard library provides Entity-Component-System architecture, scene management, input handling, 2D sprite rendering, physics simulation, and game utilities for building interactive games in SolvraScript. All modules are deterministic, frame-rate independent, and VM-compliant.

## Module Taxonomy & Imports

### Standard Library Import Syntax

```solvrascript
// Import entire module
import <game/ecs>;
import <game/scene>;
import <game/input>;
import <game/time>;
import <game/sprite>;
import <game/physics2d>;
import <game/audio>;
import <game/utils>;

// Import specific functions
import { create_world, create_entity, add_component } from <game/ecs>;
import { create_scene, transition } from <game/scene>;
import { is_key_pressed, get_mouse_pos } from <game/input>;
```

### Module Hierarchy

```
<game/>
├── ecs         # Entity-Component-System core
├── scene       # Scene graph and transitions
├── input       # Keyboard, mouse, gamepad input
├── time        # Delta-time and fixed-timestep
├── sprite      # 2D sprite rendering and animation
├── physics2d   # AABB collision and impulse physics
├── audio       # Audio playback (host bridge)
└── utils       # RNG, tweening, easing helpers
```

---

## Module: `<game/ecs>` - Entity-Component-System

### Purpose
Provides ECS architecture for composable game object design with entities, components, and systems.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `create_world` | `create_world() -> World` | ECS world instance | None |
| `create_entity` | `create_entity(world: World) -> EntityID` | Unique entity ID | None |
| `destroy_entity` | `destroy_entity(world: World, entity: EntityID) -> void` | None | `EntityNotFound` |
| `add_component` | `add_component(world: World, entity: EntityID, type: string, data: map) -> void` | None | `EntityNotFound` |
| `get_component` | `get_component(world: World, entity: EntityID, type: string) -> map` | Component data | `ComponentNotFound` |
| `remove_component` | `remove_component(world: World, entity: EntityID, type: string) -> void` | None | `ComponentNotFound` |
| `has_component` | `has_component(world: World, entity: EntityID, type: string) -> bool` | Presence status | None |
| `query` | `query(world: World, components: [string]) -> [EntityID]` | Matching entities | None |
| `register_system` | `register_system(world: World, name: string, components: [string], fn: function) -> void` | None | None |
| `run_systems` | `run_systems(world: World, delta_ms: int) -> void` | None | None |

### Component Structure

Components are plain data maps with arbitrary fields:

```solvrascript
// Position component
{"x": 100, "y": 200}

// Velocity component
{"vx": 5, "vy": -3}

// Sprite component
{"texture": "player.png", "width": 32, "height": 32}

// Health component
{"current": 80, "max": 100}
```

### Example Usage

```solvrascript
import { create_world, create_entity, add_component, query, register_system, run_systems } from <game/ecs>;

// Create world
let world = create_world();

// Create player entity with components
let player = create_entity(world);
add_component(world, player, "position", {"x": 100, "y": 100});
add_component(world, player, "velocity", {"vx": 0, "vy": 0});
add_component(world, player, "sprite", {"texture": "player.png", "width": 32, "height": 32});

// Create enemy entity
let enemy = create_entity(world);
add_component(world, enemy, "position", {"x": 300, "y": 200});
add_component(world, enemy, "velocity", {"vx": -2, "vy": 0});

// Register movement system
register_system(world, "movement", ["position", "velocity"], fn(world, entity, delta_ms) {
    let pos = get_component(world, entity, "position");
    let vel = get_component(world, entity, "velocity");

    pos["x"] = pos["x"] + vel["vx"] * (delta_ms / 1000.0);
    pos["y"] = pos["y"] + vel["vy"] * (delta_ms / 1000.0);
});

// Game loop
let delta_ms = 16;  // ~60 FPS
run_systems(world, delta_ms);

// Query all entities with position and sprite
let renderable = query(world, ["position", "sprite"]);
for (let entity in renderable) {
    let pos = get_component(world, entity, "position");
    let sprite = get_component(world, entity, "sprite");
    // Render sprite at position...
}
```

### Determinism & Sandbox Notes

- Fully deterministic (no I/O)
- Entity IDs are sequential integers starting from 1
- Component storage uses stable HashMap (insertion order)
- System execution order matches registration order
- Query results sorted by entity ID
- No dynamic component types (must be strings)

### Host Function Needs

None (pure SolvraScript implementation)

### Performance Targets

- Entity creation: < 1μs
- Component add/get/remove: < 500ns
- Query: < 10μs per 1000 entities
- System execution overhead: < 100μs
- Memory per entity: ~64 bytes base + component data

### Test Plan

1. Entity creation and destruction
2. Component add/remove/get
3. Component presence queries
4. Multi-component queries
5. System registration and execution
6. System execution order
7. Large world performance (10,000+ entities)

### @ZNOTE Rationale

ECS provides flexible, composable game architecture. Design emphasizes:
- **Performance**: Cache-friendly data layout
- **Simplicity**: Plain maps for component data
- **Flexibility**: Any component type via strings
- **Determinism**: Stable ordering for reproducibility

---

## Module: `<game/scene>` - Scene Management

### Purpose
Provides scene graph with parent-child relationships, transitions, and lifecycle hooks.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `create_scene` | `create_scene(name: string) -> Scene` | Scene instance | None |
| `add_node` | `add_node(scene: Scene, parent: NodeID, data: map) -> NodeID` | New node ID | `NodeNotFound` |
| `remove_node` | `remove_node(scene: Scene, node: NodeID) -> void` | None | `NodeNotFound` |
| `get_node` | `get_node(scene: Scene, node: NodeID) -> map` | Node data | `NodeNotFound` |
| `get_children` | `get_children(scene: Scene, node: NodeID) -> [NodeID]` | Child node IDs | `NodeNotFound` |
| `set_active` | `set_active(scene: Scene, active: bool) -> void` | None | None |
| `transition_to` | `transition_to(from: Scene, to: Scene, transition: string, duration_ms: int) -> void` | None | `InvalidTransition` |
| `update_scene` | `update_scene(scene: Scene, delta_ms: int) -> void` | None | None |

### Scene Node Structure

```solvrascript
{
    name: string,
    transform: {
        x: float,
        y: float,
        rotation: float,  // radians
        scale_x: float,
        scale_y: float
    },
    visible: bool,
    data: map  // User-defined data
}
```

### Example Usage

```solvrascript
import { create_scene, add_node, transition_to, update_scene } from <game/scene>;

// Create main menu scene
let menu_scene = create_scene("MainMenu");
let root = add_node(menu_scene, null, {
    "name": "Root",
    "transform": {"x": 0, "y": 0, "rotation": 0.0, "scale_x": 1.0, "scale_y": 1.0},
    "visible": true,
    "data": {}
});

let title = add_node(menu_scene, root, {
    "name": "Title",
    "transform": {"x": 400, "y": 100, "rotation": 0.0, "scale_x": 2.0, "scale_y": 2.0},
    "visible": true,
    "data": {"text": "My Game"}
});

// Create game scene
let game_scene = create_scene("Game");

// Transition from menu to game with fade
transition_to(menu_scene, game_scene, "fade", 1000);

// Update active scene each frame
update_scene(game_scene, 16);
```

### Determinism & Sandbox Notes

- Scene graph traversal is deterministic
- Node IDs are sequential integers
- Transform hierarchy uses stable ordering
- Transitions are time-based (requires deterministic timing)
- No automatic rendering (scene is data structure)

### Host Function Needs

None (pure SolvraScript implementation)

### Performance Targets

- Node creation: < 2μs
- Tree traversal: < 50μs per 1000 nodes
- Transform calculation: < 1μs per node
- Memory per node: ~128 bytes

### Test Plan

1. Scene creation and lifecycle
2. Node hierarchy (add, remove, get)
3. Transform inheritance
4. Visibility propagation
5. Scene transitions
6. Deep hierarchies (10+ levels)

### @ZNOTE Rationale

Scene graphs organize game objects spatially. Design focuses on:
- **Hierarchy**: Parent-child transforms
- **Flexibility**: User-defined node data
- **Transitions**: Built-in scene switching

---

## Module: `<game/input>` - Input Handling

### Purpose
Provides keyboard, mouse, and gamepad input abstractions with frame-based state tracking.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `is_key_pressed` | `is_key_pressed(key: string) -> bool` | Key state | None |
| `is_key_just_pressed` | `is_key_just_pressed(key: string) -> bool` | Key edge state | None |
| `is_key_released` | `is_key_released(key: string) -> bool` | Key release state | None |
| `get_mouse_pos` | `get_mouse_pos() -> {x: int, y: int}` | Mouse position | None |
| `is_mouse_pressed` | `is_mouse_pressed(button: int) -> bool` | Button state | None |
| `get_gamepad_axis` | `get_gamepad_axis(pad: int, axis: string) -> float` | Axis value (-1.0 to 1.0) | `GamepadNotConnected` |
| `is_gamepad_button` | `is_gamepad_button(pad: int, button: string) -> bool` | Button state | `GamepadNotConnected` |
| `update_input` | `update_input() -> void` | None | None |

### Key Name Constants

```solvrascript
// Letters: "A" - "Z"
// Numbers: "0" - "9"
// Special: "Space", "Enter", "Escape", "Shift", "Control", "Alt"
// Arrows: "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"
// Function: "F1" - "F12"
```

### Example Usage

```solvrascript
import { is_key_pressed, is_key_just_pressed, get_mouse_pos, update_input } from <game/input>;

// Game loop
while (true) {
    update_input();  // Poll input state

    // Continuous key press
    if (is_key_pressed("ArrowRight")) {
        player_x = player_x + 5;
    }

    // Single key press (once per press)
    if (is_key_just_pressed("Space")) {
        player_jump();
    }

    // Mouse input
    let mouse = get_mouse_pos();
    aim_weapon(mouse.x, mouse.y);

    if (is_mouse_pressed(0)) {  // Left button
        fire_weapon();
    }
}
```

### Determinism & Sandbox Notes

- Input polling is non-deterministic (external hardware)
- For replay/networking, log input events and replay them
- Requires `<sec/sandbox>` capability: `input.keyboard`, `input.mouse`, `input.gamepad`
- Input state cleared on `update_input()` call
- Maximum gamepad count: 4

### Host Function Needs

- `__host_input_poll() -> InputState`
- `__host_input_get_key(key: string) -> bool`
- `__host_input_get_mouse() -> (x, y, buttons)`
- `__host_input_get_gamepad(pad: int) -> GamepadState`

### Performance Targets

- Input polling: < 50μs per frame
- Key state query: < 100ns
- Mouse position: < 100ns

### Test Plan

1. Keyboard input detection
2. Just-pressed edge detection
3. Mouse position and buttons
4. Gamepad axis and buttons
5. Input state reset per frame
6. Sandbox enforcement

### @ZNOTE Rationale

Input handling is fundamental to interactive games. Design focuses on:
- **Simplicity**: Boolean queries for common cases
- **Edge detection**: Just-pressed/released helpers
- **Flexibility**: Supports keyboard, mouse, gamepad

---

## Module: `<game/time>` - Time Management

### Purpose
Provides delta-time calculation, fixed-timestep updates, and frame rate utilities.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `create_timer` | `create_timer() -> Timer` | Timer instance | None |
| `tick` | `tick(timer: Timer) -> int` | Delta time in ms | None |
| `get_fps` | `get_fps(timer: Timer) -> int` | Current FPS | None |
| `get_elapsed_ms` | `get_elapsed_ms(timer: Timer) -> int` | Total elapsed time | None |
| `create_fixed_step` | `create_fixed_step(step_ms: int) -> FixedStep` | Fixed timestep instance | None |
| `should_update` | `should_update(fixed: FixedStep, delta_ms: int) -> bool` | Update needed | None |
| `consume_step` | `consume_step(fixed: FixedStep) -> void` | None | None |

### Example Usage

```solvrascript
import { create_timer, tick, get_fps, create_fixed_step, should_update, consume_step } from <game/time>;

// Variable timestep game loop
let timer = create_timer();

while (true) {
    let delta_ms = tick(timer);

    update_game(delta_ms);
    render_game();

    if (get_elapsed_ms(timer) % 1000 < 16) {
        println("FPS: " + str(get_fps(timer)));
    }
}

// Fixed timestep physics loop
let physics_timer = create_timer();
let fixed_step = create_fixed_step(16);  // 60 Hz physics

while (true) {
    let delta_ms = tick(physics_timer);

    // Update physics at fixed rate
    while (should_update(fixed_step, delta_ms)) {
        update_physics(16);
        consume_step(fixed_step);
    }

    // Render at variable rate
    render_game();
}
```

### Determinism & Sandbox Notes

- Time measurement uses monotonic clock (non-deterministic)
- For deterministic replay, use fixed timestep and recorded inputs
- Fixed timestep ensures deterministic physics simulation
- FPS calculation uses 1-second rolling average
- Requires `<sec/sandbox>` capability: `time.monotonic`

### Host Function Needs

- `__host_time_now_ms() -> int` (monotonic time)

### Performance Targets

- Timer tick: < 1μs
- FPS calculation: < 500ns
- Fixed-step check: < 100ns

### Test Plan

1. Delta-time calculation
2. FPS averaging
3. Elapsed time tracking
4. Fixed timestep updates
5. Accumulator drain
6. Deterministic timing for `<game/time>`

### @ZNOTE Rationale

Frame-rate independent timing is essential for smooth gameplay. Design provides:
- **Variable timestep**: For rendering and general updates
- **Fixed timestep**: For deterministic physics
- **FPS monitoring**: For performance debugging

---

## Module: `<game/sprite>` - 2D Sprite Rendering

### Purpose
Provides 2D sprite rendering, sprite sheet animation, and texture management.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `load_texture` | `load_texture(path: string) -> TextureID` | Texture handle | `FileNotFound`, `InvalidFormat` |
| `unload_texture` | `unload_texture(texture: TextureID) -> void` | None | None |
| `draw_sprite` | `draw_sprite(texture: TextureID, x: int, y: int, width: int, height: int) -> void` | None | `InvalidTexture` |
| `draw_sprite_region` | `draw_sprite_region(texture: TextureID, src: Rect, dst: Rect) -> void` | None | `InvalidTexture` |
| `create_animation` | `create_animation(texture: TextureID, frames: [Rect], frame_ms: int) -> Animation` | Animation instance | None |
| `play_animation` | `play_animation(anim: Animation, x: int, y: int, elapsed_ms: int) -> void` | None | None |
| `set_sprite_flip` | `set_sprite_flip(flip_x: bool, flip_y: bool) -> void` | None | None |

### Rect Structure

```solvrascript
{
    x: int,      // Top-left X
    y: int,      // Top-left Y
    w: int,      // Width
    h: int       // Height
}
```

### Example Usage

```solvrascript
import { load_texture, draw_sprite, create_animation, play_animation } from <game/sprite>;

// Load sprite sheet
let player_tex = load_texture("assets/player_sheet.png");

// Define animation frames (8 frames in a row, each 32x32)
let walk_frames = [];
for (let i = 0; i < 8; i = i + 1) {
    push(walk_frames, {"x": i * 32, "y": 0, "w": 32, "h": 32});
}

let walk_anim = create_animation(player_tex, walk_frames, 100);  // 100ms per frame

// Game loop
let elapsed = 0;
while (true) {
    elapsed = elapsed + delta_ms;

    // Draw animated sprite
    play_animation(walk_anim, player_x, player_y, elapsed);

    // Or draw static sprite
    draw_sprite(player_tex, enemy_x, enemy_y, 32, 32);
}
```

### Determinism & Sandbox Notes

- Texture loading is non-deterministic (file I/O)
- Rendering order is deterministic (call order)
- Requires `<sec/sandbox>` capabilities: `fs.read`, `gfx.render`
- Requires host graphics backend (OpenGL, WebGPU, etc.)
- Maximum texture size: 4096x4096
- Supported formats: PNG, JPEG, BMP

### Host Function Needs

- `__host_gfx_load_texture(path) -> texture_id`
- `__host_gfx_unload_texture(texture_id) -> void`
- `__host_gfx_draw_quad(texture_id, src_rect, dst_rect, flip_x, flip_y) -> void`

### Performance Targets

- Texture load: < 100ms for 1024x1024 PNG
- Draw call: < 10μs
- Animation update: < 1μs
- Memory per texture: ~width * height * 4 bytes (RGBA)

### Test Plan

1. Texture loading (PNG, JPEG)
2. Basic sprite drawing
3. Sprite region drawing
4. Animation playback
5. Sprite flipping
6. Large sprite sheets
7. Memory cleanup

### @ZNOTE Rationale

2D sprite rendering is core to many games. Design emphasizes:
- **Simplicity**: Direct draw calls
- **Animation**: Built-in sprite sheet support
- **Flexibility**: Region-based drawing for atlases

---

## Module: `<game/physics2d>` - 2D Physics

### Purpose
Provides AABB collision detection, impulse-based physics, and basic 2D physics simulation.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `create_body` | `create_body(x: float, y: float, width: float, height: float) -> Body` | Physics body | None |
| `set_velocity` | `set_velocity(body: Body, vx: float, vy: float) -> void` | None | None |
| `apply_impulse` | `apply_impulse(body: Body, fx: float, fy: float) -> void` | None | None |
| `set_mass` | `set_mass(body: Body, mass: float) -> void` | None | None |
| `update_body` | `update_body(body: Body, delta_ms: int) -> void` | None | None |
| `check_collision` | `check_collision(a: Body, b: Body) -> bool` | Collision status | None |
| `get_collision_info` | `get_collision_info(a: Body, b: Body) -> CollisionInfo` | Collision details | `NoCollision` |
| `resolve_collision` | `resolve_collision(a: Body, b: Body, restitution: float) -> void` | None | None |

### Body Structure

```solvrascript
{
    x: float,          // Position X
    y: float,          // Position Y
    width: float,      // Bounding box width
    height: float,     // Bounding box height
    vx: float,         // Velocity X
    vy: float,         // Velocity Y
    mass: float,       // Mass (1.0 default)
    static: bool       // Is static (immovable)
}
```

### CollisionInfo Structure

```solvrascript
{
    normal_x: float,   // Collision normal X
    normal_y: float,   // Collision normal Y
    depth: float       // Penetration depth
}
```

### Example Usage

```solvrascript
import { create_body, set_velocity, apply_impulse, update_body, check_collision, resolve_collision } from <game/physics2d>;

// Create player body
let player = create_body(100.0, 100.0, 32.0, 32.0);
set_velocity(player, 5.0, 0.0);

// Create ground (static)
let ground = create_body(0.0, 400.0, 800.0, 50.0);
ground["static"] = true;

// Physics loop (fixed timestep)
let delta_ms = 16;
while (true) {
    // Apply gravity
    if (!player["static"]) {
        apply_impulse(player, 0.0, 0.5);  // Downward force
    }

    // Update physics
    update_body(player, delta_ms);

    // Check collision
    if (check_collision(player, ground)) {
        resolve_collision(player, ground, 0.5);  // 0.5 = bounciness
    }
}
```

### Determinism & Sandbox Notes

- Fully deterministic (pure math)
- Requires fixed timestep for reproducible physics
- AABB collision only (no rotation)
- Simple impulse resolution (not full physics engine)
- No external state

### Host Function Needs

None (pure SolvraScript implementation)

### Performance Targets

- Body update: < 500ns
- Collision check: < 200ns
- Collision resolution: < 1μs
- Memory per body: ~64 bytes

### Test Plan

1. Body creation and updates
2. AABB collision detection
3. Impulse application
4. Collision resolution
5. Static vs dynamic bodies
6. Multi-body collisions
7. Deterministic physics with fixed timestep

### @ZNOTE Rationale

Simple 2D physics enables platformers and arcade games. Design focuses on:
- **Simplicity**: AABB only, no rotation
- **Performance**: Fast collision checks
- **Determinism**: Reproducible simulation

---

## Module: `<game/audio>` - Audio Playback

### Purpose
Provides audio playback with volume control and basic mixing (requires host bridge).

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `load_sound` | `load_sound(path: string) -> SoundID` | Sound handle | `FileNotFound`, `InvalidFormat` |
| `unload_sound` | `unload_sound(sound: SoundID) -> void` | None | None |
| `play_sound` | `play_sound(sound: SoundID, volume: float) -> void` | None | `InvalidSound` |
| `play_music` | `play_music(sound: SoundID, volume: float, loop: bool) -> void` | None | `InvalidSound` |
| `stop_music` | `stop_music() -> void` | None | None |
| `set_volume` | `set_volume(volume: float) -> void` | None | None |

### Example Usage

```solvrascript
import { load_sound, play_sound, play_music, set_volume } from <game/audio>;

// Load audio files
let jump_sfx = load_sound("assets/jump.wav");
let bg_music = load_sound("assets/music.ogg");

// Play background music (looping)
play_music(bg_music, 0.7, true);

// Play sound effect
if (is_key_just_pressed("Space")) {
    play_sound(jump_sfx, 1.0);
}

// Adjust volume
set_volume(0.5);  // 50% master volume
```

### Determinism & Sandbox Notes

- Audio playback is non-deterministic (timing)
- For replay, do not include audio in determinism model
- Requires `<sec/sandbox>` capability: `audio.play`
- Supported formats: WAV, OGG, MP3
- Maximum concurrent sounds: 16

### Host Function Needs

- `__host_audio_load(path) -> sound_id`
- `__host_audio_unload(sound_id) -> void`
- `__host_audio_play(sound_id, volume, loop) -> void`
- `__host_audio_stop() -> void`
- `__host_audio_set_volume(volume) -> void`

### Performance Targets

- Sound loading: < 500ms
- Play latency: < 50ms
- Memory per sound: ~size of audio file

### Test Plan

1. Sound loading (WAV, OGG)
2. Sound playback
3. Music looping
4. Volume control
5. Concurrent sounds
6. Memory cleanup

### @ZNOTE Rationale

Audio is essential for game feel. Design is minimal:
- **Host-dependent**: Requires audio backend
- **Simple API**: Load and play
- **Future expansion**: 3D audio, effects, streaming

---

## Module: `<game/utils>` - Game Utilities

### Purpose
Provides RNG, tweening, easing functions, and common game math helpers.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `random_int` | `random_int(min: int, max: int) -> int` | Random integer | None |
| `random_float` | `random_float(min: float, max: float) -> float` | Random float | None |
| `random_choice` | `random_choice(list: [any]) -> any` | Random element | `EmptyList` |
| `seed_rng` | `seed_rng(seed: int) -> void` | None | None |
| `lerp` | `lerp(a: float, b: float, t: float) -> float` | Interpolated value | None |
| `clamp` | `clamp(value: float, min: float, max: float) -> float` | Clamped value | None |
| `ease_in_out` | `ease_in_out(t: float) -> float` | Eased value | None |
| `distance` | `distance(x1: float, y1: float, x2: float, y2: float) -> float` | Distance | None |
| `angle_between` | `angle_between(x1: float, y1: float, x2: float, y2: float) -> float` | Angle in radians | None |

### Example Usage

```solvrascript
import { random_int, seed_rng, lerp, ease_in_out, distance } from <game/utils>;

// Seeded RNG for deterministic replays
seed_rng(12345);

// Random enemy spawn
let enemy_x = random_int(0, 800);
let enemy_y = random_int(0, 600);

// Smooth interpolation for camera movement
let camera_x = lerp(camera_x, target_x, 0.1);

// Easing for UI animations
let t = elapsed / duration;
let eased = ease_in_out(t);
let button_y = lerp(start_y, end_y, eased);

// Distance check for AI
let dist = distance(player_x, player_y, enemy_x, enemy_y);
if (dist < 100.0) {
    enemy_chase_player();
}
```

### Determinism & Sandbox Notes

- RNG is deterministic when seeded
- Without seed, uses non-deterministic system entropy
- Math functions are fully deterministic
- No external state

### Host Function Needs

- `__host_random_bytes(count) -> bytes` (for unseeded RNG)

### Performance Targets

- RNG generation: < 100ns
- Math functions: < 50ns
- Memory: minimal (RNG state ~32 bytes)

### Test Plan

1. Seeded RNG reproducibility
2. Random range validation
3. Lerp interpolation
4. Easing function curves
5. Distance calculation
6. Angle calculation

### @ZNOTE Rationale

Game utilities provide essential math helpers. Design focuses on:
- **Determinism**: Seeded RNG for replays
- **Convenience**: Common operations in single functions
- **Performance**: Fast math operations

---

## Summary

The Game standard library provides comprehensive ECS architecture, scene management, input handling, 2D rendering, physics simulation, and game utilities for building interactive games in SolvraScript. All modules prioritize frame-rate independence, deterministic physics, and integration with host graphics/audio backends.

### Module Dependencies

```
<game/ecs> - Standalone
<game/scene> - Standalone
<game/input> - Requires host input backend
<game/time> - Requires host timing
<game/sprite> - Requires host graphics backend + <gfx/2d>
<game/physics2d> - Standalone (pure math)
<game/audio> - Requires host audio backend
<game/utils> - Minimal host RNG for unseeded random
```

### Next Implementation Phase

See `specs/module_index.md` for complete function inventory and `specs/host_bridge_map.md` for host function requirements.
