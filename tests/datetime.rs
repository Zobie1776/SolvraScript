//==============================================
// File: tests/datetime.rs
// Author: Codex
// License: Duality Public License (DPL v1.0)
// Goal: Run stdx datetime .svs fixtures
// Objective: Validate epoch formatting helpers
//==============================================

use solvrascript::runtime::run_svs_test;

#[test]
fn datetime_helpers_pass() {
    run_svs_test("stdx_tests/datetime_test.svs");
}

//==============================================
// End of file
//==============================================
