# ns

## Synopsis
`ns { ... }`

## Description
Execute a multi-line NovaScript block inline within the shell. The block is evaluated using the embedded NovaScript interpreter and any printed output streams to the terminal.

## Examples
- `ns { let x: int = 2; print(x * 5); }` — run NovaScript logic in-place.
