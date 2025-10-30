# SolvraCore `.svc` File Format

The SolvraCore runtime consumes a compact binary representation of Solvra bytecode.
`*.svc` files follow a strict layout so that the interpreter and ahead-of-time
backends can map instructions without additional metadata.  The container is
little-endian and uses the following top-level structure:

| Offset | Size | Description |
| --- | --- | --- |
| 0 | 4 bytes | Magic header `SVC1` |
| 4 | 2 bytes | Format version (currently `0x0001`) |
| 6 | 4 bytes | Entry function index |
| 10 | 4 bytes | Constant pool length (`N`) |
| 14 | ... | Constant pool records |
| ... | 4 bytes | Debug symbol table length (`D`) |
| ... | ... | Debug symbol entries |
| ... | 4 bytes | Function table length (`F`) |
| ... | ... | Function descriptors |

## Constant Pool Encoding

Each constant begins with a one-byte tag:

* `0x00` – `Null`
* `0x01` – Boolean (`0x00` false, `0x01` true)
* `0x02` – 64-bit signed integer
* `0x03` – 64-bit IEEE-754 float
* `0x04` – UTF-8 string (`u32` length prefix followed by bytes)

Constants are referenced by zero-based index from the instruction stream.

## Debug Symbols

Every debug record stores the origin of an instruction span:

```
[u32: file name byte length]
[bytes: UTF-8 file name]
[u32: 1-indexed line]
[u32: 1-indexed column]
```

The interpreter uses this table to build stack traces and to annotate
`SolvraError::RuntimeException` failures.

## Function Descriptors

Each function descriptor includes:

```
[u32: function name length]
[bytes: UTF-8 name]
[u16: arity]
[u16: local slot count]
[u32: instruction count]
[instruction bytes...]
```

Instructions are encoded as `[u8 opcode][u32 operand_a][u32 operand_b][u32 debug]`.
The debug slot stores an index into the debug symbol table or `u32::MAX` when the
instruction is synthetic.

## Textual Notation

For documentation and testing the repository ships a lightweight textual
notation that mirrors the binary structure.  The samples under
`solvra_core/samples/` use directives such as `.version`, `.entry`, `.const` and
`.fn` to describe the corresponding sections.  Each function body lists opcodes
with operands and labels (prefixed by `:`) to make control flow explicit.

This notation is intended for human-readable examples and test fixtures.  When
assembling for production the textual files must be lowered into the binary
representation described above, typically by the higher level SolvraScript
compiler or by bespoke tooling built with `solvra_core::bytecode::assembler`.

## Reference Samples

* `samples/hello_world.svc` – prints a greeting via `println`
* `samples/add_numbers.svc` – defines an `add` helper and returns `2 + 3`
* `samples/loop_counter.svc` – iterates from `0` to `5`, logging each value
* `samples/virtual_device_driver.svc` – registers a mock driver and exposes a
  polling entry point suitable for unit tests

These examples match the data layout above and exercise driver bindings,
control flow, and native calls.
