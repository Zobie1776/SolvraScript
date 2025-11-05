# SolvraScript Standard Library - Complete Module Index

**Author:** Zachariah Obie
**License:** Apache License 2.0
**Status:** Phase 1 - Specification
**Last Updated:** 2025-11-04

## Overview

This document provides a complete alphabetical index of all functions across the SolvraScript standard library, organized by module family. Use this as a quick reference for function lookup and API discovery.

---

## Web Modules (`<web/*>`)

### `<web/http>` - HTTP Client (10 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `delete(url, options)` | Response | 1 |
| `get(url, options)` | Response | 1 |
| `parse_json(response)` | any | 1 |
| `post(url, body, options)` | Response | 1 |
| `put(url, body, options)` | Response | 1 |
| `request(method, url, options)` | Response | 1 |
| `to_json(data)` | string | 1 |
| `with_headers(options, headers)` | map | 1 |
| `with_retries(options, count)` | map | 1 |
| `with_timeout(options, ms)` | map | 1 |

### `<web/server>` - HTTP Server (9 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `create_server(options)` | Server | 1 |
| `get_request_body(request)` | string | 1 |
| `get_request_header(request, name)` | string | 1 |
| `listen(server, port)` | void | 1 |
| `send_error(response, status, message)` | void | 1 |
| `send_json(response, status, data)` | void | 1 |
| `send_response(response, status, body)` | void | 1 |
| `stop(server)` | void | 1 |
| `use_middleware(server, handler)` | Server | 1 |

### `<web/router>` - Request Router (9 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `create_router()` | Router | 1 |
| `delete(router, pattern, handler)` | Router | 1 |
| `get(router, pattern, handler)` | Router | 1 |
| `get_param(match, name)` | string | 1 |
| `get_query(request, name)` | string | 1 |
| `match_request(router, request)` | Match | 1 |
| `post(router, pattern, handler)` | Router | 1 |
| `put(router, pattern, handler)` | Router | 1 |
| `route(router, method, pattern, handler)` | Router | 1 |

### `<web/ws>` - WebSocket (6 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `close(ws)` | void | 1 |
| `connect(url, options)` | WebSocket | 1 |
| `create_ws_server(server, path, handler)` | Server | 1 |
| `is_open(ws)` | bool | 1 |
| `recv(ws, timeout_ms)` | string | 1 |
| `send(ws, message)` | void | 1 |

### `<web/tpl>` - Template Engine (4 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `compile(template)` | Template | 1 |
| `include_partial(template, partial_name, content)` | string | 1 |
| `render(template, data)` | string | 1 |
| `render_string(template_str, data)` | string | 1 |

### `<web/static>` - Static File Serving (4 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `check_if_modified(request, etag)` | bool | 1 |
| `generate_etag(file_path)` | string | 1 |
| `get_mime_type(file_path)` | string | 1 |
| `serve_dir(root_path, options)` | Handler | 1 |

### `<web/utils>` - Web Utilities (9 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `build_cookie(name, value, options)` | string | 1 |
| `build_url(base, params)` | string | 1 |
| `html_escape(text)` | string | 1 |
| `join_path(segments)` | string | 1 |
| `parse_cookies(cookie_header)` | map | 1 |
| `parse_url(url)` | URL | 1 |
| `url_decode(text)` | string | 1 |
| `url_encode(text)` | string | 1 |

**Web Family Total: 51 functions**

---

## Game Modules (`<game/*>`)

### `<game/ecs>` - Entity-Component-System (10 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `add_component(world, entity, type, data)` | void | 1 |
| `create_entity(world)` | EntityID | 1 |
| `create_world()` | World | 1 |
| `destroy_entity(world, entity)` | void | 1 |
| `get_component(world, entity, type)` | map | 1 |
| `has_component(world, entity, type)` | bool | 1 |
| `query(world, components)` | [EntityID] | 1 |
| `register_system(world, name, components, fn)` | void | 1 |
| `remove_component(world, entity, type)` | void | 1 |
| `run_systems(world, delta_ms)` | void | 1 |

### `<game/scene>` - Scene Management (8 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `add_node(scene, parent, data)` | NodeID | 1 |
| `create_scene(name)` | Scene | 1 |
| `get_children(scene, node)` | [NodeID] | 1 |
| `get_node(scene, node)` | map | 1 |
| `remove_node(scene, node)` | void | 1 |
| `set_active(scene, active)` | void | 1 |
| `transition_to(from, to, transition, duration_ms)` | void | 1 |
| `update_scene(scene, delta_ms)` | void | 1 |

### `<game/input>` - Input Handling (8 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `get_gamepad_axis(pad, axis)` | float | 2 |
| `get_mouse_pos()` | {x, y} | 1 |
| `is_gamepad_button(pad, button)` | bool | 2 |
| `is_key_just_pressed(key)` | bool | 1 |
| `is_key_pressed(key)` | bool | 1 |
| `is_key_released(key)` | bool | 1 |
| `is_mouse_pressed(button)` | bool | 1 |
| `update_input()` | void | 1 |

### `<game/time>` - Time Management (7 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `consume_step(fixed)` | void | 1 |
| `create_fixed_step(step_ms)` | FixedStep | 1 |
| `create_timer()` | Timer | 1 |
| `get_elapsed_ms(timer)` | int | 1 |
| `get_fps(timer)` | int | 1 |
| `should_update(fixed, delta_ms)` | bool | 1 |
| `tick(timer)` | int | 1 |

### `<game/sprite>` - 2D Sprite Rendering (7 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `create_animation(texture, frames, frame_ms)` | Animation | 1 |
| `draw_sprite(texture, x, y, width, height)` | void | 1 |
| `draw_sprite_region(texture, src, dst)` | void | 1 |
| `load_texture(path)` | TextureID | 1 |
| `play_animation(anim, x, y, elapsed_ms)` | void | 1 |
| `set_sprite_flip(flip_x, flip_y)` | void | 1 |
| `unload_texture(texture)` | void | 1 |

### `<game/physics2d>` - 2D Physics (8 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `apply_impulse(body, fx, fy)` | void | 1 |
| `check_collision(a, b)` | bool | 1 |
| `create_body(x, y, width, height)` | Body | 1 |
| `get_collision_info(a, b)` | CollisionInfo | 1 |
| `resolve_collision(a, b, restitution)` | void | 1 |
| `set_mass(body, mass)` | void | 1 |
| `set_velocity(body, vx, vy)` | void | 1 |
| `update_body(body, delta_ms)` | void | 1 |

### `<game/audio>` - Audio Playback (6 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `load_sound(path)` | SoundID | 2 |
| `play_music(sound, volume, loop)` | void | 2 |
| `play_sound(sound, volume)` | void | 2 |
| `set_volume(volume)` | void | 2 |
| `stop_music()` | void | 2 |
| `unload_sound(sound)` | void | 2 |

### `<game/utils>` - Game Utilities (9 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `angle_between(x1, y1, x2, y2)` | float | 1 |
| `clamp(value, min, max)` | float | 1 |
| `distance(x1, y1, x2, y2)` | float | 1 |
| `ease_in_out(t)` | float | 1 |
| `lerp(a, b, t)` | float | 1 |
| `random_choice(list)` | any | 1 |
| `random_float(min, max)` | float | 1 |
| `random_int(min, max)` | int | 1 |
| `seed_rng(seed)` | void | 1 |

**Game Family Total: 63 functions**

---

## Security/Crypto Modules (`<sec/*>`)

### `<sec/hash>` - Cryptographic Hashing (9 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `blake3(data)` | string | 1 |
| `blake3_bytes(data)` | bytes | 1 |
| `hmac_sha256(key, message)` | string | 1 |
| `hmac_sha512(key, message)` | string | 1 |
| `sha256(data)` | string | 1 |
| `sha256_bytes(data)` | bytes | 1 |
| `sha512(data)` | string | 1 |
| `sha512_bytes(data)` | bytes | 1 |
| `verify_hmac(expected, key, message, algo)` | bool | 1 |

### `<sec/aead>` - Authenticated Encryption (8 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `decrypt(algo, key, nonce, ciphertext, aad)` | string | 1 |
| `decrypt_aes_gcm(key, nonce, ciphertext)` | string | 1 |
| `decrypt_xchacha(key, nonce, ciphertext)` | string | 1 |
| `encrypt(algo, key, nonce, plaintext, aad)` | bytes | 1 |
| `encrypt_aes_gcm(key, nonce, plaintext)` | bytes | 1 |
| `encrypt_xchacha(key, nonce, plaintext)` | bytes | 1 |
| `generate_key(algo)` | bytes | 1 |
| `generate_nonce(algo)` | bytes | 1 |

### `<sec/kdf>` - Key Derivation (5 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `derive_key(password, salt, algo, options)` | bytes | 1 |
| `derive_key_argon2(password, salt, variant, mem_kb, time, parallelism, keylen)` | bytes | 1 |
| `derive_key_scrypt(password, salt, n, r, p, keylen)` | bytes | 1 |
| `generate_salt(size)` | bytes | 1 |
| `verify_password(password, salt, expected_key, algo, options)` | bool | 1 |

### `<sec/jwt>` - JSON Web Tokens (5 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `decode_jwt(token)` | map | 1 |
| `sign_jwt(payload, secret, algo, expires_sec)` | string | 1 |
| `sign_jwt_rs256(payload, private_key, expires_sec)` | string | 1 |
| `verify_jwt(token, secret, algo)` | map | 1 |
| `verify_jwt_rs256(token, public_key)` | map | 1 |

### `<sec/pki>` - Public Key Infrastructure (7 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `get_cert_expiry(cert)` | int | 2 |
| `get_cert_issuer(cert)` | string | 2 |
| `get_cert_subject(cert)` | string | 2 |
| `get_public_key(cert)` | string | 2 |
| `is_cert_expired(cert)` | bool | 2 |
| `parse_pem_cert(pem)` | Certificate | 2 |
| `verify_cert_chain(cert, ca_certs)` | bool | 2 |

### `<sec/sandbox>` - Capability Sandbox (7 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `create_sandbox(allowed_caps)` | Sandbox | 1 |
| `drop_capability(cap)` | void | 1 |
| `enforce_capability(cap)` | void | 1 |
| `has_capability(cap)` | bool | 1 |
| `list_capabilities()` | [string] | 1 |
| `request_capability(cap, reason)` | bool | 1 |
| `run_sandboxed(sandbox, fn)` | any | 1 |

### `<sec/fuzz>` - Fuzzing Utilities (4 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `fuzz_test(fn, iterations, input_gen)` | FuzzResult | 2 |
| `generate_random_bytes(len)` | bytes | 2 |
| `generate_random_string(min_len, max_len)` | string | 2 |
| `mutate_string(input, mutations)` | string | 2 |

**Security/Crypto Family Total: 45 functions**

---

## Supporting Modules

### `<net/sock>` - Network Sockets (10 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `dns_resolve(hostname)` | string | 1 |
| `tcp_accept(server, timeout_ms)` | Socket | 1 |
| `tcp_close(socket)` | void | 1 |
| `tcp_connect(host, port, timeout_ms)` | Socket | 1 |
| `tcp_listen(port, backlog)` | ServerSocket | 1 |
| `tcp_recv(socket, max_bytes, timeout_ms)` | bytes | 1 |
| `tcp_send(socket, data)` | int | 1 |
| `udp_bind(port)` | UDPSocket | 1 |
| `udp_recv_from(socket, max_bytes, timeout_ms)` | {data, host, port} | 1 |
| `udp_send_to(socket, data, host, port)` | int | 1 |

### `<devops/runner>` - Process Management (7 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `is_running(proc)` | bool | 2 |
| `kill_process(proc, signal)` | void | 2 |
| `read_stderr(proc, timeout_ms)` | string | 2 |
| `read_stdout(proc, timeout_ms)` | string | 2 |
| `spawn_process(cmd, args, options)` | Process | 2 |
| `wait_process(proc, timeout_ms)` | int | 2 |
| `write_stdin(proc, data)` | void | 2 |

### `<data/io>` - Data I/O (8 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `read_csv(path, options)` | [[string]] | 1 |
| `read_json(path)` | any | 1 |
| `read_jsonl(path)` | [any] | 1 |
| `stream_csv(path, batch_size, handler)` | void | 1 |
| `validate_schema(data, schema)` | bool | 2 |
| `write_csv(path, rows, options)` | void | 1 |
| `write_json(path, data, options)` | void | 1 |
| `write_jsonl(path, objects)` | void | 1 |

### `<gfx/2d>` - 2D Graphics (10 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `blit_sprite(canvas, sprite, x, y)` | void | 1 |
| `clear_canvas(canvas, color)` | void | 1 |
| `create_canvas(width, height)` | Canvas | 1 |
| `draw_circle(canvas, cx, cy, radius, color, filled)` | void | 1 |
| `draw_line(canvas, x1, y1, x2, y2, color)` | void | 1 |
| `draw_pixel(canvas, x, y, color)` | void | 1 |
| `draw_rect(canvas, x, y, w, h, color, filled)` | void | 1 |
| `draw_text(canvas, x, y, text, color)` | void | 1 |
| `load_canvas(path)` | Canvas | 1 |
| `save_canvas(canvas, path)` | void | 1 |

### `<storage/kv>` - Key-Value Storage (9 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `clear(store)` | void | 1 |
| `close_store(store)` | void | 1 |
| `compact(store)` | void | 1 |
| `delete(store, key)` | void | 1 |
| `exists(store, key)` | bool | 1 |
| `get(store, key)` | string | 1 |
| `keys(store)` | [string] | 1 |
| `open_store(path)` | Store | 1 |
| `set(store, key, value, ttl_sec)` | void | 1 |

### `<crypto/wrap>` - Crypto Wrappers (6 functions)

| Function | Returns | Phase |
|----------|---------|-------|
| `decrypt_file(input_path, output_path, password)` | void | 2 |
| `encrypt_file(input_path, output_path, password)` | void | 2 |
| `hash_file(file_path, algo)` | string | 1 |
| `secure_delete(file_path, passes)` | void | 2 |
| `sign_file(file_path, key_path)` | string | 2 |
| `verify_file(file_path, signature, key_path)` | bool | 2 |

**Supporting Family Total: 50 functions**

---

## Summary Statistics

| Family | Modules | Functions | Phase 1 | Phase 2 |
|--------|---------|-----------|---------|---------|
| Web | 7 | 51 | 51 | 0 |
| Game | 8 | 63 | 57 | 6 |
| Security | 7 | 45 | 34 | 11 |
| Supporting | 6 | 50 | 40 | 10 |
| **TOTAL** | **28** | **209** | **182** | **27** |

## Implementation Phases

**Phase 1 (Core):** 182 functions
- Essential functions for web, game, security, and data operations
- No external dependencies beyond host bridge
- Target: Complete by Phase 7.5

**Phase 2 (Extended):** 27 functions
- Advanced features (audio, PKI, fuzzing, process management)
- May require additional host capabilities
- Target: Complete by Phase 8.0

## Function Naming Conventions

- **Verbs:** `create_*`, `get_*`, `set_*`, `is_*`, `has_*`
- **Prefixes:**
  - `create_` — Constructor functions
  - `get_` — Accessor functions
  - `set_` — Mutator functions
  - `is_` — Boolean predicates (state)
  - `has_` — Boolean predicates (presence)
- **Suffixes:**
  - `_bytes` — Raw binary output
  - `_string` — String output
  - `_file` — File-based operation

## Cross-Module Integration Points

```
<web/http> ← <web/utils> (URL parsing)
<web/server> ← <web/router> (routing)
<web/static> ← <sec/hash> (ETags)
<game/sprite> ← <gfx/2d> (rendering backend)
<crypto/wrap> ← <sec/*> (primitives)
All modules → <sec/sandbox> (permissions)
```

## Next Steps

1. Implement Phase 1 functions (182 total)
2. Create comprehensive test suite
3. Write integration examples
4. Benchmark performance targets
5. Document host bridge requirements
