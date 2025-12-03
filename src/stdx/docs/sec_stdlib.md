# SolvraScript Security & Crypto Standard Library

**Author:** Zachariah Obie
**License:** Apache License 2.0
**Status:** Phase 1 - Design & Specification
**Last Updated:** 2025-11-04

## Overview

The Security & Crypto standard library provides cryptographic hashing, authenticated encryption, key derivation, JWT token handling, PKI certificate operations, sandbox capability management, and fuzzing utilities. All modules use well-vetted algorithms, constant-time operations where applicable, and integrate with `<sec/sandbox>` for permission enforcement.

## Module Taxonomy & Imports

### Standard Library Import Syntax

```solvrascript
// Import entire module
import <sec/hash>;
import <sec/aead>;
import <sec/kdf>;
import <sec/jwt>;
import <sec/pki>;
import <sec/sandbox>;
import <sec/fuzz>;

// Import specific functions
import { sha256, blake3, hmac_sha256 } from <sec/hash>;
import { encrypt, decrypt } from <sec/aead>;
import { derive_key } from <sec/kdf>;
```

### Module Hierarchy

```
<sec/>
├── hash       # SHA-256/512, BLAKE3, HMAC
├── aead       # XChaCha20-Poly1305, AES-GCM
├── kdf        # scrypt, argon2 key derivation
├── jwt        # JWT sign/verify (HS256, RS256)
├── pki        # PEM cert parsing and verification
├── sandbox    # Capability probes and policy checks
└── fuzz       # Simple fuzzing utilities
```

---

## Module: `<sec/hash>` - Cryptographic Hashing

### Purpose
Provides cryptographic hash functions (SHA-256, SHA-512, BLAKE3) and HMAC for message authentication.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `sha256` | `sha256(data: string) -> string` | Hex-encoded hash | None |
| `sha256_bytes` | `sha256_bytes(data: bytes) -> bytes` | Raw hash bytes | None |
| `sha512` | `sha512(data: string) -> string` | Hex-encoded hash | None |
| `sha512_bytes` | `sha512_bytes(data: bytes) -> bytes` | Raw hash bytes | None |
| `blake3` | `blake3(data: string) -> string` | Hex-encoded hash | None |
| `blake3_bytes` | `blake3_bytes(data: bytes) -> bytes` | Raw hash bytes | None |
| `hmac_sha256` | `hmac_sha256(key: string, message: string) -> string` | Hex-encoded HMAC | None |
| `hmac_sha512` | `hmac_sha512(key: string, message: string) -> string` | Hex-encoded HMAC | None |
| `verify_hmac` | `verify_hmac(expected: string, key: string, message: string, algo: string) -> bool` | Verification result | None |

### Example Usage

```solvrascript
import { sha256, blake3, hmac_sha256, verify_hmac } from <sec/hash>;

// Hash password (DO NOT use for password storage, use KDF instead!)
let hash = sha256("my_password");
println("SHA-256: " + hash);

// BLAKE3 (faster, modern hash)
let fast_hash = blake3("large_data_chunk");

// HMAC for message authentication
let key = "secret_key_12345";
let message = "important_message";
let mac = hmac_sha256(key, message);

// Verify HMAC (constant-time comparison)
let valid = verify_hmac(mac, key, message, "sha256");
if (valid) {
    println("Message authenticated!");
}

// Hash file content
import { read_file } from <data/io>;
let file_data = read_file("/path/to/file.txt");
let file_hash = sha256(file_data);
println("File hash: " + file_hash);
```

### Determinism & Sandbox Notes

- All hash functions are deterministic
- Constant-time HMAC verification (prevents timing attacks)
- No special sandbox requirements (pure computation)
- Output is always hex-encoded lowercase
- Blake3 is fastest for large data (>1KB)

### Host Function Needs

- `__host_crypto_sha256(data) -> bytes`
- `__host_crypto_sha512(data) -> bytes`
- `__host_crypto_blake3(data) -> bytes`
- `__host_crypto_hmac(key, message, algo) -> bytes`

### Performance Targets

- SHA-256: ~500 MB/s throughput
- BLAKE3: ~3 GB/s throughput (parallel)
- HMAC: ~450 MB/s throughput
- Memory: < 1KB overhead

### Test Plan

1. SHA-256 test vectors (NIST)
2. SHA-512 test vectors
3. BLAKE3 test vectors
4. HMAC-SHA256 test vectors (RFC 4231)
5. Constant-time verification
6. Large data hashing (>10MB)

### @ZNOTE Rationale

Cryptographic hashing is fundamental to security. Design focuses on:
- **Standards**: NIST-approved algorithms (SHA-2) + modern (BLAKE3)
- **Safety**: Constant-time verification, no MD5/SHA-1
- **Performance**: BLAKE3 for speed, SHA-256 for compatibility

---

## Module: `<sec/aead>` - Authenticated Encryption

### Purpose
Provides authenticated encryption with associated data (AEAD) using XChaCha20-Poly1305 and AES-256-GCM.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `generate_key` | `generate_key(algo: string) -> bytes` | Random key | `InvalidAlgorithm` |
| `generate_nonce` | `generate_nonce(algo: string) -> bytes` | Random nonce | `InvalidAlgorithm` |
| `encrypt` | `encrypt(algo: string, key: bytes, nonce: bytes, plaintext: string, aad: string) -> bytes` | Ciphertext + tag | `EncryptionFailed` |
| `decrypt` | `decrypt(algo: string, key: bytes, nonce: bytes, ciphertext: bytes, aad: string) -> string` | Plaintext | `DecryptionFailed`, `AuthenticationFailed` |
| `encrypt_xchacha` | `encrypt_xchacha(key: bytes, nonce: bytes, plaintext: string) -> bytes` | Ciphertext + tag | `InvalidKey` |
| `decrypt_xchacha` | `decrypt_xchacha(key: bytes, nonce: bytes, ciphertext: bytes) -> string` | Plaintext | `DecryptionFailed` |
| `encrypt_aes_gcm` | `encrypt_aes_gcm(key: bytes, nonce: bytes, plaintext: string) -> bytes` | Ciphertext + tag | `InvalidKey` |
| `decrypt_aes_gcm` | `decrypt_aes_gcm(key: bytes, nonce: bytes, ciphertext: bytes) -> string` | Plaintext | `DecryptionFailed` |

### Algorithm Specifications

```
XChaCha20-Poly1305:
  Key size: 32 bytes (256 bits)
  Nonce size: 24 bytes (192 bits)
  Tag size: 16 bytes (128 bits)

AES-256-GCM:
  Key size: 32 bytes (256 bits)
  Nonce size: 12 bytes (96 bits)
  Tag size: 16 bytes (128 bits)
```

### Example Usage

```solvrascript
import { generate_key, generate_nonce, encrypt_xchacha, decrypt_xchacha } from <sec/aead>;

// Generate cryptographic key and nonce
let key = generate_key("xchacha20");
let nonce = generate_nonce("xchacha20");

// Encrypt message
let plaintext = "Secret message content";
let ciphertext = encrypt_xchacha(key, nonce, plaintext);

// Decrypt message
let decrypted = decrypt_xchacha(key, nonce, ciphertext);
println(decrypted);  // "Secret message content"

// With associated data (authenticated but not encrypted)
import { encrypt, decrypt } from <sec/aead>;

let aad = "user_id=12345";  // Not encrypted, but authenticated
let ciphertext = encrypt("xchacha20", key, nonce, plaintext, aad);

// Decryption will fail if AAD is tampered with
let decrypted = decrypt("xchacha20", key, nonce, ciphertext, aad);
```

### Determinism & Sandbox Notes

- Encryption is non-deterministic (requires random nonce)
- For deterministic encryption, use fixed nonce (NOT recommended for production)
- Requires `<sec/sandbox>` capability: `crypto.encrypt`
- Never reuse nonce with same key (breaks security)
- XChaCha20 preferred for larger nonce space (avoids nonce reuse)
- Constant-time tag verification

### Host Function Needs

- `__host_crypto_random_bytes(count) -> bytes`
- `__host_crypto_xchacha20_encrypt(key, nonce, plaintext, aad) -> ciphertext`
- `__host_crypto_xchacha20_decrypt(key, nonce, ciphertext, aad) -> plaintext`
- `__host_crypto_aes_gcm_encrypt(key, nonce, plaintext, aad) -> ciphertext`
- `__host_crypto_aes_gcm_decrypt(key, nonce, ciphertext, aad) -> plaintext`

### Performance Targets

- XChaCha20: ~1 GB/s throughput
- AES-GCM: ~2 GB/s throughput (hardware accelerated)
- Overhead: < 100μs per operation

### Test Plan

1. XChaCha20-Poly1305 test vectors
2. AES-256-GCM test vectors
3. Round-trip encryption/decryption
4. AAD authentication
5. Tampered ciphertext detection
6. Tampered AAD detection
7. Key/nonce size validation

### @ZNOTE Rationale

AEAD provides confidentiality and authenticity. Design emphasizes:
- **Security**: Modern algorithms (no AES-CBC, no unauthenticated encryption)
- **Simplicity**: Single encrypt/decrypt call
- **Flexibility**: AAD support for metadata authentication

---

## Module: `<sec/kdf>` - Key Derivation

### Purpose
Provides password-based key derivation functions (scrypt, argon2) for secure password storage and key stretching.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `derive_key_scrypt` | `derive_key_scrypt(password: string, salt: bytes, n: int, r: int, p: int, keylen: int) -> bytes` | Derived key | `InvalidParams` |
| `derive_key_argon2` | `derive_key_argon2(password: string, salt: bytes, variant: string, mem_kb: int, time: int, parallelism: int, keylen: int) -> bytes` | Derived key | `InvalidParams` |
| `derive_key` | `derive_key(password: string, salt: bytes, algo: string, options: map) -> bytes` | Derived key | `InvalidAlgorithm` |
| `generate_salt` | `generate_salt(size: int) -> bytes` | Random salt | None |
| `verify_password` | `verify_password(password: string, salt: bytes, expected_key: bytes, algo: string, options: map) -> bool` | Verification result | None |

### Example Usage

```solvrascript
import { derive_key_argon2, generate_salt, verify_password } from <sec/kdf>;

// Hash password for storage (Argon2id recommended)
let password = "user_password_123";
let salt = generate_salt(16);  // 16 bytes

let derived_key = derive_key_argon2(
    password,
    salt,
    "argon2id",
    65536,    // 64 MB memory
    3,        // 3 iterations
    4,        // 4 parallel threads
    32        // 32 byte output
);

// Store: salt + derived_key in database

// Verify password on login
let valid = verify_password(
    input_password,
    stored_salt,
    stored_key,
    "argon2",
    {"variant": "argon2id", "mem_kb": 65536, "time": 3, "parallelism": 4}
);

if (valid) {
    println("Login successful!");
}

// scrypt alternative
import { derive_key_scrypt } from <sec/kdf>;

let scrypt_key = derive_key_scrypt(
    password,
    salt,
    32768,  // N (CPU/memory cost)
    8,      // r (block size)
    1,      // p (parallelism)
    32      // key length
);
```

### Determinism & Sandbox Notes

- KDF is deterministic given same inputs
- Salt must be random and unique per password
- Requires `<sec/sandbox>` capability: `crypto.kdf`
- Computationally expensive (intentional for security)
- Constant-time verification
- Never log or expose derived keys

### Host Function Needs

- `__host_crypto_scrypt(password, salt, n, r, p, keylen) -> bytes`
- `__host_crypto_argon2(password, salt, variant, mem_kb, time, parallelism, keylen) -> bytes`

### Performance Targets

- Argon2: 100-500ms per derivation (configurable)
- scrypt: 50-200ms per derivation (configurable)
- Memory: User-configured (64MB typical for Argon2)

### Test Plan

1. Argon2 test vectors (RFC 9106)
2. scrypt test vectors (RFC 7914)
3. Password verification
4. Salt uniqueness
5. Parameter validation
6. Timing attack resistance

### @ZNOTE Rationale

KDF enables secure password storage. Design prioritizes:
- **Security**: Modern algorithms (Argon2id), no PBKDF2-HMAC-SHA1
- **Configurability**: Tunable cost parameters
- **Simplicity**: Single function for common cases

---

## Module: `<sec/jwt>` - JSON Web Tokens

### Purpose
Provides JWT signing and verification with HS256 (HMAC-SHA256) and RS256 (RSA-SHA256) algorithms.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `sign_jwt` | `sign_jwt(payload: map, secret: string, algo: string, expires_sec: int) -> string` | JWT token | `InvalidPayload`, `InvalidAlgorithm` |
| `verify_jwt` | `verify_jwt(token: string, secret: string, algo: string) -> map` | Decoded payload | `InvalidToken`, `ExpiredToken`, `SignatureInvalid` |
| `decode_jwt` | `decode_jwt(token: string) -> map` | Decoded payload (unverified) | `InvalidToken` |
| `sign_jwt_rs256` | `sign_jwt_rs256(payload: map, private_key: string, expires_sec: int) -> string` | JWT token | `InvalidKey` |
| `verify_jwt_rs256` | `verify_jwt_rs256(token: string, public_key: string) -> map` | Decoded payload | `SignatureInvalid` |

### JWT Structure

```
Header (base64url):
{
  "alg": "HS256",
  "typ": "JWT"
}

Payload (base64url):
{
  "sub": "user_id_123",
  "name": "Alice",
  "iat": 1704067200,
  "exp": 1704153600
}

Signature: HMAC-SHA256(header + "." + payload, secret)

Token: header.payload.signature
```

### Example Usage

```solvrascript
import { sign_jwt, verify_jwt } from <sec/jwt>;

// Create JWT with HS256
let payload = {
    "sub": "user_123",
    "name": "Alice",
    "role": "admin"
};

let secret = "my_secret_key_32_bytes_long";
let token = sign_jwt(payload, secret, "HS256", 3600);  // Expires in 1 hour

println("JWT: " + token);

// Verify JWT
let decoded = verify_jwt(token, secret, "HS256");
if (decoded != null) {
    println("User: " + decoded["name"]);
    println("Role: " + decoded["role"]);
} else {
    println("Invalid or expired token");
}

// RS256 with RSA keys
import { sign_jwt_rs256, verify_jwt_rs256 } from <sec/jwt>;
import { read_file } from <data/io>;

let private_key = read_file("private_key.pem");
let public_key = read_file("public_key.pem");

let rs_token = sign_jwt_rs256(payload, private_key, 3600);
let rs_decoded = verify_jwt_rs256(rs_token, public_key);
```

### Determinism & Sandbox Notes

- JWT generation is deterministic (fixed `iat` field)
- Signature verification uses constant-time comparison
- Expiration checked automatically in `verify_jwt`
- Requires `<sec/sandbox>` capability: `crypto.sign`
- HS256: Secret must be >= 32 bytes
- RS256: RSA key must be >= 2048 bits

### Host Function Needs

- `__host_crypto_hmac_sha256(key, message) -> bytes` (for HS256)
- `__host_crypto_rsa_sign(private_key, message) -> bytes` (for RS256)
- `__host_crypto_rsa_verify(public_key, message, signature) -> bool` (for RS256)

### Performance Targets

- HS256 sign: < 100μs
- HS256 verify: < 100μs
- RS256 sign: < 5ms
- RS256 verify: < 1ms

### Test Plan

1. HS256 JWT creation and verification
2. RS256 JWT creation and verification
3. Expired token rejection
4. Invalid signature detection
5. Malformed token handling
6. Payload encoding/decoding

### @ZNOTE Rationale

JWT enables stateless authentication. Design focuses on:
- **Standards**: RFC 7519 compliance
- **Security**: HS256 and RS256 only (no "none" algorithm)
- **Simplicity**: Single function for common use cases

---

## Module: `<sec/pki>` - Public Key Infrastructure

### Purpose
Provides PEM certificate parsing and basic X.509 certificate verification.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `parse_pem_cert` | `parse_pem_cert(pem: string) -> Certificate` | Parsed certificate | `InvalidPEM` |
| `get_cert_subject` | `get_cert_subject(cert: Certificate) -> string` | Subject DN | None |
| `get_cert_issuer` | `get_cert_issuer(cert: Certificate) -> string` | Issuer DN | None |
| `get_cert_expiry` | `get_cert_expiry(cert: Certificate) -> int` | Expiry timestamp | None |
| `is_cert_expired` | `is_cert_expired(cert: Certificate) -> bool` | Expiration status | None |
| `verify_cert_chain` | `verify_cert_chain(cert: Certificate, ca_certs: [Certificate]) -> bool` | Verification result | `VerificationFailed` |
| `get_public_key` | `get_public_key(cert: Certificate) -> string` | PEM public key | None |

### Certificate Object Structure

```solvrascript
{
    version: int,
    serial: string,
    subject: string,
    issuer: string,
    not_before: int,     // Unix timestamp
    not_after: int,      // Unix timestamp
    public_key: string,  // PEM format
    signature_algo: string
}
```

### Example Usage

```solvrascript
import { parse_pem_cert, is_cert_expired, verify_cert_chain, get_cert_subject } from <sec/pki>;
import { read_file } from <data/io>;

// Load certificate
let cert_pem = read_file("/etc/ssl/certs/server.crt");
let cert = parse_pem_cert(cert_pem);

// Check certificate details
println("Subject: " + get_cert_subject(cert));
println("Issuer: " + get_cert_issuer(cert));

if (is_cert_expired(cert)) {
    println("WARNING: Certificate expired!");
}

// Verify against CA chain
let ca_pem = read_file("/etc/ssl/certs/ca-bundle.crt");
let ca_cert = parse_pem_cert(ca_pem);

let valid = verify_cert_chain(cert, [ca_cert]);
if (valid) {
    println("Certificate chain valid");
} else {
    println("Certificate verification failed");
}
```

### Determinism & Sandbox Notes

- Certificate parsing is deterministic
- Chain verification depends on current time (expiry check)
- Requires `<sec/sandbox>` capability: `crypto.pki`
- Only X.509 v3 certificates supported
- No CRL or OCSP checking (future enhancement)

### Host Function Needs

- `__host_crypto_parse_x509(pem) -> Certificate`
- `__host_crypto_verify_signature(cert, ca_cert) -> bool`
- `__host_time_now_unix() -> int`

### Performance Targets

- Parse certificate: < 5ms
- Chain verification: < 10ms per certificate

### Test Plan

1. PEM parsing (RSA, ECDSA certs)
2. Subject/Issuer extraction
3. Expiry checking
4. Chain verification
5. Self-signed certificate handling
6. Invalid PEM rejection

### @ZNOTE Rationale

PKI enables TLS and code signing verification. Design is minimal:
- **Parsing only**: No certificate generation
- **Basic verification**: Expiry and signature chain
- **Future expansion**: CRL, OCSP, extended key usage

---

## Module: `<sec/sandbox>` - Capability Sandbox

### Purpose
Provides capability-based security model with permission probes and policy enforcement.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `has_capability` | `has_capability(cap: string) -> bool` | Permission status | None |
| `request_capability` | `request_capability(cap: string, reason: string) -> bool` | Grant status | `CapabilityDenied` |
| `drop_capability` | `drop_capability(cap: string) -> void` | None | None |
| `list_capabilities` | `list_capabilities() -> [string]` | Active capabilities | None |
| `enforce_capability` | `enforce_capability(cap: string) -> void` | None | `CapabilityDenied` |
| `create_sandbox` | `create_sandbox(allowed_caps: [string]) -> Sandbox` | Sandbox instance | None |
| `run_sandboxed` | `run_sandboxed(sandbox: Sandbox, fn: function) -> any` | Function result | `CapabilityDenied` |

### Capability Hierarchy

```
fs.read              # Read files
fs.write             # Write files
fs.execute           # Execute binaries

net.http.client      # HTTP client
net.http.server      # HTTP server
net.websocket        # WebSocket
net.tcp              # Raw TCP sockets
net.udp              # Raw UDP sockets
net.dns              # DNS resolution

crypto.encrypt       # Encryption operations
crypto.sign          # Signing operations
crypto.kdf           # Key derivation

input.keyboard       # Keyboard input
input.mouse          # Mouse input
input.gamepad        # Gamepad input

gfx.render           # Graphics rendering
audio.play           # Audio playback

time.monotonic       # Monotonic clock access
time.system          # System clock access

process.spawn        # Spawn child processes
process.env          # Access environment variables
```

### Example Usage

```solvrascript
import { has_capability, enforce_capability, create_sandbox, run_sandboxed } from <sec/sandbox>;

// Check if we have network permission
if (has_capability("net.http.client")) {
    import { get } from <web/http>;
    let response = get("https://api.example.com/data", {});
} else {
    println("Network access denied");
}

// Enforce capability (throws error if not granted)
enforce_capability("fs.read");
let data = read_file("/etc/config.toml");

// Request capability at runtime
import { request_capability } from <sec/sandbox>;

let granted = request_capability("fs.write", "Save user preferences");
if (granted) {
    write_file("/home/user/.config/app.toml", data);
}

// Run code in restricted sandbox
let sandbox = create_sandbox(["fs.read", "crypto.hash"]);

let result = run_sandboxed(sandbox, fn() {
    // This code can only read files and hash data
    let content = read_file("/data/public.txt");
    return sha256(content);
});
```

### Determinism & Sandbox Notes

- Capability checks are deterministic (static policy)
- Runtime requests are non-deterministic (user approval)
- All privileged operations route through capability checks
- Capabilities cannot be elevated after drop
- Sandbox is inherited by child scripts
- No capability bypass mechanism

### Host Function Needs

- `__host_sandbox_has_cap(cap) -> bool`
- `__host_sandbox_request_cap(cap, reason) -> bool`
- `__host_sandbox_enforce_cap(cap) -> void`
- `__host_sandbox_list_caps() -> [string]`

### Performance Targets

- Capability check: < 100ns (cached)
- Sandbox creation: < 10μs

### Test Plan

1. Capability presence checks
2. Capability enforcement
3. Sandbox creation
4. Sandboxed execution
5. Capability denial handling
6. Nested sandboxes

### @ZNOTE Rationale

Capability-based security prevents unauthorized access. Design emphasizes:
- **Least privilege**: Default-deny, explicit grants
- **Transparency**: Clear capability names
- **Composability**: Sandbox nesting

---

## Module: `<sec/fuzz>` - Fuzzing Utilities

### Purpose
Provides simple fuzzing utilities for testing input validation and security properties.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `generate_random_string` | `generate_random_string(min_len: int, max_len: int) -> string` | Random string | None |
| `generate_random_bytes` | `generate_random_bytes(len: int) -> bytes` | Random bytes | None |
| `mutate_string` | `mutate_string(input: string, mutations: int) -> string` | Mutated string | None |
| `fuzz_test` | `fuzz_test(fn: function, iterations: int, input_gen: function) -> FuzzResult` | Test results | None |

### FuzzResult Structure

```solvrascript
{
    iterations: int,
    failures: int,
    crashes: [string],      // Stack traces
    unique_crashes: int
}
```

### Example Usage

```solvrascript
import { fuzz_test, generate_random_string, mutate_string } from <sec/fuzz>;

// Test input validation
let test_fn = fn(input) {
    // Function under test
    let parsed = parse_url(input);
    if (parsed == null) {
        return false;  // Expected failure
    }
    return true;
};

let input_generator = fn() {
    return generate_random_string(1, 100);
};

let result = fuzz_test(test_fn, 10000, input_generator);

println("Iterations: " + str(result.iterations));
println("Crashes: " + str(result.crashes));
println("Unique crashes: " + str(result.unique_crashes));

// Manual mutation testing
let base_input = "https://example.com/path";
for (let i = 0; i < 1000; i = i + 1) {
    let mutated = mutate_string(base_input, 5);
    test_url_parser(mutated);
}
```

### Determinism & Sandbox Notes

- Fuzzing is non-deterministic (random generation)
- For reproducible fuzzing, seed RNG before running
- No special sandbox requirements
- Does not save crash corpus (manual logging required)

### Host Function Needs

- `__host_random_bytes(count) -> bytes`

### Performance Targets

- Input generation: < 10μs per input
- Mutation: < 5μs per mutation
- Fuzz iterations: > 10,000/second

### Test Plan

1. Random string generation
2. Byte generation
3. String mutation
4. Fuzz test execution
5. Crash detection
6. Reproducibility with seeded RNG

### @ZNOTE Rationale

Fuzzing finds edge cases and vulnerabilities. Design is minimal:
- **Simple API**: Easy to integrate into tests
- **Lightweight**: No corpus management
- **Flexible**: User-defined input generators

---

## Summary

The Security & Crypto standard library provides comprehensive cryptographic primitives, authentication mechanisms, PKI support, capability-based sandboxing, and fuzzing utilities for building secure SolvraScript applications. All modules use well-vetted algorithms and integrate with the SolvraCore sandbox model.

### Module Dependencies

```
<sec/hash> - Standalone
<sec/aead> - Requires host crypto backend
<sec/kdf> - Requires host crypto backend
<sec/jwt> - Depends on <sec/hash>
<sec/pki> - Requires host crypto backend
<sec/sandbox> - Core SolvraCore integration
<sec/fuzz> - Minimal host RNG
```

### Security Notes

- **Never** roll your own crypto (use host primitives)
- **Always** validate inputs before cryptographic operations
- **Always** use constant-time comparisons for secrets
- **Never** log or expose keys, nonces, or derived secrets
- **Always** use strong random generation for keys/salts

### Next Implementation Phase

See `specs/module_index.md` for complete function inventory and `specs/host_bridge_map.md` for host function requirements.
