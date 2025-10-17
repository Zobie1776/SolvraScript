# echo

## Synopsis
`echo [WORDS...]`

## Description
Write the provided words to standard output followed by a newline. When used in a pipeline, `echo` also echoes piped input when no explicit arguments are supplied.

## Examples
- `echo hello world` — print `hello world`.
- `cat file.txt | echo` — repeat the contents of `file.txt`.
