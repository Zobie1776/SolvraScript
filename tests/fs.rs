//==============================================
// File: tests/fs.rs
// Author: Codex
// License: Duality Public License (DPL v1.0)
// Goal: Run stdx fs .svs fixtures
// Objective: Validate filesystem helpers and path handling
//==============================================

use solvrascript::runtime::run_svs_test;

#[test]
fn fs_helpers_pass() {
    run_svs_test("stdx_tests/fs_test.svs");
}

//==============================================
// End of file
//==============================================
