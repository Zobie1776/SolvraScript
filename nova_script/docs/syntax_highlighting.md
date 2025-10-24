## NovaScript TextMate Grammar Quickstart

### Minimal Grammar Snippet (JSON)

```jsonc
{
  "scopeName": "source.novascript",
  "patterns": [
    { "include": "#comments" },
    { "include": "#strings" },
    { "include": "#numbers" },
    { "include": "#keywords" }
  ],
  "repository": {
    "comments": {
      "patterns": [
        { "name": "comment.line.double-slash.novascript", "match": "//.*$" }
      ]
    },
    "strings": {
      "patterns": [
        {
          "name": "string.quoted.double.novascript",
          "begin": "\"",
          "end": "\"",
          "patterns": [
            { "name": "constant.character.escape.novascript", "match": "\\\\[ntr0\"\\]" }
          ]
        },
        {
          "name": "string.quoted.template.novascript",
          "begin": "`",
          "end": "`",
          "patterns": [
            {
              "name": "source.novascript.interpolated",
              "begin": "\\$\\{",
              "end": "}",
              "patterns": [{ "include": "source.novascript" }]
            }
          ]
        }
      ]
    },
    "numbers": {
      "patterns": [
        { "name": "constant.numeric.novascript", "match": "\\b(?:0x[0-9a-fA-F]+|[0-9]+(?:\\.[0-9]+)?)\\b" }
      ]
    },
    "keywords": {
      "patterns": [
        {
          "name": "keyword.control.novascript",
          "match": "\\b(?:let|mut|const|fn|if|else|while|for|in|match|return|break|continue|try|catch|async|await)\\b"
        },
        {
          "name": "support.function.builtin.novascript",
          "match": "\\b(?:prt|print|println|endl|div|division|sbt|subtract|bool|boolean|len|type|parse_int|parse_float|random|time|now|sleep|push|pop|insert|remove|to_string|env_get|env_set|http_get|http_post|on_event|trigger_event)\\b"
        }
      ]
    }
  }
}
```

### Installation Instructions (VSCode)

1. Create a folder such as `.nova-vscode/syntax` in your workspace.
2. Save the JSON snippet above as `novascript.tmLanguage.json` inside that folder.
3. Add a `package.json` to define a lightweight extension:

```json
{
  "name": "novascript-syntax",
  "displayName": "NovaScript Syntax",
  "version": "0.0.1",
  "engines": { "vscode": "^1.80.0" },
  "contributes": {
    "grammars": [
      {
        "language": "novascript",
        "scopeName": "source.novascript",
        "path": "./novascript.tmLanguage.json"
      }
    ],
    "languages": [
      {
        "id": "novascript",
        "aliases": ["NovaScript", "novascript"],
        "extensions": [".ns"],
        "configuration": "./language-configuration.json"
      }
    ]
  }
}
```

4. Provide a minimal language configuration for bracket/brace pairing as `language-configuration.json`:

```json
{
  "comments": { "lineComment": "//" },
  "brackets": [["{", "}"], ["[", "]"], ["(", ")"]],
  "autoClosingPairs": [
    { "open": "{", "close": "}" },
    { "open": "[", "close": "]" },
    { "open": "(", "close": ")" },
    { "open": "\"", "close": "\"", "notIn": ["string"] },
    { "open": "`", "close": "`", "notIn": ["string"] }
  ]
}
```

5. Run `code --install-extension <path-to-folder>` or use the VSCode `Developer: Install Extension from Location…` command to install the local package.
6. Open any `.ns` file and run `Developer: Reload Window` to activate highlighting.
7. Verify highlighting by checking that keywords, built-ins, numbers, comments, and interpolated template strings adopt theme-specific colours.

### Testing Tips

- Use the `Developer: Inspect Editor Tokens and Scopes` command in VSCode to confirm token scopes.
- Pair the grammar with the sample code from `docs/language_reference.md` to validate escape sequence colouring and bracket matching.
- Incrementally extend the `support.function.builtin.novascript` pattern as new built-ins are added.
