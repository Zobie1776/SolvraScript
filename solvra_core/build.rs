use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let ffi_dir = manifest_dir.join("ffi");
    fs::create_dir_all(&ffi_dir).expect("create ffi dir");

    let header = r#"#ifndef SOLVRA_CORE_C_API_H
#define SOLVRA_CORE_C_API_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SolvraRuntime SolvraRuntime;

typedef enum {
    SOLVRA_STATUS_OK = 0,
    SOLVRA_STATUS_ERROR = 1
} SolvraStatus;

typedef struct {
    uint32_t tag;
    int64_t int_value;
    double float_value;
} SolvraValue;

SolvraRuntime* solvra_runtime_new(void);
SolvraStatus solvra_runtime_free(SolvraRuntime* runtime);
SolvraStatus solvra_runtime_execute(SolvraRuntime* runtime, const unsigned char* bytes, unsigned int len, SolvraValue* out_value);

#ifdef __cplusplus
}
#endif

#endif /* SOLVRA_CORE_C_API_H */
"#;

    fs::write(ffi_dir.join("c_api.h"), header).expect("write header");

    let json = serde_json::json!({
        "version": 1,
        "functions": [
            {
                "name": "solvra_runtime_new",
                "returns": "SolvraRuntime*",
                "arguments": []
            },
            {
                "name": "solvra_runtime_free",
                "returns": "SolvraStatus",
                "arguments": ["SolvraRuntime*"]
            },
            {
                "name": "solvra_runtime_execute",
                "returns": "SolvraStatus",
                "arguments": ["SolvraRuntime*", "const unsigned char*", "unsigned int", "SolvraValue*"]
            }
        ]
    });

    fs::write(
        ffi_dir.join("c_api.json"),
        serde_json::to_string_pretty(&json).unwrap(),
    )
    .expect("write json");

    println!("cargo:rerun-if-changed=build.rs");
}
