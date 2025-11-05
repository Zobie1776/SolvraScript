# SolvraScript Standard Library

**Author:** Zachariah Obie
**License:** Apache License 2.0
**Status:** Phase 1 - Design Complete
**Date:** 2025-11-04

## Overview

This is the **expanded SolvraScript standard library** covering Web, Game, Security, and Supporting modules. All modules are designed following **Zobie.format**, implemented in native `.svs/.svc` syntax, and compiled by the SolvraCore VM. The library provides 209 functions across 28 modules organized into 4 families.

## Project Structure

```
solvra_script/stdlib/
├── README.md                 # This file
│
├── docs/                     # Module documentation
│   ├── web_stdlib.md         # HTTP, WebSocket, routing, templates
│   ├── game_stdlib.md        # ECS, sprites, physics, input
│   ├── sec_stdlib.md         # Crypto, JWT, PKI, sandbox
│   └── supporting_stdlib.md  # Networking, data I/O, graphics
│
├── specs/                    # Technical specifications
│   ├── module_index.md       # Complete function index (209 functions)
│   ├── security_model.md     # Capability-based security model
│   └── host_bridge_map.md    # Host function mappings (72 host functions)
│
├── web/                      # Web modules
│   ├── http.svs              # HTTP client
│   ├── server.svs            # HTTP server (stub)
│   ├── router.svs            # Request routing (stub)
│   ├── ws.svs                # WebSocket (stub)
│   ├── tpl.svs               # Template engine (stub)
│   ├── static.svs            # Static files (stub)
│   └── utils.svs             # Web utilities (stub)
│
├── game/                     # Game modules
│   ├── ecs.svs               # Entity-Component-System
│   ├── scene.svs             # Scene management (stub)
│   ├── input.svs             # Input handling (stub)
│   ├── time.svs              # Time/delta management (stub)
│   ├── sprite.svs            # 2D sprites (stub)
│   ├── physics2d.svs         # 2D physics (stub)
│   ├── audio.svs             # Audio playback (stub)
│   └── utils.svs             # Game utilities (stub)
│
├── sec/                      # Security/Crypto modules
│   ├── hash.svs              # SHA-256, BLAKE3, HMAC
│   ├── aead.svs              # XChaCha20-Poly1305, AES-GCM (stub)
│   ├── kdf.svs               # scrypt, Argon2 (stub)
│   ├── jwt.svs               # JSON Web Tokens (stub)
│   ├── pki.svs               # PKI/X.509 (stub)
│   ├── sandbox.svs           # Capability sandbox (stub)
│   └── fuzz.svs              # Fuzzing utilities (stub)
│
├── net/                      # Networking modules
│   └── sock.svs              # TCP/UDP sockets, DNS
│
├── devops/                   # DevOps modules
│   └── runner.svs            # Process spawning (stub)
│
├── data/                     # Data I/O modules
│   └── io.svs                # CSV/JSON/JSONL (stub)
│
├── gfx/                      # Graphics modules
│   └── 2d.svs                # 2D framebuffer drawing (stub)
│
├── storage/                  # Storage modules
│   └── kv.svs                # Key-value store (stub)
│
├── crypto/                   # High-level crypto wrappers
│   └── wrap.svs              # File encryption/signing (stub)
│
├── examples/                 # Example programs
│   ├── web_min_server.svs    # Minimal HTTP server
│   ├── game_ecs_pong.svs     # ECS-based Pong game
│   ├── sec_jwt_roundtrip.svs # JWT authentication demo
│   └── data_csv_pipeline.svs # CSV ETL pipeline
│
└── tests/                    # Test suites
    └── test_http.svs         # HTTP module tests
```

## Module Families

### 🌐 Web Modules (51 functions)
- `<web/http>` — HTTP client with retry/timeout
- `<web/server>` — HTTP server with middleware
- `<web/router>` — Path routing with parameters
- `<web/ws>` — WebSocket client/server
- `<web/tpl>` — Template engine (`{{var}}`)
- `<web/static>` — Static file serving with ETags
- `<web/utils>` — URL parsing, cookies, escaping

### 🎮 Game Modules (63 functions)
- `<game/ecs>` — Entity-Component-System
- `<game/scene>` — Scene graph with transitions
- `<game/input>` — Keyboard, mouse, gamepad
- `<game/time>` — Delta-time, fixed-timestep
- `<game/sprite>` — 2D sprite rendering
- `<game/physics2d>` — AABB collision, impulses
- `<game/audio>` — Audio playback
- `<game/utils>` — RNG, lerp, easing

### 🔒 Security Modules (45 functions)
- `<sec/hash>` — SHA-256, SHA-512, BLAKE3, HMAC
- `<sec/aead>` — XChaCha20-Poly1305, AES-256-GCM
- `<sec/kdf>` — scrypt, Argon2 key derivation
- `<sec/jwt>` — JWT signing (HS256, RS256)
- `<sec/pki>` — X.509 certificate parsing
- `<sec/sandbox>` — Capability-based security
- `<sec/fuzz>` — Fuzzing utilities

### 🔧 Supporting Modules (50 functions)
- `<net/sock>` — TCP/UDP sockets, DNS
- `<devops/runner>` — Process spawning
- `<data/io>` — CSV/JSON/JSONL I/O
- `<gfx/2d>` — 2D canvas drawing
- `<storage/kv>` — Key-value persistence
- `<crypto/wrap>` — High-level crypto wrappers

## Implementation Status

| Family | Modules | Functions | Documented | Stubbed | Examples | Tests |
|--------|---------|-----------|------------|---------|----------|-------|
| Web | 7 | 51 | ✅ 100% | ✅ 1/7 | ✅ 1 | ✅ 1 |
| Game | 8 | 63 | ✅ 100% | ✅ 1/8 | ✅ 1 | ⏳ 0 |
| Security | 7 | 45 | ✅ 100% | ✅ 1/7 | ✅ 1 | ⏳ 0 |
| Supporting | 6 | 50 | ✅ 100% | ✅ 1/6 | ✅ 1 | ⏳ 0 |
| **TOTAL** | **28** | **209** | **✅ 100%** | **4/28** | **✅ 4** | **1/28** |

**Legend:**
- ✅ Complete
- ⏳ Pending
- ❌ Not started

## Security Model

The stdlib uses **capability-based security** integrated with `<sec/sandbox>`:

### Permission Tiers

| Tier | Name | Capabilities | Risk | Use Cases |
|------|------|-------------|------|-----------|
| 0 | Pure Computation | None | ✅ Safe | Math, algorithms |
| 1 | Read-Only | `fs.read`, `net.dns` | ✅ Low | Config loaders |
| 2 | Local Write | `fs.read`, `fs.write` | ⚠️ Medium | Data storage |
| 3 | Network Client | `net.http.client` | ⚠️ Medium | API clients |
| 4 | Network Server | `net.http.server` | ⚠️ High | Web servers |
| 5 | Process Exec | `process.spawn` | ⚠️ High | Build tools |
| 6 | Cryptography | `crypto.*` | ⚠️ Medium | Encryption |
| 7 | Full System | All capabilities | ❌ Critical | Admin tools |

### Capability Hierarchy

```
fs.read              net.http.client      crypto.encrypt
fs.write             net.http.server      crypto.sign
fs.execute           net.websocket        crypto.kdf
fs (all)             net.tcp              crypto (all)
                     net.udp
                     net (all)
```

See `specs/security_model.md` for complete details.

## Host Bridge Architecture

The stdlib interfaces with SolvraCore via 72 host functions:

### Host Function Categories

| Category | Host Functions | Examples |
|----------|----------------|----------|
| Filesystem | 8 | `__host_fs_read`, `__host_fs_write` |
| Networking | 12 | `__host_net_tcp_connect`, `__host_http_request` |
| Cryptography | 15 | `__host_crypto_sha256`, `__host_crypto_aes_gcm_encrypt` |
| Graphics | 10 | `__host_gfx_draw_rect`, `__host_gfx_load_texture` |
| Input/Audio | 6 | `__host_input_poll`, `__host_audio_play` |
| Process | 7 | `__host_process_spawn`, `__host_process_wait` |
| Time/Random | 3 | `__host_time_now_ms`, `__host_crypto_random_bytes` |

See `specs/host_bridge_map.md` for complete mappings.

## Usage Examples

### Web Server

```solvrascript
import { create_server, listen, send_json } from <web/server>;
import { create_router, get } from <web/router>;

let router = create_router();
router = get(router, "/api/status", fn(req, res, match) {
    send_json(res, 200, {"status": "ok"});
});

let server = create_server({});
server["handler"] = create_handler(router);
listen(server, 8080);
```

### Game ECS

```solvrascript
import { create_world, create_entity, add_component, run_systems } from <game/ecs>;

let world = create_world();
let player = create_entity(world);
add_component(world, player, "position", {"x": 100, "y": 200});
add_component(world, player, "velocity", {"vx": 5, "vy": 0});

register_system(world, "movement", ["position", "velocity"], fn(world, entity, dt) {
    let pos = get_component(world, entity, "position");
    let vel = get_component(world, entity, "velocity");
    pos["x"] = pos["x"] + vel["vx"] * (dt / 1000.0);
});

run_systems(world, 16);  // 60 FPS
```

### JWT Authentication

```solvrascript
import { sign_jwt, verify_jwt } from <sec/jwt>;

let payload = {"sub": "user123", "role": "admin"};
let token = sign_jwt(payload, "secret_key", "HS256", 3600);

let decoded = verify_jwt(token, "secret_key", "HS256");
if (decoded != null) {
    println("Authenticated: " + decoded["sub"]);
}
```

### CSV Pipeline

```solvrascript
import { read_csv, write_json } from <data/io>;

let rows = read_csv("/data/sales.csv", {"header": true});
let filtered = [];
for (let row in rows) {
    if (int(row[2]) > 100) {  // Quantity > 100
        push(filtered, row);
    }
}
write_json("/output/high_volume.json", filtered, {"pretty": true});
```

## Testing

Run test suites:

```bash
# Run all tests
cargo test -p solvrascript --test stdlib_tests

# Run specific module tests
cargo run -p solvrascript -- stdlib/tests/test_http.svs
cargo run -p solvrascript -- stdlib/tests/test_ecs.svs
cargo run -p solvrascript -- stdlib/tests/test_hash.svs
```

## Building & Running Examples

```bash
# Run HTTP server example
cargo run -p solvrascript -- stdlib/examples/web_min_server.svs

# Run ECS Pong demo
cargo run -p solvrascript -- stdlib/examples/game_ecs_pong.svs

# Run JWT demo
cargo run -p solvrascript -- stdlib/examples/sec_jwt_roundtrip.svs

# Run CSV pipeline
cargo run -p solvrascript -- stdlib/examples/data_csv_pipeline.svs
```

## Documentation

- **Module Docs:** See `docs/*.md` for detailed API documentation
- **Specifications:** See `specs/*.md` for technical specs
- **Language Reference:** See `../docs/language_reference.md`
- **Builtin Status:** See `../docs/builtin_status.md`

---

## 📋 Next Steps for Codex (Implementation Checklist)

### Phase 1: Core Infrastructure (Weeks 1-2)

**1.1 Host Bridge Implementation**
- [ ] Implement filesystem host functions (`__host_fs_*` — 8 functions)
  - [ ] `__host_fs_read` / `__host_fs_write`
  - [ ] `__host_fs_open` / `__host_fs_close`
  - [ ] `__host_fs_stat` with metadata
  - [ ] Path validation and sandboxing
- [ ] Implement time host functions (`__host_time_*` — 2 functions)
  - [ ] `__host_time_now_ms` (monotonic)
  - [ ] `__host_time_now_unix` (system time)
- [ ] Implement random host functions
  - [ ] `__host_crypto_random_bytes` (cryptographically secure)

**1.2 Security Foundation**
- [ ] Implement capability system in SolvraCore
  - [ ] Capability storage and checking
  - [ ] `__host_sandbox_has_cap` / `__host_sandbox_enforce_cap`
  - [ ] Capability inheritance for child contexts
- [ ] Add sandbox integration to all host functions
  - [ ] Check appropriate capability before execution
  - [ ] Return `CapabilityDenied` error on failure

**1.3 Core Module Completion**
- [ ] Complete `.svs` stub implementations (24 remaining)
  - [ ] Web: server, router, ws, tpl, static, utils (6 modules)
  - [ ] Game: scene, input, time, sprite, physics2d, audio, utils (7 modules)
  - [ ] Security: aead, kdf, jwt, pki, sandbox, fuzz (6 modules)
  - [ ] Supporting: runner, io, 2d, kv, wrap (5 modules)
- [ ] Implement pure SolvraScript modules (no host bridge needed)
  - [ ] `<web/tpl>` — Template engine
  - [ ] `<web/utils>` — URL parsing, HTML escaping
  - [ ] `<game/physics2d>` — 2D physics math
  - [ ] `<game/utils>` — Math helpers

### Phase 2: Networking & HTTP (Week 3)

**2.1 HTTP Client**
- [ ] Implement HTTP host functions
  - [ ] `__host_http_request` with timeout/retry
  - [ ] `__host_net_dns_resolve` with caching
  - [ ] TLS validation (always enabled)
- [ ] Complete `<web/http>` implementation
  - [ ] Test GET, POST, PUT, DELETE
  - [ ] Verify timeout enforcement
  - [ ] Validate retry logic
  - [ ] Test JSON parsing helpers

**2.2 HTTP Server**
- [ ] Implement server host functions
  - [ ] `__host_server_create` / `__host_server_listen`
  - [ ] `__host_server_accept` (non-blocking)
  - [ ] `__host_server_read_request` / `__host_server_write_response`
- [ ] Complete `<web/server>` and `<web/router>`
  - [ ] Test request routing
  - [ ] Verify middleware chain execution
  - [ ] Test concurrent connections

**2.3 Raw Sockets**
- [ ] Implement socket host functions (10 functions)
  - [ ] TCP: connect, send, recv, close, listen, accept
  - [ ] UDP: bind, send_to, recv_from
- [ ] Complete `<net/sock>` implementation
  - [ ] Test TCP client/server
  - [ ] Test UDP datagram exchange
  - [ ] Verify timeout behavior

### Phase 3: Cryptography (Week 4)

**3.1 Hash Functions**
- [ ] Implement crypto host functions
  - [ ] `__host_crypto_sha256` / `__host_crypto_sha512`
  - [ ] `__host_crypto_blake3` with SIMD acceleration
  - [ ] `__host_crypto_hmac` with constant-time verification
- [ ] Complete `<sec/hash>` implementation
  - [ ] Validate against NIST test vectors
  - [ ] Benchmark performance targets

**3.2 Encryption**
- [ ] Implement AEAD host functions
  - [ ] `__host_crypto_xchacha20_encrypt` / `_decrypt`
  - [ ] `__host_crypto_aes_gcm_encrypt` / `_decrypt`
  - [ ] `__host_crypto_random_bytes` for keys/nonces
- [ ] Complete `<sec/aead>` implementation
  - [ ] Test encryption/decryption roundtrip
  - [ ] Verify AAD authentication
  - [ ] Test tamper detection

**3.3 Key Derivation**
- [ ] Implement KDF host functions
  - [ ] `__host_crypto_scrypt`
  - [ ] `__host_crypto_argon2`
- [ ] Complete `<sec/kdf>` and `<sec/jwt>`
  - [ ] Test password derivation
  - [ ] Test JWT creation/verification
  - [ ] Validate constant-time operations

### Phase 4: Game Infrastructure (Week 5)

**4.1 Input & Time**
- [ ] Implement input host functions
  - [ ] `__host_input_poll` for keyboard/mouse/gamepad
- [ ] Complete `<game/input>` and `<game/time>`
  - [ ] Test key press detection
  - [ ] Test fixed-timestep accuracy

**4.2 Graphics & Sprites**
- [ ] Implement graphics host functions (10 functions)
  - [ ] `__host_gfx_create_canvas` / canvas operations
  - [ ] `__host_gfx_load_texture` / `__host_gfx_draw_quad`
  - [ ] PNG encoding/decoding
- [ ] Complete `<game/sprite>` and `<gfx/2d>`
  - [ ] Test sprite loading and rendering
  - [ ] Test 2D drawing primitives
  - [ ] Test animation playback

### Phase 5: Data & Storage (Week 6)

**5.1 Data I/O**
- [ ] Implement `<data/io>` in pure SolvraScript
  - [ ] CSV parser and writer
  - [ ] JSON streaming support
  - [ ] JSONL processing
- [ ] Test with large files (>10MB)

**5.2 Key-Value Storage**
- [ ] Complete `<storage/kv>` implementation
  - [ ] Append-only journal format
  - [ ] Index building
  - [ ] TTL expiration logic
  - [ ] Compaction algorithm
- [ ] Test persistence and recovery

### Phase 6: Testing & Validation (Week 7)

**6.1 Comprehensive Test Suite**
- [ ] Write unit tests for all 28 modules
  - [ ] Web: 7 test files
  - [ ] Game: 8 test files
  - [ ] Security: 7 test files
  - [ ] Supporting: 6 test files
- [ ] Write integration tests
  - [ ] End-to-end HTTP roundtrip
  - [ ] Game rendering pipeline
  - [ ] Encryption/decryption workflow
  - [ ] Data processing pipeline

**6.2 Performance Validation**
- [ ] Benchmark all modules against targets
  - [ ] HTTP: >10K req/s throughput
  - [ ] Crypto: SHA-256 >500 MB/s
  - [ ] ECS: <10μs query for 1000 entities
  - [ ] Storage: <100μs per key-value operation
- [ ] Profile and optimize bottlenecks

**6.3 Security Audit**
- [ ] Review all capability checks
- [ ] Test sandbox escape attempts
- [ ] Validate constant-time operations
- [ ] Test TLS certificate validation
- [ ] Code review for common vulnerabilities

### Phase 7: Documentation & Polish (Week 8)

**7.1 Complete Documentation**
- [ ] Finalize all module docs
- [ ] Write tutorial series
  - [ ] Building a web API
  - [ ] Creating a 2D game
  - [ ] Secure data processing
- [ ] Create API reference website

**7.2 Example Applications**
- [ ] Build production-ready examples
  - [ ] REST API with database
  - [ ] Real-time multiplayer game
  - [ ] Secure file encryption tool
  - [ ] Data ETL pipeline

**7.3 Curriculum Integration**
- [ ] Create Solvra_Curriculum lessons
  - [ ] Phase 7.5: Web programming
  - [ ] Phase 7.5: Game development
  - [ ] Phase 8.0: Security practices
- [ ] Add interactive exercises
- [ ] Build learning progression

---

## Phase 2 Features (Future)

- Audio playback (`<game/audio>`)
- PKI certificate verification (`<sec/pki>`)
- Fuzzing utilities (`<sec/fuzz>`)
- Process management (`<devops/runner>`)
- Advanced graphics (shaders, 3D)

---

## Contributing

When adding new stdlib modules:

1. Follow **Zobie.format** for all `.svs` files
2. Document in appropriate `docs/*.md` file
3. Add function to `specs/module_index.md`
4. Map host functions in `specs/host_bridge_map.md`
5. Write example usage
6. Create unit tests
7. Update this README

---

## License

Apache License 2.0 — See LICENSE file for details.

---

## Contact

**Author:** Zachariah Obie
**Project:** SolvraOS
**Repository:** https://github.com/Zobie1776/SolvraOS
**Curriculum:** https://github.com/Zobie1776/Solvra_Curriculum
