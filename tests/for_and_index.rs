#[path = "../src/tests/util.rs"]
mod util;

use std::fs;
use tempfile::tempdir;
use util::run_svs_source;

#[test]
fn for_loop_over_literal_list_prints_items() {
    let script = r#"
fn main() {
    for agent in ["alpha", "beta"] {
        println(agent);
    }
}

main();
"#;

    let output = run_svs_source(script);
    assert!(
        output.contains("alpha"),
        "expected alpha in output, got {output}"
    );
    assert!(
        output.contains("beta"),
        "expected beta in output, got {output}"
    );
}

#[test]
fn toml_load_file_handles_indexing() {
    let dir = tempdir().expect("create temp dir");
    let config_path = dir.path().join("models.toml");
    let contents = r#"
[agents.eolas]
provider = "openai"
model = "gpt-4o-mini"

[agents.aegis]
provider = "anthropic"
model = "claude-3"
"#;
    fs::write(&config_path, contents).expect("write config");

    let escaped = config_path.to_string_lossy().replace('\\', "\\\\");
    let script = format!(
        "fn main() {{\n    let cfg = toml::load_file(\"{escaped}\");\n    println(cfg[\"agents.eolas.provider\"]);\n    println(cfg[\"agents.aegis.model\"]);\n}}\n\nmain();\n"
    );

    let output = run_svs_source(&script);
    assert!(
        output.contains("openai"),
        "expected provider from toml, got {output}"
    );
    assert!(
        output.contains("claude-3"),
        "expected model from toml, got {output}"
    );
}
