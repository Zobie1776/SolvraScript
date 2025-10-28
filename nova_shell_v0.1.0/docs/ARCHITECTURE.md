# Nova Shell Architecture

Nova Shell consists of a smithay-powered compositor, iced launcher and settings applications, a shared theme engine, and a wasm-first plugin sandbox. Communication between processes happens over JSON-RPC on Unix domain sockets managed by the compositor. The layout intentionally decouples rendering (Glow via smithay) from UI surfaces (iced) to keep GPU stacks isolated.
