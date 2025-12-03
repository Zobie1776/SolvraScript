//==============================================
// File: tests/path.rs
// Author: Codex
// License: Duality Public License (DPL v1.0)
// Goal: Run stdx path .svs fixtures
// Objective: Validate path join/split/normalize helpers
//==============================================

use solvrascript::runtime::run_svs_test;

#[test]
fn path_helpers_pass() {
    run_svs_test("stdx_tests/path_test.svs");
}

//==============================================
// End of file
//==============================================
