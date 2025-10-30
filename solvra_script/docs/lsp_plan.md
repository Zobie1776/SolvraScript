## SolvraScript / SolvraCore LSP Roadmap

### Phase 1 – Foundations

- **Shared crates:** Extract tokenizer/parser into a reusable crate consumed by both the interpreter and a new `solvrascript-lsp` binary.
- **Document store:** Track open buffers with incremental text updates (rope-based data structure via `xi-rope` or `lapce` crates).
- **Synchronous diagnostics:** Re-tokenize and parse on each change, returning syntax and escape-sequence errors (including the single-quote rejection) as `textDocument/publishDiagnostics`.
- **Configuration:** Support workspace settings for maximum call depth, experimental template string interpolation, and SolvraCore-specific options.

### Phase 2 – Language Features

- **Symbol indexing:** Build AST symbol tables for functions, variables, and events to power `textDocument/documentSymbol` and `workspace/symbol` requests.
- **Hover & signature help:** Leverage the built-in registry (e.g., `prt`, `div`, `sbt`) to surface descriptions from `docs/language_reference.md` via `textDocument/hover` and `textDocument/signatureHelp`.
- **Completion provider:** Offer keyword, builtin, and alias completions with snippets; apply context filters to avoid suggesting legacy names unless enabled.
- **Formatting engine:** Provide `textDocument/formatting` using a simple pretty-printer with explicit space insertion rules that honour SolvraScript’s no-implicit-spacing contract.

### Phase 3 – SolvraCore Integration

- **Bytecode hooks:** For projects containing SolvraCore modules, expose `workspace/executeCommand` entries that trigger SolvraCore compilation, capturing diagnostics.
- **Cross-file analysis:** Resolve `import`/`use` statements, build dependency graph, and surface unused symbol warnings.
- **Runtime evaluation:** Embed the interpreter in a sandbox to execute selected code blocks (`solvrascript.runSelection` command) with captured stdout/stderr for quick feedback.

### Phase 4 – Tooling & Distribution

- **VSCode extension:** Package the LSP server with the TextMate grammar from `docs/syntax_highlighting.md`; provide settings UI for enabling legacy aliases.
- **Testing harness:** Create integration tests using `lsp-test` crate to validate diagnostics, completions, and formatting behaviours.
- **CI integration:** Add workflows that run `cargo test`, `cargo clippy`, and the LSP integration suite; publish binaries via GitHub Releases.
- **Telemetry & logging:** Add optional structured logging with `tracing` crate for debugging, guarded behind user consent in the LSP client.

### Deliverables

- Standalone `solvrascript-lsp` executable.
- VSCode extension bundling the server, grammar, and commands.
- Documentation updates covering configuration, troubleshooting, and SolvraCore workflows.
