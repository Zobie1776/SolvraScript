## SolvraScript Stdlib Migration & Directory Blueprint

### Goals
1. Consolidate existing `stdlib/` + `lib/` contents into the new `std/` (stable) and `stdx/` (extended) tree defined in the SolvraOS blueprint.
2. Provide compatibility shims in the legacy directories during the migration window, emitting runtime warnings and documenting new import paths.
3. Ensure documentation (language reference, modules) reflects updated imports, slice semantics, async behavior, and the new two-tier architecture.

### Current State
```
solvra_script/
  stdlib/
    crypto/
    data/
    devops/
    game/
    gfx/
    io.svs
    net/
    sec/
    storage/
    string.svs
    vector.svs
    web/
    docs/
  lib/
    ai/
    std/
```
Existing modules are scattered; imports target deprecated paths. `io.svs`, `string.svs`, `vector.svs` remain as standalone scripts.

### Migration Plan
1. **Phase 0 – Scaffolding**
   - Create `std/` and `stdx/` directories with `mod.svs` files exporting their submodules.
   - Move foundational files:
     - `io.svs` → `std/io/mod.svs`, `std/io/io.svs` (wrap `stdin`, `stdout`, `println!`).
     - `string.svs`, `vector.svs` → `std/core/` as part of the prelude.
   - Create `compat/legacy_shims/` containing:
     ```svs
     @deprecated("Use std.net.tcp instead")
     export std.net.tcp;
     ```
   - Adjust compiler module path order to search `std`, `stdx`, then `compat`.

2. **Phase 1 – Core Target Modules**
   - Populate `std/core`, `std/io`, `std/fs`, `std/math`, `std/time`.
   - Move `stdlib/net`, `stdlib/html`, `stdlib/web` contents into `std/net` and `std/http`.
   - Ensure WM intrinsics (slices, `core_index`) used in `std/io`, `std/fs`, `std/serialization`.
   - Update docs/reference + `docs/modules.md` to reference `std.*` imports.

3. **Phase 2 – Extended Domains**
   - Map `lib/ai` → `stdx/ai`; `stdlib/data` → `stdx/data`; `stdlib/devops` → `stdx/devops`; `stdlib/storage` → `stdx/storage`.
   - Create missing directories per blueprint (`stdx/gpu`, `stdx/cloud`, etc.) with top-level `mod.svs` containing TODO placeholders.
   - Provide compatibility shards in legacy locs referencing new paths with warnings logged once per session.

4. **Phase 3 – Documentation & Tooling**
   - Publish `docs/stdlib_migration.md` (this file) covering import changes, compat shims, and deprecation schedule.
   - Update `docs/language_reference.md` with slice semantics, async usage, module conventions (done).
   - Add migration codemods/scripts to rewrite `stdlib.foo` to `stdx.foo` (future work).

5. **Phase 4 – Clean-up**
   - After two release cycles (~6 months), remove legacy `stdlib/` and `lib/` directories entirely.
   - Ensure CI/test suites refer only to `std/` and `stdx/`.

### Import Updates
- Old: `import "stdlib/io.svs"` → New: `import std.io`.
- Old: `import <vector>` → New: `import std.core.vec`.
- Old: `import "lib/ai/tensor.svs"` → New: `import stdx.ai.tensor`.

### Resetting Module Resolution
1. `std/core` must load before any other std modules (prelude + `Vec`, `Map`, `Option`, `Result`).
2. Module loader checks: user dir → `std/` → `stdx/` → `compat/legacy_shims/`.
3. Introduce `std.core.prelude` auto-imported via compiler hook (already activated).

### Documentation Alignment
- All stdlib modules must mention their new path in doc comments (e.g., `/// @module std.net.tcp`).
- `docs/modules.md` to list the new hierarchy and compatibility shims with “@deprecated” notes.
- Existing README files under `stdlib/docs` should be moved/merged into `docs/` in repo root to minimize duplication.

### Next Steps (after this plan)
- Physically move files and update imports to match the tree.
- Implement `compat/legacy_shims` warnings.
- Extend CI to verify only `std/` and `stdx/` imports remain.
