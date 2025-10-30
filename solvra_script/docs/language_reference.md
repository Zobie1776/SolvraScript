## SolvraScript Language Reference

### Statements and Keywords

| Construct | Purpose | Short Alias |
| --------- | ------- | ----------- |
| `let` | Declare an immutable binding. | – |
| `let mut` | Declare a mutable binding. | – |
| `const` | Declare a compile-time constant. | – |
| `fn` | Define a function. | – |
| `if` / `else` | Branching logic. | – |
| `while` | Loop with condition check. | – |
| `for … in` | Iterator-style loop. | – |
| `match` | Pattern matching with arms. | – |
| `return` | Exit the current function. | – |
| `break` / `continue` | Loop control. | – |
| `true` / `false` | Boolean literals. | – |
| `bool` | Boolean type keyword (primary). | `boolean` *(legacy)* |
| `string` | UTF-8 string type keyword. | – |
| `int` / `float` | Numeric type keywords. | – |

### Built-in Functions and Aliases

| Primary | Legacy Alias | Description |
| ------- | ------------- | ----------- |
| `prt(…values)` | `print(…values)` | Writes values exactly as provided without automatic spacing or newline. |
| `println(…values)` | – | Writes values without separators and appends a newline. |
| `endl()` | – | Emits a single newline and flushes stdout; pairs with `prt`. |
| `div(a, b)` | `division(a, b)` | Floating-point division with divide-by-zero protection. |
| `sbt(a, b)` | `subtract(a, b)` | Subtracts `b` from `a`; returns `int` when both operands are integers. |
| `bool(value)` | `boolean(value)` | Converts `value` to a SolvraScript boolean using truthiness rules. |
| `len(value)` | – | Length of strings/arrays/objects. |
| `type(value)` | – | Returns the SolvraScript type name. |
| `parse_int(text, base?)` | – | Parses integers with optional radix. |
| `parse_float(text)` | – | Parses floating point numbers. |
| `random(min?, max?)` | – | Random floats (0..1) or ints (min..max). |
| `time()` | – | Seconds since UNIX epoch. |
| `now()` | – | Structured UTC timestamp. |
| `sleep(ms)` | – | Suspends execution for `ms` milliseconds. |
| `push(array, value)` | – | Appends `value`; returns new array. |
| `pop(array)` | – | Removes last element. |
| `insert(array, index, value)` | – | Inserts `value` at `index`. |
| `remove(array, index)` | – | Removes element at `index`. |
| `on_event(name, handler)` | – | Registers handler for events. |
| `trigger_event(name, payload?)` | – | Triggers event; returns execution count. |
| `to_string(value)` | – | Stringifies using runtime display. |
| `env_get(key)` / `env_set(key, value)` | – | Environment variable helpers. |
| `http_get(url)` / `http_post(url, body, headers?)` | – | HTTP client helpers. |
| `core_module_execute(handle, entry?)` | – | Executes a compiled SolvraCore module linked via the import system. Optional entry selection is reserved for future expansion. |
| `core_module_release(handle)` | – | Releases a compiled module handle from the shared SolvraCore memory contract. |
| `core_value_release(handle)` | – | Disposes opaque SolvraCore values returned from modules. |
| `core_memory_stats()` | – | Reports deterministic allocator metrics (`capacity_bytes`, `used_bytes`, `allocations`). |

> **Spacing rule:** SolvraScript no longer inserts implicit spaces during string concatenation or printing. Include literal spaces in strings (e.g. `"Hello " + name`) to control layout.

### Modules and Imports

- `import <vector>;` loads a standard library module (`vector`, `string`, `io`, …).
- `import "lib/math.svs";` loads a script relative to the importing file or search paths.
- `import { append, length } from <vector>;` pulls named exports directly into scope.
- `import "tools/format.svs" as fmt;` binds the module namespace to `fmt` for dot access.

Modules expose all globals they define beyond the built-in set. See `docs/modules.md` for loader details, standard module inventory, and packaging guidance.

### Syntax Rules

**String Literals**

- Only double-quoted (`"…"`) strings are permitted.
- Recognised escape sequences: `\n`, `\t`, `\r`, `\0`, `\\`, and `\"`.
- Use template strings (`` `…` ``) for multi-line literals without escape expansion.
- Single quotes (`'`) are rejected to avoid ambiguity.

**Escape Sequences**

| Sequence | Meaning |
| -------- | ------- |
| `\n` | Line feed |
| `\t` | Horizontal tab |
| `\r` | Carriage return |
| `\0` | Null byte |
| `\\` | Literal backslash |
| `\"` | Double quote |

**Variables and Types**

- Variables default to immutable. Use `let mut` for mutability.
- Type annotations use postfix syntax: `let id: string = "value";`.
- Compound types: arrays (`[T]`), tuples (`(A, B)`), and objects (`{ key: value }`).

**Conditionals and Loops**

```solvrascript
let mut total = 0;
for value in numbers {
    if value > 0 {
        total = total + value;
    }
}
```

### Idiomatic Examples

```solvrascript
fn greet(name: string) {
    prt("Hello, ");
    prt(name);
    endl();
}

let first = 9;
let second = 3;

println("Quotient: ", to_string(div(first, second)));
println("Difference: ", to_string(sbt(first, second)));

let ready = bool(first);
if ready {
    println("Computation complete!\n");
}
```

### String Concatenation and Formatting

- Strings concatenate with `+`. Add spaces explicitly: `"A " + value + " B"`.
- Pair `prt` with `endl` for fine-grained console output.
- Prefer template strings for embedded expressions: `` `Result: ${div(x, y)}` ``.

### Escape-Sequence Testing Snippet

```solvrascript
println("Line 1\nLine 2");
println("Tabbed\tColumn");
println("Quote: \" and backslash: \\");
```

All lines above render with the intended control characters because the tokenizer resolves escape sequences during lexing.
