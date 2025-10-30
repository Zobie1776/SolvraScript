# ns

## Synopsis
`ns { ... }`

## Description
Execute a multi-line SolvraScript block inline within the shell. The block is evaluated using the embedded SolvraScript interpreter and any printed output streams to the terminal.

## Examples
- `ns { let x: int = 2; print(x * 5); }` — run SolvraScript logic in-place.
