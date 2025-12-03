# SolvraScript Standard Library - Host Bridge Map

**Author:** Zachariah Obie
**License:** Apache License 2.0
**Status:** Phase 1 - Specification
**Last Updated:** 2025-11-04

## Overview

This document maps SolvraScript stdlib functions to their required host (SolvraCore) bridge functions. The host bridge provides low-level system operations that cannot be implemented in pure SolvraScript (network I/O, file access, cryptography, etc.).

All host functions follow the naming convention: `__host_<domain>_<operation>`

---

## Bridge Architecture

```
┌─────────────────────────────────┐
│   SolvraScript User Code        │
│   (Pure .svs functions)         │
└────────────┬────────────────────┘
             │
             │ import <module>
             ▼
┌─────────────────────────────────┐
│   SolvraScript Stdlib           │
│   (.svs modules + wrappers)     │
└────────────┬────────────────────┘
             │
             │ __host_* calls
             ▼
┌─────────────────────────────────┐
│   SolvraCore Host Bridge        │
│   (Rust FFI implementation)     │
└────────────┬────────────────────┘
             │
             │ System calls
             ▼
┌─────────────────────────────────┐
│   Operating System / Hardware   │
└─────────────────────────────────┘
```

---

## Host Function Categories

### Pure SolvraScript (No Host Functions)

These modules are implemented entirely in `.svs` without host bridge requirements:

- `<web/tpl>` — Template engine (string manipulation)
- `<web/utils>` — URL parsing, HTML escaping (pure logic)
- `<game/ecs>` — Entity-component-system (data structures)
- `<game/scene>` — Scene graph (tree operations)
- `<game/physics2d>` — 2D physics (math operations)
- `<game/utils>` — Math helpers (lerp, easing, etc.)
- `<sec/sandbox>` — Capability checks (VM introspection)

**Implementation:** Write directly in `.svs` with no external dependencies.

---

## Web Modules Host Functions

### `<web/http>` - HTTP Client

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `get()` | `__host_http_request` | `(method: str, url: str, headers: map, body: bytes, timeout_ms: int)` | `{status: int, headers: map, body: bytes, time_ms: int}` |
| `post()` | `__host_http_request` | Same as above | Same as above |
| `put()` | `__host_http_request` | Same as above | Same as above |
| `delete()` | `__host_http_request` | Same as above | Same as above |
| `request()` | `__host_http_request` | Same as above | Same as above |

**Host Implementation Notes:**
- Use `reqwest` or `ureq` crate for Rust implementation
- TLS validation always enabled
- Follow redirects (max 5)
- Enforce timeout strictly
- Return timing information

**Error Codes:**
- `1001` — Connection failed
- `1002` — Timeout
- `1003` — Invalid URL
- `1004` — TLS error

---

### `<web/server>` - HTTP Server

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `create_server()` | `__host_server_create` | `(port: int)` | `server_fd: int` |
| `listen()` | `__host_server_listen` | `(server_fd: int, port: int)` | `void` |
| `stop()` | `__host_server_close` | `(server_fd: int)` | `void` |
| (internal) | `__host_server_accept` | `(server_fd: int, timeout_ms: int)` | `{client_fd: int, remote_addr: str}` |
| (internal) | `__host_server_read_request` | `(client_fd: int, timeout_ms: int)` | `{method: str, path: str, query: map, headers: map, body: bytes}` |
| (internal) | `__host_server_write_response` | `(client_fd: int, status: int, headers: map, body: bytes)` | `void` |

**Host Implementation Notes:**
- Use `hyper` or `tiny_http` crate
- Non-blocking I/O with tokio runtime
- Request/response parsing
- Connection pooling

**Error Codes:**
- `2001` — Port in use
- `2002` — Permission denied (privileged port)
- `2003` — Invalid config

---

### `<web/ws>` - WebSocket

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `connect()` | `__host_ws_connect` | `(url: str, timeout_ms: int)` | `ws_fd: int` |
| `send()` | `__host_ws_send` | `(ws_fd: int, message: str)` | `void` |
| `recv()` | `__host_ws_recv` | `(ws_fd: int, timeout_ms: int)` | `message: str` |
| `close()` | `__host_ws_close` | `(ws_fd: int)` | `void` |
| `create_ws_server()` | `__host_ws_upgrade` | `(server_fd: int, path: str)` | `ws_fd: int` |

**Host Implementation Notes:**
- Use `tungstenite` crate
- Automatic ping/pong for keepalive
- Message framing handled by host

**Error Codes:**
- `3001` — Connection failed
- `3002` — Send failed
- `3003` — Connection closed

---

### `<web/static>` - Static File Serving

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `serve_dir()` | `__host_fs_read` | `(path: str)` | `bytes` |
| `generate_etag()` | `__host_hash_sha256` | `(data: bytes)` | `hash: str` |

**Host Implementation Notes:**
- Reuses filesystem and crypto host functions
- MIME type detection via stdlib (no host function)

---

## Game Modules Host Functions

### `<game/input>` - Input Handling

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `update_input()` | `__host_input_poll` | `()` | `{keys: map, mouse: map, gamepads: [map]}` |
| `is_key_pressed()` | (uses cached state) | — | — |
| `get_mouse_pos()` | (uses cached state) | — | — |
| `get_gamepad_axis()` | (uses cached state) | — | — |

**Host Implementation Notes:**
- Use `winit` or SDL2 for input handling
- Poll once per frame, cache results
- Key names standardized (QWERTY layout)

---

### `<game/time>` - Time Management

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `create_timer()` | `__host_time_now_ms` | `()` | `timestamp: int` |
| `tick()` | `__host_time_now_ms` | `()` | `timestamp: int` |

**Host Implementation Notes:**
- Use `std::time::Instant` for monotonic time
- Microsecond precision

---

### `<game/sprite>` - 2D Sprite Rendering

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `load_texture()` | `__host_gfx_load_texture` | `(path: str)` | `texture_id: int` |
| `unload_texture()` | `__host_gfx_unload_texture` | `(texture_id: int)` | `void` |
| `draw_sprite()` | `__host_gfx_draw_quad` | `(texture_id: int, src: rect, dst: rect, flip_x: bool, flip_y: bool)` | `void` |
| `draw_sprite_region()` | `__host_gfx_draw_quad` | Same as above | Same as above |

**Host Implementation Notes:**
- Use `wgpu` or `OpenGL` for rendering
- Texture atlas support
- Batch rendering for performance

---

### `<game/audio>` - Audio Playback

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `load_sound()` | `__host_audio_load` | `(path: str)` | `sound_id: int` |
| `unload_sound()` | `__host_audio_unload` | `(sound_id: int)` | `void` |
| `play_sound()` | `__host_audio_play` | `(sound_id: int, volume: float, loop: bool)` | `void` |
| `stop_music()` | `__host_audio_stop` | `()` | `void` |
| `set_volume()` | `__host_audio_set_volume` | `(volume: float)` | `void` |

**Host Implementation Notes:**
- Use `rodio` crate for audio playback
- Support WAV, OGG, MP3 formats
- Mixing up to 16 simultaneous sounds

---

## Security/Crypto Modules Host Functions

### `<sec/hash>` - Cryptographic Hashing

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `sha256()` | `__host_crypto_sha256` | `(data: bytes)` | `hash: bytes` |
| `sha512()` | `__host_crypto_sha512` | `(data: bytes)` | `hash: bytes` |
| `blake3()` | `__host_crypto_blake3` | `(data: bytes)` | `hash: bytes` |
| `hmac_sha256()` | `__host_crypto_hmac` | `(key: bytes, message: bytes, algo: str)` | `mac: bytes` |
| `hmac_sha512()` | `__host_crypto_hmac` | Same as above | Same as above |

**Host Implementation Notes:**
- Use `sha2`, `blake3`, `hmac` crates
- Hardware acceleration (SHA-NI) when available
- Constant-time verification

---

### `<sec/aead>` - Authenticated Encryption

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `generate_key()` | `__host_crypto_random_bytes` | `(count: int)` | `bytes` |
| `generate_nonce()` | `__host_crypto_random_bytes` | `(count: int)` | `bytes` |
| `encrypt_xchacha()` | `__host_crypto_xchacha20_encrypt` | `(key: bytes, nonce: bytes, plaintext: bytes, aad: bytes)` | `ciphertext: bytes` |
| `decrypt_xchacha()` | `__host_crypto_xchacha20_decrypt` | `(key: bytes, nonce: bytes, ciphertext: bytes, aad: bytes)` | `plaintext: bytes` |
| `encrypt_aes_gcm()` | `__host_crypto_aes_gcm_encrypt` | Same interface | Same return |
| `decrypt_aes_gcm()` | `__host_crypto_aes_gcm_decrypt` | Same interface | Same return |

**Host Implementation Notes:**
- Use `chacha20poly1305` and `aes-gcm` crates
- Hardware AES-NI when available
- Constant-time tag verification

**Error Codes:**
- `4001` — Encryption failed
- `4002` — Decryption failed
- `4003` — Authentication failed

---

### `<sec/kdf>` - Key Derivation

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `derive_key_scrypt()` | `__host_crypto_scrypt` | `(password: str, salt: bytes, n: int, r: int, p: int, keylen: int)` | `key: bytes` |
| `derive_key_argon2()` | `__host_crypto_argon2` | `(password: str, salt: bytes, variant: str, mem_kb: int, time: int, parallelism: int, keylen: int)` | `key: bytes` |
| `generate_salt()` | `__host_crypto_random_bytes` | `(count: int)` | `bytes` |

**Host Implementation Notes:**
- Use `scrypt` and `argon2` crates
- Configurable cost parameters
- Memory and time limits enforced

---

### `<sec/jwt>` - JSON Web Tokens

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `sign_jwt()` (HS256) | `__host_crypto_hmac` | `(key: bytes, message: bytes, algo: str)` | `signature: bytes` |
| `verify_jwt()` (HS256) | `__host_crypto_hmac` | Same as above | Same as above |
| `sign_jwt_rs256()` | `__host_crypto_rsa_sign` | `(private_key: str, message: bytes)` | `signature: bytes` |
| `verify_jwt_rs256()` | `__host_crypto_rsa_verify` | `(public_key: str, message: bytes, signature: bytes)` | `valid: bool` |

**Host Implementation Notes:**
- Use `hmac` and `rsa` crates
- Base64url encoding handled in `.svs`
- Timestamp validation in `.svs`

---

### `<sec/pki>` - Public Key Infrastructure

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `parse_pem_cert()` | `__host_crypto_parse_x509` | `(pem: str)` | `{version: int, serial: str, subject: str, issuer: str, not_before: int, not_after: int, public_key: str, signature_algo: str}` |
| `verify_cert_chain()` | `__host_crypto_verify_signature` | `(cert: map, ca_cert: map)` | `valid: bool` |

**Host Implementation Notes:**
- Use `x509-parser` or `rustls` crate
- X.509 v3 certificates only
- Basic chain verification (no CRL/OCSP)

---

## Supporting Modules Host Functions

### `<net/sock>` - Network Sockets

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `tcp_connect()` | `__host_net_tcp_connect` | `(host: str, port: int, timeout_ms: int)` | `fd: int` |
| `tcp_send()` | `__host_net_tcp_send` | `(fd: int, data: bytes)` | `sent: int` |
| `tcp_recv()` | `__host_net_tcp_recv` | `(fd: int, max_bytes: int, timeout_ms: int)` | `data: bytes` |
| `tcp_close()` | `__host_net_tcp_close` | `(fd: int)` | `void` |
| `tcp_listen()` | `__host_net_tcp_listen` | `(port: int, backlog: int)` | `fd: int` |
| `tcp_accept()` | `__host_net_tcp_accept` | `(fd: int, timeout_ms: int)` | `{client_fd: int, remote_addr: str}` |
| `udp_bind()` | `__host_net_udp_bind` | `(port: int)` | `fd: int` |
| `udp_send_to()` | `__host_net_udp_send_to` | `(fd: int, data: bytes, host: str, port: int)` | `sent: int` |
| `udp_recv_from()` | `__host_net_udp_recv_from` | `(fd: int, max_bytes: int, timeout_ms: int)` | `{data: bytes, host: str, port: int}` |
| `dns_resolve()` | `__host_net_dns_resolve` | `(hostname: str)` | `ip: str` |

**Host Implementation Notes:**
- Use `tokio::net` for async I/O
- File descriptor management
- Non-blocking operations with timeout

**Error Codes:**
- `5001` — Connection failed
- `5002` — Bind failed
- `5003` — DNS resolution failed
- `5004` — Timeout

---

### `<devops/runner>` - Process Management

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `spawn_process()` | `__host_process_spawn` | `(cmd: str, args: [str], env: map, cwd: str)` | `pid: int` |
| `wait_process()` | `__host_process_wait` | `(pid: int, timeout_ms: int)` | `exit_code: int` |
| `kill_process()` | `__host_process_kill` | `(pid: int, signal: int)` | `void` |
| `read_stdout()` | `__host_process_read_stdout` | `(pid: int, timeout_ms: int)` | `data: str` |
| `read_stderr()` | `__host_process_read_stderr` | `(pid: int, timeout_ms: int)` | `data: str` |
| `write_stdin()` | `__host_process_write_stdin` | `(pid: int, data: str)` | `void` |
| `is_running()` | `__host_process_is_running` | `(pid: int)` | `running: bool` |

**Host Implementation Notes:**
- Use `tokio::process` for async process management
- Pipe management for stdin/stdout/stderr
- Environment variable sanitization
- Working directory validation

**Error Codes:**
- `6001` — Spawn failed
- `6002` — Process not found
- `6003` — Wait timeout

---

### `<data/io>` - Data I/O

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `read_csv()` | `__host_fs_read` | `(path: str)` | `bytes` |
| `write_csv()` | `__host_fs_write` | `(path: str, data: bytes)` | `void` |
| `read_json()` | `__host_fs_read` | `(path: str)` | `bytes` |
| `write_json()` | `__host_fs_write` | `(path: str, data: bytes)` | `void` |
| `read_jsonl()` | `__host_fs_read` | `(path: str)` | `bytes` |
| `write_jsonl()` | `__host_fs_write` | `(path: str, data: bytes)` | `void` |

**Host Implementation Notes:**
- CSV/JSON parsing done in `.svs` (pure implementation)
- Host only provides file I/O primitives

---

### `<gfx/2d>` - 2D Graphics

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `create_canvas()` | `__host_gfx_create_canvas` | `(width: int, height: int)` | `canvas_id: int` |
| `draw_pixel()` | `__host_gfx_set_pixel` | `(canvas_id: int, x: int, y: int, color: int)` | `void` |
| `draw_line()` | `__host_gfx_draw_line` | `(canvas_id: int, x1: int, y1: int, x2: int, y2: int, color: int)` | `void` |
| `draw_rect()` | `__host_gfx_draw_rect` | `(canvas_id: int, x: int, y: int, w: int, h: int, color: int, filled: bool)` | `void` |
| `draw_circle()` | `__host_gfx_draw_circle` | `(canvas_id: int, cx: int, cy: int, radius: int, color: int, filled: bool)` | `void` |
| `draw_text()` | `__host_gfx_draw_text` | `(canvas_id: int, x: int, y: int, text: str, color: int)` | `void` |
| `blit_sprite()` | `__host_gfx_blit` | `(dest_canvas: int, src_canvas: int, x: int, y: int)` | `void` |
| `save_canvas()` | `__host_gfx_save_png` | `(canvas_id: int, path: str)` | `void` |
| `load_canvas()` | `__host_gfx_load_png` | `(path: str)` | `canvas_id: int` |

**Host Implementation Notes:**
- Use `image` crate for PNG encoding/decoding
- Software rendering (CPU-based)
- Framebuffer stored as RGBA u8 array

---

### `<storage/kv>` - Key-Value Storage

| Stdlib Function | Host Function | Signature | Returns |
|----------------|---------------|-----------|---------|
| `open_store()` | `__host_fs_open` | `(path: str, mode: str)` | `fd: int` |
| `close_store()` | `__host_fs_close` | `(fd: int)` | `void` |
| `get()` | `__host_fs_read` | `(fd: int, offset: int, size: int)` | `bytes` |
| `set()` | `__host_fs_write` | `(fd: int, offset: int, data: bytes)` | `void` |
| (internal) | `__host_fs_sync` | `(fd: int)` | `void` |
| (internal) | `__host_time_now_unix` | `()` | `timestamp: int` |

**Host Implementation Notes:**
- Key-value logic implemented in `.svs`
- Host provides file I/O primitives
- Append-only journal with index

---

## Filesystem Host Functions (Common)

Used by multiple modules (`<data/io>`, `<storage/kv>`, `<web/static>`, etc.):

| Host Function | Signature | Returns | Used By |
|---------------|-----------|---------|---------|
| `__host_fs_read` | `(path: str)` | `bytes` | data/io, web/static, crypto/wrap |
| `__host_fs_write` | `(path: str, data: bytes)` | `void` | data/io, storage/kv, crypto/wrap |
| `__host_fs_open` | `(path: str, mode: str)` | `fd: int` | storage/kv |
| `__host_fs_read_at` | `(fd: int, offset: int, size: int)` | `bytes` | storage/kv |
| `__host_fs_write_at` | `(fd: int, offset: int, data: bytes)` | `void` | storage/kv |
| `__host_fs_close` | `(fd: int)` | `void` | storage/kv |
| `__host_fs_sync` | `(fd: int)` | `void` | storage/kv |
| `__host_fs_stat` | `(path: str)` | `{size: int, modified: int, is_dir: bool}` | web/static |

**Error Codes:**
- `7001` — File not found
- `7002` — Permission denied
- `7003` — I/O error

---

## Time Host Functions (Common)

| Host Function | Signature | Returns | Used By |
|---------------|-----------|---------|---------|
| `__host_time_now_ms` | `()` | `timestamp: int` | game/time |
| `__host_time_now_unix` | `()` | `timestamp: int` | storage/kv, sec/pki |

---

## Random Host Functions (Common)

| Host Function | Signature | Returns | Used By |
|---------------|-----------|---------|---------|
| `__host_crypto_random_bytes` | `(count: int)` | `bytes` | sec/aead, sec/kdf, sec/fuzz, game/utils (unseeded) |

---

## Summary Statistics

| Category | Host Functions | Pure .svs | Total |
|----------|----------------|-----------|-------|
| Web | 12 | 3 | 15 |
| Game | 8 | 5 | 13 |
| Security | 12 | 1 | 13 |
| Supporting | 40 | 1 | 41 |
| **TOTAL** | **72** | **10** | **82** |

---

## Implementation Priority

### Phase 1 (Critical Path)
1. Filesystem (`__host_fs_*`) — 8 functions
2. Crypto primitives (`__host_crypto_*`) — 15 functions
3. Network (`__host_net_*`, `__host_http_*`) — 12 functions
4. Time (`__host_time_*`) — 2 functions

### Phase 2 (Enhanced Features)
1. Graphics (`__host_gfx_*`) — 10 functions
2. Input (`__host_input_*`) — 1 function
3. Audio (`__host_audio_*`) — 5 functions
4. Process (`__host_process_*`) — 7 functions

---

## Testing Strategy

### Host Function Tests
Each host function requires:
1. Unit tests in Rust
2. Integration tests from `.svs`
3. Error handling validation
4. Performance benchmarks

### Example Test Structure
```rust
#[test]
fn test_host_fs_read() {
    let path = "/tmp/test_file.txt";
    std::fs::write(path, b"Hello, SolvraOS!").unwrap();

    let result = __host_fs_read(path);
    assert_eq!(result, b"Hello, SolvraOS!");
}
```

---

**End of Host Bridge Map**
