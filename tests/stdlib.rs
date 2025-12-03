//==============================================
// File: tests/stdlib.rs
// Author: Codex
// License: Duality Public License (DPL v1.0)
// Goal: Core stdlib regression runner
// Objective: Execute stdx core fixture via shared SVS harness
//==============================================

use solvrascript::runtime::run_svs_test;

//==============================================
// Section 1.0 - Core coverage
//==============================================

#[test]
fn stdlib_core_behaves() {
    run_svs_test("stdx_tests/core_test.svs");
}
