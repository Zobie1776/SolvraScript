# Nova GUI Architecture

Nova GUI wraps a smithay 0.7 compositor, iced-based GUI surfaces, a theme parser, and optional wasm plugin surfaces into a cohesive workspace. Components communicate via JSON-RPC over Unix domain sockets, keeping rendering and UI stacks decoupled.
