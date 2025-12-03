# SolvraScript Supporting Standard Library

**Author:** Zachariah Obie
**License:** Apache License 2.0
**Status:** Phase 1 - Design & Specification
**Last Updated:** 2025-11-04

## Overview

The Supporting standard library provides networking (TCP/UDP/DNS), process management, data I/O (CSV/JSON), 2D graphics primitives, key-value storage, and high-level crypto wrappers. These modules complement the core Web, Game, and Security families with essential infrastructure functionality.

## Module Taxonomy & Imports

### Standard Library Import Syntax

```solvrascript
// Import entire module
import <net/sock>;
import <devops/runner>;
import <data/io>;
import <gfx/2d>;
import <storage/kv>;
import <crypto/wrap>;

// Import specific functions
import { tcp_connect, tcp_send, tcp_recv } from <net/sock>;
import { spawn_process, wait_process } from <devops/runner>;
import { read_csv, write_json } from <data/io>;
```

### Module Hierarchy

```
Supporting Modules:
├── net/sock       # TCP/UDP clients, DNS
├── devops/runner  # Process spawning with env + cwd
├── data/io        # CSV/JSON/JSONL read/write
├── gfx/2d         # Framebuffer-style draw API
├── storage/kv     # Key-value store with TTL
└── crypto/wrap    # Friendly crypto wrappers
```

---

## Module: `<net/sock>` - Network Sockets

### Purpose
Provides low-level TCP/UDP socket operations and DNS resolution for custom network protocols.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `tcp_connect` | `tcp_connect(host: string, port: int, timeout_ms: int) -> Socket` | TCP socket | `ConnectionFailed`, `TimeoutError` |
| `tcp_send` | `tcp_send(socket: Socket, data: bytes) -> int` | Bytes sent | `SendFailed`, `ConnectionClosed` |
| `tcp_recv` | `tcp_recv(socket: Socket, max_bytes: int, timeout_ms: int) -> bytes` | Received data | `RecvFailed`, `TimeoutError` |
| `tcp_close` | `tcp_close(socket: Socket) -> void` | None | None |
| `tcp_listen` | `tcp_listen(port: int, backlog: int) -> ServerSocket` | Server socket | `BindFailed`, `PortInUse` |
| `tcp_accept` | `tcp_accept(server: ServerSocket, timeout_ms: int) -> Socket` | Client socket | `TimeoutError` |
| `udp_bind` | `udp_bind(port: int) -> UDPSocket` | UDP socket | `BindFailed` |
| `udp_send_to` | `udp_send_to(socket: UDPSocket, data: bytes, host: string, port: int) -> int` | Bytes sent | `SendFailed` |
| `udp_recv_from` | `udp_recv_from(socket: UDPSocket, max_bytes: int, timeout_ms: int) -> {data: bytes, host: string, port: int}` | Received datagram | `TimeoutError` |
| `dns_resolve` | `dns_resolve(hostname: string) -> string` | IP address | `ResolutionFailed` |

### Socket Object Structure

```solvrascript
{
    fd: int,            // File descriptor
    remote_host: string,
    remote_port: int,
    local_port: int
}
```

### Example Usage

```solvrascript
import { tcp_connect, tcp_send, tcp_recv, tcp_close, dns_resolve } from <net/sock>;

// Resolve hostname
let ip = dns_resolve("example.com");
println("Resolved to: " + ip);

// TCP client
let socket = tcp_connect("example.com", 80, 5000);

let request = "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
tcp_send(socket, request);

let response = tcp_recv(socket, 4096, 5000);
println("Response: " + response);

tcp_close(socket);

// UDP socket
import { udp_bind, udp_send_to, udp_recv_from } from <net/sock>;

let udp_sock = udp_bind(0);  // Bind to random port
udp_send_to(udp_sock, "Hello, UDP!", "127.0.0.1", 8080);

let datagram = udp_recv_from(udp_sock, 1024, 5000);
println("Received: " + datagram.data + " from " + datagram.host);
```

### Determinism & Sandbox Notes

- All network operations are non-deterministic
- Requires `<sec/sandbox>` capabilities:
  - `net.tcp` for TCP operations
  - `net.udp` for UDP operations
  - `net.dns` for DNS resolution
- DNS results are cached per-session (5 minute TTL)
- Maximum concurrent sockets: 100
- Buffer sizes are user-controlled (no automatic buffering)
- TCP_NODELAY enabled by default

### Host Function Needs

- `__host_net_tcp_connect(host, port, timeout_ms) -> fd`
- `__host_net_tcp_send(fd, data) -> int`
- `__host_net_tcp_recv(fd, max_bytes, timeout_ms) -> bytes`
- `__host_net_tcp_close(fd) -> void`
- `__host_net_tcp_listen(port, backlog) -> fd`
- `__host_net_tcp_accept(fd, timeout_ms) -> (client_fd, remote_addr)`
- `__host_net_udp_bind(port) -> fd`
- `__host_net_udp_send_to(fd, data, host, port) -> int`
- `__host_net_udp_recv_from(fd, max_bytes, timeout_ms) -> (data, host, port)`
- `__host_net_dns_resolve(hostname) -> ip_address`

### Performance Targets

- TCP connect: < 100ms (local network)
- TCP send/recv: < 1ms overhead
- UDP send/recv: < 500μs overhead
- DNS resolution: < 50ms (cached: < 1μs)

### Test Plan

1. TCP connection establishment
2. TCP send and receive
3. TCP server listen and accept
4. UDP bind and datagram exchange
5. DNS resolution (A and AAAA records)
6. Timeout enforcement
7. Connection failure handling
8. Sandbox enforcement

### @ZNOTE Rationale

Raw socket access enables custom protocols. Design focuses on:
- **Low-level control**: Direct socket operations
- **Timeout enforcement**: Prevents hanging I/O
- **Flexibility**: Both TCP and UDP support

---

## Module: `<devops/runner>` - Process Management

### Purpose
Provides sandboxed process spawning with environment variable control and working directory specification.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `spawn_process` | `spawn_process(cmd: string, args: [string], options: map) -> Process` | Process handle | `SpawnFailed`, `InvalidCommand` |
| `wait_process` | `wait_process(proc: Process, timeout_ms: int) -> int` | Exit code | `TimeoutError`, `ProcessFailed` |
| `kill_process` | `kill_process(proc: Process, signal: int) -> void` | None | `ProcessNotFound` |
| `read_stdout` | `read_stdout(proc: Process, timeout_ms: int) -> string` | Stdout data | `TimeoutError` |
| `read_stderr` | `read_stderr(proc: Process, timeout_ms: int) -> string` | Stderr data | `TimeoutError` |
| `write_stdin` | `write_stdin(proc: Process, data: string) -> void` | None | `ProcessClosed` |
| `is_running` | `is_running(proc: Process) -> bool` | Process status | None |

### Process Options Structure

```solvrascript
{
    env: map,          // Environment variables (default: inherit)
    cwd: string,       // Working directory (default: current)
    stdin: bool,       // Capture stdin (default: false)
    stdout: bool,      // Capture stdout (default: true)
    stderr: bool       // Capture stderr (default: true)
}
```

### Example Usage

```solvrascript
import { spawn_process, wait_process, read_stdout, read_stderr } from <devops/runner>;

// Run simple command
let proc = spawn_process("ls", ["-la", "/tmp"], {
    "cwd": "/home/user",
    "stdout": true,
    "stderr": true
});

let exit_code = wait_process(proc, 5000);
let output = read_stdout(proc, 1000);

println("Exit code: " + str(exit_code));
println("Output: " + output);

// Run with custom environment
let build_proc = spawn_process("cargo", ["build", "--release"], {
    "env": {"RUSTFLAGS": "-C target-cpu=native", "CARGO_HOME": "/custom/path"},
    "cwd": "/project/dir",
    "stdout": true
});

while (is_running(build_proc)) {
    let line = read_stdout(build_proc, 100);
    if (line != "") {
        println("[BUILD] " + line);
    }
}

// Interactive process with stdin
import { write_stdin } from <devops/runner>;

let interactive = spawn_process("python3", ["-i"], {
    "stdin": true,
    "stdout": true
});

write_stdin(interactive, "print('Hello from SolvraScript')\n");
let result = read_stdout(interactive, 1000);
println(result);
```

### Determinism & Sandbox Notes

- Process execution is non-deterministic
- Requires `<sec/sandbox>` capabilities:
  - `process.spawn` for general process spawning
  - `process.spawn.{command}` for specific command whitelist
  - `process.env` to access/modify environment
- Processes run in isolated security context
- No shell expansion (arguments passed directly)
- Working directory must exist and be accessible
- Environment variables are sanitized (no PATH injection)
- Maximum concurrent processes: 10

### Host Function Needs

- `__host_process_spawn(cmd, args, env, cwd) -> pid`
- `__host_process_wait(pid, timeout_ms) -> exit_code`
- `__host_process_kill(pid, signal) -> void`
- `__host_process_read_stdout(pid, timeout_ms) -> string`
- `__host_process_read_stderr(pid, timeout_ms) -> string`
- `__host_process_write_stdin(pid, data) -> void`
- `__host_process_is_running(pid) -> bool`

### Performance Targets

- Process spawn: < 10ms
- I/O operations: < 1ms overhead
- Wait timeout accuracy: ± 10ms

### Test Plan

1. Process spawning and waiting
2. Stdout/stderr capture
3. Stdin writing
4. Exit code retrieval
5. Timeout enforcement
6. Environment variable passing
7. Working directory changes
8. Process termination (kill)
9. Sandbox enforcement (command whitelist)

### @ZNOTE Rationale

Process spawning enables automation and integration. Design prioritizes:
- **Security**: Sandboxed execution, no shell injection
- **Control**: Environment and working directory configuration
- **Observability**: Stdout/stderr capture

---

## Module: `<data/io>` - Data I/O

### Purpose
Provides CSV/JSON/JSONL file reading and writing with schema validation and batching support.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `read_csv` | `read_csv(path: string, options: map) -> [[string]]` | CSV rows | `FileNotFound`, `ParseError` |
| `write_csv` | `write_csv(path: string, rows: [[string]], options: map) -> void` | None | `WriteError` |
| `read_json` | `read_json(path: string) -> any` | Parsed JSON | `FileNotFound`, `JSONParseError` |
| `write_json` | `write_json(path: string, data: any, options: map) -> void` | None | `WriteError`, `JSONEncodeError` |
| `read_jsonl` | `read_jsonl(path: string) -> [any]` | JSON objects | `FileNotFound`, `ParseError` |
| `write_jsonl` | `write_jsonl(path: string, objects: [any]) -> void` | None | `WriteError` |
| `stream_csv` | `stream_csv(path: string, batch_size: int, handler: fn) -> void` | None | `FileNotFound` |
| `validate_schema` | `validate_schema(data: any, schema: map) -> bool` | Validation result | None |

### CSV Options Structure

```solvrascript
{
    delimiter: string,   // Default: ","
    header: bool,        // Has header row (default: true)
    skip_rows: int       // Rows to skip (default: 0)
}
```

### JSON Options Structure

```solvrascript
{
    pretty: bool,        // Pretty-print (default: false)
    indent: int          // Indent spaces (default: 2)
}
```

### Example Usage

```solvrascript
import { read_csv, write_csv, read_json, write_json } from <data/io>;

// Read CSV file
let rows = read_csv("/data/users.csv", {"header": true, "delimiter": ","});

for (let row in rows) {
    println("User: " + row[0] + ", Email: " + row[1]);
}

// Write CSV file
let new_rows = [
    ["Name", "Age", "City"],
    ["Alice", "30", "NYC"],
    ["Bob", "25", "LA"]
];
write_csv("/tmp/output.csv", new_rows, {});

// Read JSON file
let config = read_json("/etc/app/config.json");
println("Port: " + str(config["port"]));

// Write JSON file with pretty printing
let data = {"users": [{"name": "Alice"}, {"name": "Bob"}]};
write_json("/tmp/output.json", data, {"pretty": true, "indent": 2});

// Stream large CSV in batches
import { stream_csv } from <data/io>;

let total_rows = 0;
stream_csv("/data/large_file.csv", 1000, fn(batch) {
    total_rows = total_rows + len(batch);
    println("Processed " + str(len(batch)) + " rows");
});

println("Total rows: " + str(total_rows));

// Schema validation
import { validate_schema } from <data/io>;

let schema = {
    "type": "object",
    "required": ["name", "email"],
    "properties": {
        "name": {"type": "string"},
        "email": {"type": "string"},
        "age": {"type": "integer"}
    }
};

let valid = validate_schema(data, schema);
```

### Determinism & Sandbox Notes

- File I/O is non-deterministic (filesystem state)
- Requires `<sec/sandbox>` capabilities:
  - `fs.read` for reading files
  - `fs.write` for writing files
- CSV parsing is deterministic for well-formed files
- JSON parsing follows strict JSON spec (no trailing commas)
- JSONL processes one JSON object per line
- Maximum file size: 100MB (configurable)
- Streaming recommended for large files (>10MB)

### Host Function Needs

- `__host_fs_read(path) -> bytes`
- `__host_fs_write(path, data) -> void`
- `__host_fs_stat(path) -> FileInfo`

### Performance Targets

- CSV parsing: ~50 MB/s
- JSON parsing: ~100 MB/s
- CSV writing: ~30 MB/s
- JSON writing: ~80 MB/s
- Streaming overhead: < 10μs per batch

### Test Plan

1. CSV reading with various delimiters
2. CSV writing with headers
3. JSON reading and parsing
4. JSON writing with pretty-print
5. JSONL reading (multi-line)
6. JSONL writing
7. Streaming large CSV files
8. Schema validation
9. Malformed data handling
10. Sandbox enforcement

### @ZNOTE Rationale

Data I/O is essential for ETL and data processing. Design focuses on:
- **Formats**: CSV, JSON, JSONL (common data formats)
- **Performance**: Streaming for large files
- **Validation**: Schema checking

---

## Module: `<gfx/2d>` - 2D Graphics

### Purpose
Provides framebuffer-style 2D drawing API for text, shapes, and sprite output.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `create_canvas` | `create_canvas(width: int, height: int) -> Canvas` | Canvas instance | `InvalidSize` |
| `clear_canvas` | `clear_canvas(canvas: Canvas, color: int) -> void` | None | None |
| `draw_pixel` | `draw_pixel(canvas: Canvas, x: int, y: int, color: int) -> void` | None | `OutOfBounds` |
| `draw_line` | `draw_line(canvas: Canvas, x1: int, y1: int, x2: int, y2: int, color: int) -> void` | None | None |
| `draw_rect` | `draw_rect(canvas: Canvas, x: int, y: int, w: int, h: int, color: int, filled: bool) -> void` | None | None |
| `draw_circle` | `draw_circle(canvas: Canvas, cx: int, cy: int, radius: int, color: int, filled: bool) -> void` | None | None |
| `draw_text` | `draw_text(canvas: Canvas, x: int, y: int, text: string, color: int) -> void` | None | None |
| `blit_sprite` | `blit_sprite(canvas: Canvas, sprite: Canvas, x: int, y: int) -> void` | None | None |
| `save_canvas` | `save_canvas(canvas: Canvas, path: string) -> void` | None | `WriteError` |
| `load_canvas` | `load_canvas(path: string) -> Canvas` | Canvas instance | `FileNotFound`, `InvalidFormat` |

### Color Format

```
32-bit RGBA: 0xRRGGBBAA
Examples:
  0xFF0000FF - Red
  0x00FF00FF - Green
  0x0000FFFF - Blue
  0xFFFFFFFF - White
  0x000000FF - Black
  0xFF000080 - Semi-transparent red
```

### Example Usage

```solvrascript
import { create_canvas, clear_canvas, draw_rect, draw_circle, draw_text, save_canvas } from <gfx/2d>;

// Create 800x600 canvas
let canvas = create_canvas(800, 600);

// Clear to black
clear_canvas(canvas, 0x000000FF);

// Draw filled rectangle
draw_rect(canvas, 100, 100, 200, 150, 0xFF0000FF, true);

// Draw circle outline
draw_circle(canvas, 400, 300, 50, 0x00FF00FF, false);

// Draw text
draw_text(canvas, 50, 50, "Hello, SolvraScript!", 0xFFFFFFFF);

// Save to PNG
save_canvas(canvas, "/tmp/output.png");

// Sprite blitting
let sprite = load_canvas("assets/player.png");
blit_sprite(canvas, sprite, 300, 200);
```

### Determinism & Sandbox Notes

- Drawing operations are deterministic
- Requires `<sec/sandbox>` capabilities:
  - `gfx.render` for drawing operations
  - `fs.write` for saving canvases
  - `fs.read` for loading canvases
- Canvas is in-memory bitmap
- Maximum canvas size: 4096x4096
- Pixel format: RGBA (8 bits per channel)
- Drawing operations clip to canvas bounds

### Host Function Needs

- `__host_gfx_create_canvas(width, height) -> canvas_id`
- `__host_gfx_set_pixel(canvas_id, x, y, color) -> void`
- `__host_gfx_draw_line(canvas_id, x1, y1, x2, y2, color) -> void`
- `__host_gfx_draw_rect(canvas_id, x, y, w, h, color, filled) -> void`
- `__host_gfx_draw_circle(canvas_id, cx, cy, radius, color, filled) -> void`
- `__host_gfx_draw_text(canvas_id, x, y, text, color) -> void`
- `__host_gfx_blit(dest_canvas, src_canvas, x, y) -> void`
- `__host_gfx_save_png(canvas_id, path) -> void`
- `__host_gfx_load_png(path) -> canvas_id`

### Performance Targets

- Pixel write: < 50ns
- Line drawing: < 10μs per line
- Rectangle fill: < 1ms per 1000x1000 rect
- Text rendering: < 5ms per 100 characters
- Blit: < 1ms per 100x100 sprite

### Test Plan

1. Canvas creation and sizing
2. Pixel drawing
3. Line drawing (horizontal, vertical, diagonal)
4. Rectangle drawing (filled and outlined)
5. Circle drawing
6. Text rendering
7. Sprite blitting
8. PNG save and load
9. Clipping behavior

### @ZNOTE Rationale

2D graphics API enables visualization and UI. Design is simple:
- **Immediate mode**: Direct drawing calls
- **Software rendering**: CPU-based (no GPU required)
- **Compatibility**: Works with `<game/sprite>` module

---

## Module: `<storage/kv>` - Key-Value Storage

### Purpose
Provides file-backed key-value store with TTL support and write-ahead journaling.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `open_store` | `open_store(path: string) -> Store` | Store instance | `OpenFailed` |
| `close_store` | `close_store(store: Store) -> void` | None | None |
| `get` | `get(store: Store, key: string) -> string` | Value | `KeyNotFound` |
| `set` | `set(store: Store, key: string, value: string, ttl_sec: int) -> void` | None | `WriteError` |
| `delete` | `delete(store: Store, key: string) -> void` | None | None |
| `exists` | `exists(store: Store, key: string) -> bool` | Key existence | None |
| `keys` | `keys(store: Store) -> [string]` | All keys | None |
| `clear` | `clear(store: Store) -> void` | None | None |
| `compact` | `compact(store: Store) -> void` | None | None |

### Example Usage

```solvrascript
import { open_store, close_store, get, set, delete, exists, keys } from <storage/kv>;

// Open or create store
let store = open_store("/var/lib/app/data.kv");

// Set key-value pairs
set(store, "user:123:name", "Alice", 0);      // No TTL
set(store, "session:abc", "token123", 3600);  // Expires in 1 hour

// Get values
let name = get(store, "user:123:name");
println("User name: " + name);

// Check existence
if (exists(store, "session:abc")) {
    println("Session active");
}

// List all keys
let all_keys = keys(store);
for (let key in all_keys) {
    println("Key: " + key);
}

// Delete key
delete(store, "session:abc");

// Compact store (remove expired and deleted keys)
compact(store);

close_store(store);

// Pattern matching (manual iteration)
let user_keys = [];
for (let key in keys(store)) {
    if (starts_with(key, "user:")) {
        push(user_keys, key);
    }
}
```

### Determinism & Sandbox Notes

- Storage operations are non-deterministic (filesystem I/O)
- Requires `<sec/sandbox>` capabilities:
  - `fs.read` for reading store
  - `fs.write` for writing store
- Store file format: append-only journal + index
- TTL expiration checked on access
- Compaction removes tombstones and expired keys
- Thread-safe for single process (file locking)
- Not suitable for concurrent multi-process access

### Host Function Needs

- `__host_fs_open(path, mode) -> fd`
- `__host_fs_read(fd, offset, size) -> bytes`
- `__host_fs_write(fd, offset, data) -> void`
- `__host_fs_sync(fd) -> void`
- `__host_fs_close(fd) -> void`
- `__host_time_now_unix() -> int` (for TTL)

### Performance Targets

- Get operation: < 10μs (cached)
- Set operation: < 100μs (with journal)
- Delete operation: < 50μs
- Compaction: < 1s per 100k keys
- Memory overhead: ~64 bytes per key

### Test Plan

1. Store creation and opening
2. Set and get operations
3. TTL expiration
4. Key deletion
5. Key listing
6. Compaction
7. Persistence across restarts
8. Large value storage (>1MB)

### @ZNOTE Rationale

Key-value storage enables persistent state. Design focuses on:
- **Simplicity**: Single-file database
- **Durability**: Write-ahead journal
- **TTL support**: Automatic expiration

---

## Module: `<crypto/wrap>` - Crypto Wrappers

### Purpose
Provides friendly high-level wrappers around `<sec/*>` modules for common cryptographic tasks.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `sign_file` | `sign_file(file_path: string, key_path: string) -> string` | Signature (hex) | `FileNotFound`, `InvalidKey` |
| `verify_file` | `verify_file(file_path: string, signature: string, key_path: string) -> bool` | Verification result | `FileNotFound` |
| `encrypt_file` | `encrypt_file(input_path: string, output_path: string, password: string) -> void` | None | `FileNotFound`, `EncryptionFailed` |
| `decrypt_file` | `decrypt_file(input_path: string, output_path: string, password: string) -> void` | None | `DecryptionFailed` |
| `hash_file` | `hash_file(file_path: string, algo: string) -> string` | Hash (hex) | `FileNotFound` |
| `secure_delete` | `secure_delete(file_path: string, passes: int) -> void` | None | `FileNotFound` |

### Example Usage

```solvrascript
import { sign_file, verify_file, encrypt_file, decrypt_file, hash_file } from <crypto/wrap>;

// Hash file
let file_hash = hash_file("/data/document.pdf", "sha256");
println("SHA-256: " + file_hash);

// Sign file with RSA private key
let signature = sign_file("/data/release.zip", "/keys/private.pem");
println("Signature: " + signature);

// Verify signature with public key
let valid = verify_file("/data/release.zip", signature, "/keys/public.pem");
if (valid) {
    println("Signature valid!");
}

// Encrypt file with password
encrypt_file("/data/secrets.txt", "/data/secrets.txt.enc", "my_strong_password");

// Decrypt file
decrypt_file("/data/secrets.txt.enc", "/data/secrets_decrypted.txt", "my_strong_password");

// Secure delete (overwrite with random data)
import { secure_delete } from <crypto/wrap>;
secure_delete("/data/temp_secrets.txt", 3);  // 3 passes
```

### Determinism & Sandbox Notes

- File operations are non-deterministic
- Requires `<sec/sandbox>` capabilities:
  - `fs.read` for reading files
  - `fs.write` for writing files
  - `crypto.sign` for signing
  - `crypto.encrypt` for encryption
- Encryption uses XChaCha20-Poly1305 with Argon2-derived key
- Password-based encryption includes random salt
- Signature format: HMAC-SHA256 or RSA-SHA256

### Host Function Needs

All functions delegate to `<sec/*>` and `<data/io>` modules

### Performance Targets

- File hashing: ~500 MB/s
- File signing: < 10ms for typical files
- File encryption: ~200 MB/s
- Secure delete: ~50 MB/s (3 passes)

### Test Plan

1. File hashing
2. File signing and verification
3. Password-based file encryption
4. File decryption
5. Secure deletion
6. Large file handling (>100MB)

### @ZNOTE Rationale

High-level wrappers simplify common tasks. Design focuses on:
- **Convenience**: Single function for common operations
- **Safety**: Secure defaults (strong algorithms, salts, etc.)
- **Composition**: Builds on `<sec/*>` primitives

---

## Summary

The Supporting standard library provides essential infrastructure modules for networking, process management, data I/O, 2D graphics, persistent storage, and high-level crypto operations. These modules complement the Web, Game, and Security families to create a complete application development platform.

### Module Dependencies

```
<net/sock> - Requires host network stack
<devops/runner> - Requires host process management
<data/io> - Requires host filesystem
<gfx/2d> - Requires host graphics backend (CPU-based)
<storage/kv> - Requires host filesystem
<crypto/wrap> - Depends on <sec/*> and <data/io>
```

### Integration Notes

- `<net/sock>` provides primitives for custom protocols (use `<web/http>` for HTTP)
- `<gfx/2d>` complements `<game/sprite>` (use together for game rendering)
- `<storage/kv>` is single-process; use database for multi-process
- `<crypto/wrap>` wraps `<sec/*>` for ergonomics

### Next Implementation Phase

See `specs/module_index.md` for complete function inventory and `specs/host_bridge_map.md` for host function requirements.
