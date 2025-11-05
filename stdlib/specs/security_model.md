# SolvraScript Standard Library - Security Model

**Author:** Zachariah Obie
**License:** Apache License 2.0
**Status:** Phase 1 - Specification
**Last Updated:** 2025-11-04

## Overview

The SolvraScript standard library uses a capability-based security model integrated with `<sec/sandbox>` to enforce least-privilege access control. All privileged operations (network, filesystem, process, etc.) require explicit capability grants. This document defines the security model, permission tiers, and safe defaults.

---

## Security Principles

### 1. Default Deny

All privileged operations are **denied by default**. Scripts must explicitly request capabilities either:
- At launch time via manifest/CLI flags
- At runtime via `request_capability()` with user approval

### 2. Least Privilege

Scripts receive only the minimal capabilities needed for their stated purpose. Capabilities are:
- **Fine-grained:** Separate read/write, HTTP client/server, etc.
- **Non-escalating:** Cannot gain capabilities after initial grant
- **Revocable:** Can be dropped but not re-acquired

### 3. Capability Hierarchy

Capabilities follow a hierarchical naming scheme:

```
domain.category.operation
```

Examples:
- `fs.read` — Read files
- `net.http.client` — HTTP client operations
- `crypto.encrypt` — Encryption operations

Parent capabilities imply child capabilities:
- `net` grants all `net.*` capabilities
- `net.http` grants `net.http.client` and `net.http.server`

### 4. Sandbox Isolation

Scripts run in isolated sandboxes with:
- Private working directory (no access to parent by default)
- Restricted environment variables
- Capability-based resource access
- Memory and CPU limits

---

## Permission Tiers

### Tier 0: Pure Computation (No Capabilities)

**Description:** Scripts with no external effects. Safe for untrusted code execution.

**Allowed Operations:**
- Arithmetic and logic
- String manipulation
- Data structure operations (maps, arrays)
- Pure function calls
- Local variable access

**Denied Operations:**
- File I/O
- Network access
- Process spawning
- External state modification

**Use Cases:**
- Mathematical calculations
- Data transformations
- Algorithm implementations
- Unit test logic

**Example Script:**
```solvrascript
// Pure computation - no capabilities needed
fn fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

println("Fib(10) = " + str(fibonacci(10)));
```

---

### Tier 1: Read-Only Access

**Required Capabilities:**
- `fs.read` — Read files and directories
- `net.dns` — DNS resolution only

**Allowed Operations:**
- Read configuration files
- Load data files (JSON, CSV)
- Resolve hostnames
- Read environment variables (with `process.env.read`)

**Denied Operations:**
- Writing files
- Making network requests
- Process execution

**Use Cases:**
- Configuration loaders
- Data analysis scripts
- Read-only reporting tools

**Example Script:**
```solvrascript
import { read_json } from <data/io>;
import { enforce_capability } from <sec/sandbox>;

enforce_capability("fs.read");

let config = read_json("/etc/app/config.json");
println("Database: " + config["database"]["host"]);
```

---

### Tier 2: Local Write Access

**Required Capabilities:**
- `fs.read` — Read files
- `fs.write` — Write files (restricted to specific directories)

**Allowed Operations:**
- Read and write files in designated directories
- Create/modify configuration
- Generate reports
- Persistent key-value storage

**Denied Operations:**
- Network access
- Process execution
- System file modification (e.g., `/etc`, `/sys`)

**Use Cases:**
- Log processors
- Local data storage
- Configuration management
- Offline applications

**Example Script:**
```solvrascript
import { open_store, set, get } from <storage/kv>;
import { enforce_capability } from <sec/sandbox>;

enforce_capability("fs.read");
enforce_capability("fs.write");

let store = open_store("/home/user/.app/data.kv");
set(store, "last_run", str(time_now()));
```

---

### Tier 3: Network Client

**Required Capabilities:**
- `fs.read` — Read files (optional)
- `net.http.client` — HTTP/HTTPS requests
- `net.dns` — DNS resolution

**Allowed Operations:**
- Make HTTP requests
- Fetch remote data
- API integrations
- WebSocket client connections (with `net.websocket`)

**Denied Operations:**
- HTTP server (no listening)
- Raw TCP/UDP sockets
- Process execution

**Use Cases:**
- API clients
- Web scrapers
- Remote data fetchers
- Integration scripts

**Example Script:**
```solvrascript
import { get, parse_json } from <web/http>;
import { enforce_capability } from <sec/sandbox>;

enforce_capability("net.http.client");

let response = get("https://api.github.com/users/octocat", {});
let data = parse_json(response);
println("User: " + data["name"]);
```

---

### Tier 4: Network Server

**Required Capabilities:**
- `net.http.server` — HTTP server
- `fs.read` — Read static files (optional)
- `fs.write` — Write logs (optional)

**Allowed Operations:**
- Listen on network ports (>= 1024)
- Serve HTTP requests
- WebSocket server
- Static file serving

**Denied Operations:**
- Privileged port binding (< 1024)
- Raw socket access
- Process spawning

**Use Cases:**
- Web servers
- API backends
- WebSocket servers
- Microservices

**Example Script:**
```solvrascript
import { create_server, listen, send_json } from <web/server>;
import { enforce_capability } from <sec/sandbox>;

enforce_capability("net.http.server");

let server = create_server({});
server.handler = fn(request, response) {
    send_json(response, 200, {"status": "ok"});
};

listen(server, 8080);
println("Server running on :8080");
```

---

### Tier 5: Process Execution

**Required Capabilities:**
- `process.spawn` — Spawn child processes
- `process.env` — Access environment variables
- `fs.read` / `fs.write` — File access (usually needed)

**Allowed Operations:**
- Execute external programs
- Spawn child processes
- Capture stdout/stderr
- Set environment variables for children

**Denied Operations:**
- Unrestricted command execution (whitelist enforced)
- Shell metacharacter injection

**Use Cases:**
- Build automation
- CI/CD scripts
- System administration
- DevOps tools

**Example Script:**
```solvrascript
import { spawn_process, wait_process, read_stdout } from <devops/runner>;
import { enforce_capability } from <sec/sandbox>;

enforce_capability("process.spawn");

let proc = spawn_process("cargo", ["build", "--release"], {
    "cwd": "/project",
    "stdout": true
});

let exit_code = wait_process(proc, 60000);
println("Build exited with code: " + str(exit_code));
```

---

### Tier 6: Cryptographic Operations

**Required Capabilities:**
- `crypto.hash` — Hashing operations
- `crypto.encrypt` — Encryption/decryption
- `crypto.sign` — Signing operations
- `crypto.kdf` — Key derivation

**Allowed Operations:**
- Hash data (SHA-256, BLAKE3)
- Encrypt/decrypt with AEAD
- Sign and verify signatures
- Derive keys from passwords

**Denied Operations:**
- Access to system keyring (future enhancement)
- Hardware security modules (future enhancement)

**Use Cases:**
- Password hashing
- Data encryption
- Digital signatures
- Authentication systems

**Example Script:**
```solvrascript
import { sha256 } from <sec/hash>;
import { encrypt_xchacha, generate_key, generate_nonce } from <sec/aead>;
import { enforce_capability } from <sec/sandbox>;

enforce_capability("crypto.encrypt");

let key = generate_key("xchacha20");
let nonce = generate_nonce("xchacha20");
let ciphertext = encrypt_xchacha(key, nonce, "secret message");
```

---

### Tier 7: Full System Access (Privileged)

**Required Capabilities:**
- `fs` — Full filesystem access
- `net` — All network operations
- `process` — All process operations
- `crypto` — All cryptographic operations
- `system` — System administration

**Allowed Operations:**
- Unrestricted file access (including `/etc`, `/sys`)
- Bind privileged ports (< 1024)
- Execute any command
- Modify system configuration

**⚠️ WARNING:** Only grant to fully trusted scripts.

**Use Cases:**
- System administration tools
- Package managers
- Installer scripts
- OS maintenance utilities

**Example Script:**
```solvrascript
// Full system access - HIGH RISK
import { enforce_capability } from <sec/sandbox>;

enforce_capability("system.admin");

// Proceed with caution...
```

---

## Capability Reference

### Filesystem (`fs.*`)

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `fs.read` | Read files and directories | Low |
| `fs.write` | Write/modify files | Medium |
| `fs.execute` | Execute files | High |
| `fs` | Full filesystem access | Critical |

**Path Restrictions:**
- Scripts cannot access parent directory by default
- Absolute paths require explicit capability grant
- Symbolic link following is restricted

---

### Network (`net.*`)

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `net.dns` | DNS resolution | Low |
| `net.http.client` | HTTP/HTTPS client | Medium |
| `net.http.server` | HTTP server | Medium |
| `net.websocket` | WebSocket client/server | Medium |
| `net.tcp` | Raw TCP sockets | High |
| `net.udp` | Raw UDP sockets | High |
| `net` | All network operations | High |

**Additional Restrictions:**
- Privileged ports (< 1024) require `net.privileged`
- Maximum concurrent connections enforced
- Timeouts mandatory for all operations

---

### Process (`process.*`)

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `process.env.read` | Read environment variables | Low |
| `process.env` | Modify environment variables | Medium |
| `process.spawn` | Spawn child processes | High |
| `process.spawn.<cmd>` | Spawn specific command | Medium |
| `process` | Full process control | Critical |

**Command Whitelist:**
- By default, `process.spawn` uses whitelist mode
- Only explicitly allowed commands can execute
- `process.spawn.<cmd>` grants permission for specific command

---

### Cryptography (`crypto.*`)

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `crypto.hash` | Hashing operations | Low |
| `crypto.kdf` | Key derivation | Low |
| `crypto.encrypt` | Encryption/decryption | Medium |
| `crypto.sign` | Digital signatures | Medium |
| `crypto.pki` | PKI operations | Medium |
| `crypto` | All crypto operations | Medium |

---

### Input/Graphics (`input.*`, `gfx.*`)

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `input.keyboard` | Keyboard input | Low |
| `input.mouse` | Mouse input | Low |
| `input.gamepad` | Gamepad input | Low |
| `gfx.render` | Graphics rendering | Low |
| `audio.play` | Audio playback | Low |

---

### Time (`time.*`)

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `time.monotonic` | Monotonic clock | Low |
| `time.system` | System clock access | Low |
| `time.system.set` | Set system time | Critical |

---

## Safe Defaults

### Default Grants (Minimal Sandbox)

Scripts with no manifest default to:
```
[No capabilities]
```

All operations requiring capabilities will fail with `CapabilityDenied` error.

### Interactive Mode Grants

When running scripts interactively (CLI), user may be prompted:
```
Script requests capability: fs.read
Reason: Load configuration file
Grant? [y/N]
```

### Manifest-Based Grants

Scripts can declare capabilities in a manifest:

**`.svs.toml`:**
```toml
[permissions]
capabilities = [
    "fs.read",
    "net.http.client"
]

[permissions.paths]
read = [
    "/home/user/.config/app",
    "/data/public"
]
```

---

## Auditing and Logging

### Capability Audit Log

All capability checks are logged:
```
[AUDIT] Script: /scripts/fetch_data.svs
[AUDIT] Capability: net.http.client
[AUDIT] Status: GRANTED
[AUDIT] Timestamp: 2025-11-04T12:34:56Z
```

### Security Events

Critical events trigger alerts:
- Capability denial
- Privilege escalation attempts
- Sandbox escape attempts
- Suspicious operations (e.g., reading `/etc/shadow`)

---

## Best Practices

### 1. Request Minimum Capabilities

```solvrascript
// BAD: Requesting too much
enforce_capability("fs");

// GOOD: Requesting only what's needed
enforce_capability("fs.read");
```

### 2. Drop Capabilities After Use

```solvrascript
import { enforce_capability, drop_capability } from <sec/sandbox>;

enforce_capability("fs.write");
write_file("/tmp/output.txt", data);
drop_capability("fs.write");  // Drop after use
```

### 3. Use Nested Sandboxes

```solvrascript
import { create_sandbox, run_sandboxed } from <sec/sandbox>;

// Run untrusted code in restricted sandbox
let sandbox = create_sandbox(["fs.read"]);
let result = run_sandboxed(sandbox, fn() {
    // Limited to fs.read only
    return read_file("/data/public.txt");
});
```

### 4. Validate Inputs Before Privileged Operations

```solvrascript
fn safe_read_file(path) {
    // Validate path before accessing
    if (starts_with(path, "/etc/")) {
        error("Access denied: system files");
    }
    enforce_capability("fs.read");
    return read_file(path);
}
```

---

## Security Checklist for Library Implementation

- [ ] All privileged operations check capabilities before execution
- [ ] Capability checks use constant-time comparison
- [ ] Path traversal prevention (reject `..` in paths)
- [ ] Command injection prevention (no shell expansion)
- [ ] Timeout enforcement for all I/O operations
- [ ] Resource limits (memory, CPU, file descriptors)
- [ ] Audit logging for all capability checks
- [ ] Secure defaults (deny by default)
- [ ] Input validation before cryptographic operations
- [ ] Constant-time secret comparison

---

## Future Enhancements

### Phase 2
- Hardware security module (HSM) integration
- System keyring access (`crypto.keyring`)
- SELinux/AppArmor integration
- Container isolation

### Phase 3
- Capability delegation (transfer to other scripts)
- Time-limited capabilities (auto-expire)
- Capability markets (fine-grained control)
- Formal verification of security properties

---

## References

- Capability-based security: Dennis and Van Horn (1966)
- POSIX capabilities: `capabilities(7)`
- Web Permissions API: W3C Permissions
- Android permissions model

---

## Appendix: Capability Matrix

| Module | Required Capabilities | Default Tier |
|--------|----------------------|--------------|
| `<web/http>` | `net.http.client`, `net.dns` | 3 |
| `<web/server>` | `net.http.server` | 4 |
| `<web/ws>` | `net.websocket` | 3-4 |
| `<web/static>` | `fs.read`, `net.http.server` | 4 |
| `<game/input>` | `input.keyboard`, `input.mouse` | 1 |
| `<game/sprite>` | `gfx.render`, `fs.read` | 2 |
| `<game/audio>` | `audio.play`, `fs.read` | 2 |
| `<sec/hash>` | None | 0 |
| `<sec/aead>` | `crypto.encrypt` | 6 |
| `<sec/jwt>` | `crypto.sign` | 6 |
| `<net/sock>` | `net.tcp` or `net.udp` | 3-4 |
| `<devops/runner>` | `process.spawn` | 5 |
| `<data/io>` | `fs.read`, `fs.write` | 2 |
| `<gfx/2d>` | `gfx.render`, `fs.read`, `fs.write` | 2 |
| `<storage/kv>` | `fs.read`, `fs.write` | 2 |
| `<crypto/wrap>` | `crypto.encrypt`, `crypto.sign`, `fs.read`, `fs.write` | 6 |

---

**End of Security Model Specification**
