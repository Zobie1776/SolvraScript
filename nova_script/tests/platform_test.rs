//=============================================
// nova_script/tests/platform_test.rs
//=============================================
// Author: NovaOS Contributors
// License: MIT (see LICENSE)
// Goal: Cross-platform integration tests
// Objective: Verify platform layer works on all targets
// Formatting: Zobie.format (.novaformat)
//=============================================

use novascript::platform;
use std::time::Duration;

#[test]
fn test_system_time() {
let time = platform::system_time().expect(“system_time should work”);
assert!(