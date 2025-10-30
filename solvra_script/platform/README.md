# SolvraScript

**A modern, cross-platform scripting language for the SolvraLang ecosystem**

SolvraScript is a lightweight, expressive scripting language designed for embedded systems, operating system development, and general-purpose scripting. It features a clean syntax, powerful runtime capabilities, and seamless cross-platform support across Linux, macOS, Windows, and SolvraOS.

## 🌟 Features

- **Cross-Platform**: Runs natively on Linux, macOS, Windows, and SolvraOS
- **Modern Syntax**: Clean, intuitive language design with type inference
- **Module System**: Import standard library and custom modules with ease
- **Hardware Abstraction**: Direct HAL integration for embedded development
- **Event-Driven**: Built-in event system for reactive programming
- **Network Support**: HTTP client built into the runtime
- **File I/O**: Comprehensive file system operations
- **Process Control**: Execute and spawn external processes

## 📦 Supported Platforms

|Platform   |Status          |Target Triple                                |
|-----------|----------------|---------------------------------------------|
|**Linux**  |✅ Stable        |`x86_64-unknown-linux-gnu`                   |
|**macOS**  |✅ Stable        |`aarch64-apple-darwin`, `x86_64-apple-darwin`|
|**Windows**|✅ Stable        |`x86_64-pc-windows-msvc`                     |
|**SolvraOS** |🚧 In Development|`x86_64-solvraos`                              |

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/solvraos/solvra_script.git
cd solvra_script

# Build for your platform
cargo build --release

# Run the CLI
cargo run -- examples/hello.svs
```

### Your First Script

Create `hello.svs`:

```solvrascript
// Hello World in SolvraScript
println("Hello, SolvraScript!");

let name = input("What's your name? ");
println("Welcome, " + name + "!");
```

Run it:

```bash
solvrascript hello.svs
```

## 🏗️ Building for Different Platforms

### Linux

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

### macOS

```bash
# Intel Macs
cargo build --release --target x86_64-apple-darwin

# Apple Silicon
cargo build --release --target aarch64-apple-darwin
```

### Windows

```bash
cargo build --release --target x86_64-pc-windows-msvc
```

### SolvraOS (Future)

```bash
# Requires SolvraOS toolchain
cargo build --release --target x86_64-solvraos
```

## 🔧 Cross-Platform Architecture

SolvraScript uses a clean abstraction layer to isolate platform-specific code:

```
solvra_script/
├── platform/
│   ├── mod.rs          # Platform trait definition & public API
│   ├── sys_linux.rs    # Linux/POSIX implementation
│   ├── sys_windows.rs  # Windows/Win32 implementation
│   ├── sys_macos.rs    # macOS/Darwin implementation
│   └── sys_solvraos.rs   # SolvraOS native syscalls (in development)
├── interpreter.rs      # Platform-agnostic runtime
├── parser.rs          # Platform-agnostic parser
├── tokenizer.rs       # Platform-agnostic lexer
└── main.rs            # CLI entry point
```

### Platform Abstraction

All OS-dependent operations go through the `platform` module:

```rust
use crate::platform;

// Cross-platform file operations
let content = platform::read_file("config.svs")?;
platform::write_file("output.txt", &result)?;

// Cross-platform time
let timestamp = platform::system_time()?;

// Cross-platform I/O
platform::println("Hello from any OS!")?;
```

### Conditional Compilation

Platform-specific code is selected at compile time:

```rust
#[cfg(target_os = "linux")]
pub use sys_linux::LinuxPlatform as NativePlatform;

#[cfg(target_os = "windows")]
pub use sys_windows::WindowsPlatform as NativePlatform;

#[cfg(target_os = "macos")]
pub use sys_macos::MacOSPlatform as NativePlatform;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub use sys_solvraos::SolvraOSPlatform as NativePlatform;
```

## 📚 Language Features

### Variables and Constants

```solvrascript
let x = 42;              // Immutable binding
let mut y = 10;          // Mutable binding
const LIMIT = 100;       // Compile-time constant
```

### Functions

```solvrascript
fn fibonacci(n) {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

println("Fib(10) = " + fibonacci(10));
```

### Lambdas and Closures

```solvrascript
let offset = 5;
let add_offset = lambda |x| -> x + offset;
println("Result: " + add_offset(10));  // Prints: Result: 15
```

### Arrays and Objects

```solvrascript
let numbers = [1, 2, 3, 4, 5];
let user = {
    name: "Alice",
    age: 30,
    active: true
};

println("First: " + numbers[0]);
println("Name: " + user.name);
```

### Module System

```solvrascript
// Import standard library
import <vector>;

let mut data = vector.make();
data = vector.append(data, 42);
println("Length: " + vector.length(data));

// Import local module
import "utils/math.svs" as math;
println("Square: " + math.square(7));
```

### Hardware Abstraction Layer

```solvrascript
// List available devices
let devices = hal_devices();
for device in devices {
    println("Device: " + device.name);
}

// Read/write registers
hal_write("keyboard", 0, 0x01);
let status = hal_read("keyboard", 1);
```

### Event System

```solvrascript
// Register event handler
on_event("tick", fn handle_tick(data) {
    println("Tick: " + data.count);
});

// Trigger event
trigger_event("tick", { count: 42 });
```

### HTTP Client

```solvrascript
// GET request
let response = http_get("https://api.example.com/data");
println(json_stringify(response));

// POST request
let data = { user: "alice", action: "login" };
let result = http_post("https://api.example.com/login", data);
```

## 🧪 Running Tests

```bash
# Run all tests
cargo test --all

# Run platform-specific tests
cargo test --test platform_test

# Run integration tests
cargo test --test imports
```

## 📖 Documentation

- [Language Reference](docs/language_reference.md) - Complete syntax and semantics
- [Module System](docs/modules.md) - Import system and standard library
- [Built-in Functions](docs/builtin_status.md) - Runtime function inventory
- [Syntax Highlighting](docs/syntax_highlighting.md) - Editor integration guide
- [LSP Plan](docs/lsp_plan.md) - Language server roadmap

## 🔮 Future Integration with SolvraCore

SolvraScript is designed to integrate seamlessly with **SolvraCore**, the low-level bytecode execution engine for SolvraOS. The platform abstraction layer will be replaced with direct SolvraCore syscalls when running on SolvraOS:

```rust
// Future: SolvraOS native implementation
#[cfg(target_os = "solvraos")]
fn system_time() -> PlatformResult<f64> {
    solvra_syscall!(SYS_TIME, 0)  // Direct kernel syscall
}
```

### SolvraCore Integration Roadmap

1. ✅ **Phase 1**: Cross-platform abstraction layer (Complete)
1. 🚧 **Phase 2**: SolvraOS syscall definitions (In Progress)
1. ⏳ **Phase 3**: SolvraCore bytecode compilation
1. ⏳ **Phase 4**: Native SolvraOS runtime

## 🛠️ Development

### Project Structure

```
solvra_script/
├── Cargo.toml          # Project manifest
├── lib.rs              # Library root
├── main.rs             # CLI entry point
├── ast.rs              # Abstract syntax tree
├── parser.rs           # Recursive descent parser
├── tokenizer.rs        # Lexical analyzer
├── interpreter.rs      # Runtime evaluator
├── modules.rs          # Module loader
├── platform/           # Cross-platform abstraction
│   ├── mod.rs
│   ├── sys_linux.rs
│   ├── sys_windows.rs
│   ├── sys_macos.rs
│   └── sys_solvraos.rs
├── stdlib/             # Standard library modules
│   ├── vector.svs
│   ├── string.svs
│   └── io.svs
├── examples/           # Example scripts
├── tests/              # Integration tests
└── docs/               # Documentation
```

### Adding New Platform Operations

1. Add method to `PlatformOps` trait in `platform/mod.rs`
1. Implement for each platform in `sys_*.rs` files
1. Add public function wrapper in `platform/mod.rs`
1. Update interpreter to use platform API

Example:

```rust
// 1. Add to trait
pub trait PlatformOps {
    fn my_new_operation(param: &str) -> PlatformResult<String>;
}

// 2. Implement per-platform
#[cfg(target_os = "linux")]
impl PlatformOps for LinuxPlatform {
    fn my_new_operation(param: &str) -> PlatformResult<String> {
        // Linux-specific implementation
    }
}

// 3. Add public wrapper
pub fn my_new_operation(param: &str) -> PlatformResult<String> {
    NativePlatform::my_new_operation(param)
}
```

### Code Style

SolvraScript follows the `.solvraformat` standard:

```rust
//=============================================
// solvra_script/myfile.rs
//=============================================
// Author: SolvraOS Contributors
// License: MIT (see LICENSE)
// Goal: Brief description
// Objective: Specific purpose
// Formatting: Zobie.format (.solvraformat)
//=============================================

//=============================================
//            Section 1: Description
//=============================================

// Implementation...
```

## 🤝 Contributing

We welcome contributions! Please follow these guidelines:

1. **Code Quality**: Follow Rust best practices and `.solvraformat` style
1. **Cross-Platform**: Test on all supported platforms
1. **Documentation**: Update relevant docs and examples
1. **Tests**: Add tests for new features
1. **Compatibility**: Maintain backward compatibility

## 📄 License

SolvraScript is licensed under the MIT License. See <LICENSE> for details.

## 🙏 Acknowledgments

- Built on the Rust programming language
- Part of the SolvraLang ecosystem
- Inspired by modern scripting languages

## 📞 Contact

- **Project**: [github.com/solvraos/solvra_script](https://github.com/solvraos/solvra_script)
- **Issues**: [github.com/solvraos/solvra_script/issues](https://github.com/solvraos/solvra_script/issues)
- **Discussions**: [github.com/solvraos/solvra_script/discussions](https://github.com/solvraos/solvra_script/discussions)

-----

**SolvraScript** - Write once, run anywhere. From embedded systems to the cloud.
