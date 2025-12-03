//==============================================
// File: tests/string.rs
// Author: Codex
// License: Duality Public License (DPL v1.0)
// Goal: Run stdx string .svs fixtures
// Objective: Validate parsing and formatting helpers
//==============================================

use solvrascript::runtime::run_svs_test;

#[test]
fn string_helpers_pass() {
    run_svs_test("stdx_tests/string_test.svs");
}

//==============================================
// End of file
//==============================================
