//==============================================
// File: tests/math.rs
// Author: Codex
// License: Duality Public License (DPL v1.0)
// Goal: Run stdx math .svs fixtures
// Objective: Validate core, trig, transcendental, and random helpers
//==============================================

use solvrascript::runtime::run_svs_test;

#[test]
fn test_math_core() {
    run_svs_test("stdx_tests/math_core_test.svs");
}

#[test]
fn test_math_trig() {
    run_svs_test("stdx_tests/math_trig_test.svs");
}

#[test]
fn test_math_transcendental() {
    run_svs_test("stdx_tests/math_transcendental_test.svs");
}

#[test]
fn test_math_random() {
    run_svs_test("stdx_tests/math_random_test.svs");
}

// @ZNOTE: Run with `--test-threads=1` to avoid parallel math allocations.

//==============================================
// End of file
//==============================================
