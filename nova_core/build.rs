use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let ffi_dir = manifest_dir.join("ffi");
    fs::create_dir_all(&ffi_dir).expect("create ffi dir");

    let header = r#"#ifndef NOVA_CORE_C_API_H
#define NOVA_CORE_C_API_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NovaRuntime NovaRuntime;

typedef enum {
    NOVA_STATUS_OK = 0,
    NOVA_STATUS_ERROR = 1
} NovaStatus;

typedef struct {
    uint32_t tag;
    int64_t int_value;
    double float_value;
} NovaValue;

NovaRuntime* nova_runtime_new(void);
NovaStatus nova_runtime_free(NovaRuntime* runtime);
NovaStatus nova_runtime_execute(NovaRuntime* runtime, const unsigned char* bytes, unsigned int len, NovaValue* out_value);

#ifdef __cplusplus
}
#endif

#endif /* NOVA_CORE_C_API_H */
"#;

    fs::write(ffi_dir.join("c_api.h"), header).expect("write header");

    let json = serde_json::json!({
        "version": 1,
        "functions": [
            {
                "name": "nova_runtime_new",
                "returns": "NovaRuntime*",
                "arguments": []
            },
            {
                "name": "nova_runtime_free",
                "returns": "NovaStatus",
                "arguments": ["NovaRuntime*"]
            },
            {
                "name": "nova_runtime_execute",
                "returns": "NovaStatus",
                "arguments": ["NovaRuntime*", "const unsigned char*", "unsigned int", "NovaValue*"]
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
