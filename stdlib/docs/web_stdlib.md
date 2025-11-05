# SolvraScript Web Standard Library

**Author:** Zachariah Obie
**License:** Apache License 2.0
**Status:** Phase 1 - Design & Specification
**Last Updated:** 2025-11-04

## Overview

The Web standard library provides HTTP client/server capabilities, WebSocket support, templating, static file serving, and utility functions for building web applications in SolvraScript. All modules are deterministic, sandbox-safe, and VM-compliant with no external dependencies.

## Module Taxonomy & Imports

### Standard Library Import Syntax

```solvrascript
// Import entire module
import <web/http>;
import <web/server>;
import <web/router>;
import <web/ws>;
import <web/tpl>;
import <web/static>;
import <web/utils>;

// Import specific functions
import { get, post, put, delete } from <web/http>;
import { create_server, listen } from <web/server>;
import { route, param } from <web/router>;
```

### Module Hierarchy

```
<web/>
├── http       # HTTP client operations
├── server     # HTTP server with middleware
├── router     # Request routing and path matching
├── ws         # WebSocket client/server
├── tpl        # Template engine
├── static     # Static file serving
└── utils      # URL parsing, cookies, helpers
```

---

## Module: `<web/http>` - HTTP Client

### Purpose
Provides HTTP/HTTPS client functionality for making requests with headers, JSON body helpers, configurable timeouts, and retry logic.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `get` | `get(url: string, options: map) -> Response` | Response object | `NetworkError`, `TimeoutError`, `InvalidURL` |
| `post` | `post(url: string, body: any, options: map) -> Response` | Response object | `NetworkError`, `TimeoutError`, `InvalidURL` |
| `put` | `put(url: string, body: any, options: map) -> Response` | Response object | `NetworkError`, `TimeoutError`, `InvalidURL` |
| `delete` | `delete(url: string, options: map) -> Response` | Response object | `NetworkError`, `TimeoutError`, `InvalidURL` |
| `request` | `request(method: string, url: string, options: map) -> Response` | Response object | `NetworkError`, `TimeoutError`, `InvalidURL`, `InvalidMethod` |
| `with_headers` | `with_headers(options: map, headers: map) -> map` | Modified options | None |
| `with_timeout` | `with_timeout(options: map, ms: int) -> map` | Modified options | None |
| `with_retries` | `with_retries(options: map, count: int) -> map` | Modified options | None |
| `parse_json` | `parse_json(response: Response) -> any` | Parsed JSON | `JSONParseError` |
| `to_json` | `to_json(data: any) -> string` | JSON string | `JSONEncodeError` |

### Response Object Structure

```solvrascript
{
    status: int,           // HTTP status code (200, 404, etc.)
    headers: map,          // Response headers as key-value pairs
    body: string,          // Response body as string
    ok: bool,              // true if status 200-299
    time_ms: int           // Request duration in milliseconds
}
```

### Example Usage

```solvrascript
import { get, post, with_headers, with_timeout, parse_json } from <web/http>;

// Simple GET request
let response = get("https://api.example.com/users", {});
if (response.ok) {
    let data = parse_json(response);
    println("Fetched " + str(len(data)) + " users");
}

// POST with JSON body and custom headers
let payload = {"name": "Alice", "email": "alice@example.com"};
let options = with_headers({}, {
    "Content-Type": "application/json",
    "Authorization": "Bearer token123"
});
options = with_timeout(options, 5000);  // 5 second timeout

let response = post("https://api.example.com/users", payload, options);
println("Status: " + str(response.status));

// Retry logic
let options = with_retries({}, 3);
let response = get("https://api.flaky-service.com/data", options);
```

### Determinism & Sandbox Notes

- All network operations require `<sec/sandbox>` capability: `net.http.client`
- DNS resolution is non-deterministic but cached per-session
- Timeout enforcement is deterministic and uses SolvraCore's async runtime
- Default timeout: 10,000ms (10 seconds)
- Maximum retries: 5 (prevents infinite loops)
- TLS certificate validation always enabled (cannot be disabled)

### Host Function Needs

- `__host_http_request(method, url, headers, body, timeout_ms) -> Response`
- `__host_dns_resolve(hostname) -> string` (IP address)
- `__host_tls_handshake(socket_fd, hostname) -> bool`

### Performance Targets

- Overhead per request: < 100μs
- Memory per active request: < 4KB
- Connection pooling: up to 10 persistent connections
- DNS cache: up to 100 entries with 5-minute TTL

### Test Plan

1. Unit tests for each HTTP method (GET, POST, PUT, DELETE)
2. Timeout enforcement validation (should fail after specified ms)
3. Retry logic with simulated network failures
4. JSON parsing for various payloads
5. Header manipulation and propagation
6. Connection pooling behavior
7. Sandbox enforcement (should fail without capability)

### @ZNOTE Rationale

HTTP client is essential for modern web applications and microservices. The design prioritizes:
- **Simplicity**: Single-function calls for common operations
- **Safety**: Mandatory timeouts, retry limits, and TLS validation
- **Composability**: Options builders allow flexible configuration
- **Observability**: Response includes timing information

---

## Module: `<web/server>` - HTTP Server

### Purpose
Provides minimal HTTP server with middleware support, routing integration, and request/response abstractions.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `create_server` | `create_server(options: map) -> Server` | Server instance | `InvalidConfig` |
| `listen` | `listen(server: Server, port: int) -> void` | None | `PortInUse`, `PermissionDenied` |
| `stop` | `stop(server: Server) -> void` | None | None |
| `use_middleware` | `use_middleware(server: Server, handler: fn) -> Server` | Modified server | None |
| `get_request_body` | `get_request_body(request: Request) -> string` | Body string | None |
| `get_request_header` | `get_request_header(request: Request, name: string) -> string` | Header value | `HeaderNotFound` |
| `send_response` | `send_response(response: Response, status: int, body: string) -> void` | None | None |
| `send_json` | `send_json(response: Response, status: int, data: any) -> void` | None | `JSONEncodeError` |
| `send_error` | `send_error(response: Response, status: int, message: string) -> void` | None | None |

### Request Object Structure

```solvrascript
{
    method: string,        // HTTP method (GET, POST, etc.)
    path: string,          // Request path (/users/123)
    query: map,            // Query parameters as key-value
    headers: map,          // Request headers
    body: string,          // Raw body content
    remote_addr: string    // Client IP address
}
```

### Example Usage

```solvrascript
import { create_server, listen, use_middleware, send_json } from <web/server>;
import { route } from <web/router>;

// Create server with logging middleware
let server = create_server({});
server = use_middleware(server, fn(request, next) {
    println("[" + request.method + "] " + request.path);
    return next(request);
});

// Define route handler
let handler = fn(request, response) {
    if (request.path == "/api/status") {
        send_json(response, 200, {"status": "ok", "version": "1.0.0"});
    } else {
        send_error(response, 404, "Not Found");
    }
};

// Attach handler and start listening
server.handler = handler;
listen(server, 8080);
println("Server listening on port 8080");
```

### Determinism & Sandbox Notes

- Server operations require `<sec/sandbox>` capability: `net.http.server`
- Server can only bind to ports >= 1024 (non-privileged)
- Request ordering is non-deterministic (concurrent clients)
- Each request handled in isolated async context
- Maximum concurrent connections: 1000
- Maximum request body size: 10MB (configurable)

### Host Function Needs

- `__host_server_create(port) -> server_fd`
- `__host_server_accept(server_fd) -> (client_fd, remote_addr)`
- `__host_server_read_request(client_fd) -> Request`
- `__host_server_write_response(client_fd, Response) -> void`
- `__host_server_close(fd) -> void`

### Performance Targets

- Request handling overhead: < 500μs per request
- Memory per connection: < 8KB
- Throughput: > 10,000 req/s on modern hardware
- Latency: p99 < 10ms (excluding handler logic)

### Test Plan

1. Server creation and lifecycle (create, listen, stop)
2. Request parsing for all HTTP methods
3. Response generation (text, JSON, errors)
4. Middleware chain execution order
5. Concurrent request handling
6. Maximum connection limit enforcement
7. Request body size limit enforcement
8. Sandbox enforcement (should fail without capability)

### @ZNOTE Rationale

Minimal HTTP server enables SolvraScript to build backend services. Design focuses on:
- **Simplicity**: Straightforward request/response model
- **Middleware**: Composable processing pipeline
- **Safety**: Connection and body size limits
- **Integration**: Works seamlessly with `<web/router>`

---

## Module: `<web/router>` - Request Router

### Purpose
Provides path pattern matching, parameter extraction, query parsing, and HTTP method routing.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `create_router` | `create_router() -> Router` | Router instance | None |
| `get` | `get(router: Router, pattern: string, handler: fn) -> Router` | Modified router | None |
| `post` | `post(router: Router, pattern: string, handler: fn) -> Router` | Modified router | None |
| `put` | `put(router: Router, pattern: string, handler: fn) -> Router` | Modified router | None |
| `delete` | `delete(router: Router, pattern: string, handler: fn) -> Router` | Modified router | None |
| `route` | `route(router: Router, method: string, pattern: string, handler: fn) -> Router` | Modified router | None |
| `match_request` | `match_request(router: Router, request: Request) -> Match` | Match object or null | None |
| `get_param` | `get_param(match: Match, name: string) -> string` | Parameter value | `ParamNotFound` |
| `get_query` | `get_query(request: Request, name: string) -> string` | Query value | `QueryNotFound` |

### Route Pattern Syntax

```
/users              # Exact match
/users/:id          # Named parameter (captured as "id")
/users/:id/posts    # Multiple segments with parameters
/files/*path        # Wildcard (captures rest of path)
```

### Example Usage

```solvrascript
import { create_router, get, post, match_request, get_param } from <web/router>;
import { send_json, send_error } from <web/server>;

// Create router with routes
let router = create_router();
router = get(router, "/api/users", fn(request, response, match) {
    send_json(response, 200, [{"id": 1, "name": "Alice"}]);
});

router = get(router, "/api/users/:id", fn(request, response, match) {
    let user_id = get_param(match, "id");
    send_json(response, 200, {"id": user_id, "name": "User " + user_id});
});

router = post(router, "/api/users", fn(request, response, match) {
    let body = parse_json(get_request_body(request));
    send_json(response, 201, {"created": true, "id": 123});
});

// Use router in server handler
let handler = fn(request, response) {
    let match = match_request(router, request);
    if (match != null) {
        match.handler(request, response, match);
    } else {
        send_error(response, 404, "Not Found");
    }
};
```

### Determinism & Sandbox Notes

- Route matching is fully deterministic
- Routes matched in registration order (first match wins)
- No regex support (intentional - ensures predictable performance)
- Parameter extraction uses deterministic string operations
- No special sandbox requirements (pure routing logic)

### Host Function Needs

None (pure SolvraScript implementation)

### Performance Targets

- Route matching: < 10μs per request
- Memory overhead: < 1KB per route
- Maximum routes: 1000

### Test Plan

1. Exact path matching
2. Named parameter extraction
3. Wildcard path capture
4. Query parameter parsing
5. HTTP method routing
6. Route priority (first-match behavior)
7. 404 handling for unmatched routes

### @ZNOTE Rationale

Routing is fundamental to web applications. Design emphasizes:
- **Predictability**: No regex, clear matching rules
- **Performance**: Fast lookup with minimal overhead
- **Ergonomics**: Method-specific helpers (get, post, etc.)

---

## Module: `<web/ws>` - WebSocket

### Purpose
Provides WebSocket client and server support for bidirectional real-time communication.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `connect` | `connect(url: string, options: map) -> WebSocket` | WebSocket instance | `ConnectionFailed`, `InvalidURL` |
| `send` | `send(ws: WebSocket, message: string) -> void` | None | `ConnectionClosed`, `SendFailed` |
| `recv` | `recv(ws: WebSocket, timeout_ms: int) -> string` | Message string | `TimeoutError`, `ConnectionClosed` |
| `close` | `close(ws: WebSocket) -> void` | None | None |
| `is_open` | `is_open(ws: WebSocket) -> bool` | Connection status | None |
| `create_ws_server` | `create_ws_server(server: Server, path: string, handler: fn) -> Server` | Modified server | None |

### Example Usage

```solvrascript
import { connect, send, recv, close } from <web/ws>;

// WebSocket client
let ws = connect("wss://echo.example.com", {});
send(ws, "Hello, WebSocket!");

let response = recv(ws, 5000);  // 5 second timeout
println("Received: " + response);

close(ws);

// WebSocket server (integrated with web/server)
import { create_server, listen } from <web/server>;
import { create_ws_server } from <web/ws>;

let server = create_server({});
server = create_ws_server(server, "/ws", fn(ws, message) {
    println("Received: " + message);
    send(ws, "Echo: " + message);
});

listen(server, 8080);
```

### Determinism & Sandbox Notes

- Requires `<sec/sandbox>` capability: `net.websocket`
- Message ordering is guaranteed within single connection
- Connection establishment is non-deterministic (network timing)
- Automatic ping/pong for connection keepalive
- Maximum message size: 1MB
- Maximum frame size: 64KB

### Host Function Needs

- `__host_ws_connect(url) -> ws_fd`
- `__host_ws_send(ws_fd, message) -> bool`
- `__host_ws_recv(ws_fd, timeout_ms) -> string`
- `__host_ws_close(ws_fd) -> void`
- `__host_ws_upgrade(server_fd, path) -> ws_fd` (server-side)

### Performance Targets

- Message latency: < 1ms (local network)
- Throughput: > 1000 messages/second
- Memory per connection: < 16KB

### Test Plan

1. Client connection establishment
2. Send and receive messages
3. Connection close handling
4. Timeout enforcement
5. Server-side WebSocket upgrade
6. Echo server roundtrip
7. Sandbox enforcement

### @ZNOTE Rationale

WebSocket support enables real-time applications (chat, games, live updates). Design focuses on:
- **Simplicity**: Synchronous send/recv model
- **Reliability**: Guaranteed message ordering
- **Integration**: Works with existing HTTP server

---

## Module: `<web/tpl>` - Template Engine

### Purpose
Simple template engine with variable interpolation (`{{var}}`) and partial includes.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `compile` | `compile(template: string) -> Template` | Compiled template | `SyntaxError` |
| `render` | `render(template: Template, data: map) -> string` | Rendered HTML | `MissingVariable` |
| `render_string` | `render_string(template_str: string, data: map) -> string` | Rendered HTML | `SyntaxError`, `MissingVariable` |
| `include_partial` | `include_partial(template: string, partial_name: string, content: string) -> string` | Modified template | None |

### Template Syntax

```html
<!DOCTYPE html>
<html>
<head>
    <title>{{title}}</title>
</head>
<body>
    <h1>Hello, {{name}}!</h1>
    <p>You have {{count}} new messages.</p>
    {{> footer}}  <!-- Include partial -->
</body>
</html>
```

### Example Usage

```solvrascript
import { compile, render, render_string } from <web/tpl>;

// Simple rendering
let html = render_string("<h1>{{title}}</h1>", {"title": "Welcome"});
println(html);  // <h1>Welcome</h1>

// Compiled template (for repeated use)
let template = compile("<p>Hello, {{name}}!</p>");
let html1 = render(template, {"name": "Alice"});
let html2 = render(template, {"name": "Bob"});

// With partials
let layout = `
<html>
<body>
    {{> content}}
    <footer>{{year}}</footer>
</body>
</html>
`;

let content_partial = "<h1>{{title}}</h1>";
layout = include_partial(layout, "content", content_partial);

let html = render_string(layout, {"title": "Home", "year": "2025"});
```

### Determinism & Sandbox Notes

- Fully deterministic (no I/O, no external state)
- Template compilation is pure transformation
- No scripting or code execution in templates
- No file system access (templates passed as strings)
- HTML escaping not included (use `<web/utils>` for that)

### Host Function Needs

None (pure SolvraScript implementation)

### Performance Targets

- Compilation: < 100μs for typical templates
- Rendering: < 10μs per variable substitution
- Memory: < 2KB overhead per compiled template

### Test Plan

1. Variable interpolation
2. Missing variable handling
3. Nested templates with partials
4. Empty templates
5. Special characters in data
6. Large templates (>10KB)

### @ZNOTE Rationale

Template engine enables server-side rendering. Design is intentionally minimal:
- **No logic**: Templates are pure data substitution
- **Fast**: Simple string replacement
- **Safe**: No code execution in templates

---

## Module: `<web/static>` - Static File Serving

### Purpose
Serves static files from directory with MIME type detection and ETag caching.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `serve_dir` | `serve_dir(root_path: string, options: map) -> Handler` | Request handler | `InvalidPath` |
| `get_mime_type` | `get_mime_type(file_path: string) -> string` | MIME type | None |
| `generate_etag` | `generate_etag(file_path: string) -> string` | ETag hash | `FileNotFound` |
| `check_if_modified` | `check_if_modified(request: Request, etag: string) -> bool` | Modification status | None |

### Example Usage

```solvrascript
import { serve_dir } from <web/static>;
import { create_server, listen, use_middleware } from <web/server>;

let server = create_server({});

// Serve static files from ./public directory
let static_handler = serve_dir("/var/www/public", {
    "index": "index.html",
    "cache_max_age": 3600
});

server = use_middleware(server, fn(request, next) {
    if (starts_with(request.path, "/static/")) {
        return static_handler(request);
    }
    return next(request);
});

listen(server, 8080);
```

### Determinism & Sandbox Notes

- Requires `<sec/sandbox>` capability: `fs.read`
- File reads are non-deterministic (filesystem state)
- ETag generation uses deterministic hashing
- Path traversal protection (rejects `..` in paths)
- Maximum file size: 10MB
- Only serves files (not directories or symlinks)

### Host Function Needs

- `__host_fs_read(path) -> bytes`
- `__host_fs_stat(path) -> FileInfo`
- `__host_hash_sha256(data) -> string`

### Performance Targets

- File serving overhead: < 200μs
- Memory per request: < file size + 4KB
- ETag generation: < 1ms for typical files

### Test Plan

1. Serve HTML, CSS, JS files
2. MIME type detection for common formats
3. ETag generation and validation
4. 304 Not Modified responses
5. Path traversal attack prevention
6. Large file handling
7. Sandbox enforcement

### @ZNOTE Rationale

Static file serving is essential for web applications. Design prioritizes:
- **Security**: Path traversal protection
- **Performance**: ETag caching
- **Simplicity**: Single function for common use case

---

## Module: `<web/utils>` - Utilities

### Purpose
URL parsing, cookie helpers, HTML escaping, and other common web utilities.

### API Function Table

| Function | Signature | Returns | Errors |
|----------|-----------|---------|--------|
| `parse_url` | `parse_url(url: string) -> URL` | Parsed URL object | `InvalidURL` |
| `build_url` | `build_url(base: string, params: map) -> string` | Complete URL | None |
| `join_path` | `join_path(segments: [string]) -> string` | Joined path | None |
| `parse_cookies` | `parse_cookies(cookie_header: string) -> map` | Cookie map | None |
| `build_cookie` | `build_cookie(name: string, value: string, options: map) -> string` | Cookie header | None |
| `html_escape` | `html_escape(text: string) -> string` | Escaped HTML | None |
| `url_encode` | `url_encode(text: string) -> string` | Encoded string | None |
| `url_decode` | `url_decode(text: string) -> string` | Decoded string | `InvalidEncoding` |

### URL Object Structure

```solvrascript
{
    scheme: string,      // "https"
    host: string,        // "example.com"
    port: int,           // 443
    path: string,        // "/api/users"
    query: map,          // {"page": "1", "limit": "10"}
    fragment: string     // "section-1"
}
```

### Example Usage

```solvrascript
import { parse_url, build_url, html_escape, parse_cookies, build_cookie } from <web/utils>;

// URL manipulation
let url = parse_url("https://api.example.com:443/users?page=1#top");
println(url.host);  // "api.example.com"
println(url.query["page"]);  // "1"

let new_url = build_url("https://api.example.com/search", {
    "q": "solvraos",
    "page": "2"
});
println(new_url);  // "https://api.example.com/search?q=solvraos&page=2"

// Cookie handling
let cookies = parse_cookies("session=abc123; theme=dark");
println(cookies["session"]);  // "abc123"

let cookie_header = build_cookie("session", "abc123", {
    "Max-Age": "3600",
    "HttpOnly": true,
    "Secure": true
});

// HTML escaping
let safe_html = html_escape("<script>alert('xss')</script>");
println(safe_html);  // "&lt;script&gt;alert('xss')&lt;/script&gt;"
```

### Determinism & Sandbox Notes

- All functions are deterministic and pure
- No network or file system access
- URL parsing follows RFC 3986
- HTML escaping prevents XSS attacks
- Cookie parsing follows RFC 6265

### Host Function Needs

None (pure SolvraScript implementation)

### Performance Targets

- URL parsing: < 5μs
- HTML escaping: < 1μs per character
- Cookie parsing: < 10μs per cookie

### Test Plan

1. URL parsing with all components
2. URL building with query parameters
3. Path joining with edge cases
4. Cookie parsing and generation
5. HTML escaping for XSS prevention
6. URL encoding/decoding roundtrip

### @ZNOTE Rationale

Web utilities provide essential helpers. Design focuses on:
- **Correctness**: RFC-compliant implementations
- **Security**: HTML escaping, URL validation
- **Convenience**: Common operations in single functions

---

## Summary

The Web standard library provides comprehensive HTTP client/server, WebSocket, templating, routing, and utility functions for building modern web applications in SolvraScript. All modules prioritize determinism, sandbox safety, and integration with `<sec/sandbox>` for capability-based security.

### Module Dependencies

```
<web/server> depends on <web/router> (optional)
<web/tpl> depends on <web/utils> (for HTML escaping)
<web/static> depends on <sec/hash> (for ETags)
All modules integrate with <sec/sandbox> for permission checks
```

### Next Implementation Phase

See `specs/module_index.md` for complete function inventory and `specs/host_bridge_map.md` for host function requirements.
