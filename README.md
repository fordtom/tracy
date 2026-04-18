# Tracy

Scans codebases for requirement references in comments and outputs JSON.

Docs:

- [CLI](docs/cli.md)
- [Config](docs/config.md)

## Getting Started

```bash
tracy --slug REQ --root .
```

If `tracy.toml` is present, Tracy loads it by default. When `--root` is set, config discovery starts from that path and walks upward; otherwise discovery starts from the current directory. CLI flags override config.

Finds `{SLUG}-{NUMBER}` formatted references in comments across your codebase, returning JSON keyed by requirement id. Repeat `--slug` to match multiple prefixes.

Example (single hit):

```rust
// src/lib.rs
// REQ-1: validate input
```

```json
{
  "REQ-1": [
    {
      "file": "src/lib.rs",
      "line": 1,
      "comment_text": "// REQ-1: validate input"
    }
  ]
}
```

Each entry may also include `above`, `below`, `inline`, and `scope` context fields when available.
When `--include-blame` is enabled, Tracy adds `blame` metadata when `git blame`
can resolve that line. Matches in untracked files or other unblamable paths are
still returned, but without a `blame` field.

`--output` writes the same report to a file. Tracy still prints to stdout unless
you also pass `--quiet`.

See [docs/cli.md](docs/cli.md) for the full CLI reference and [docs/config.md](docs/config.md) for the full `tracy.toml` schema.

## Supported Languages

Vendored/generated filtering is Git-backed. If Tracy needs to resolve those
attributes, the scan root must be inside a Git repository.

All languages supported by [ast-grep](https://ast-grep.github.io/guide/introduction.html#supported-languages), including Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, and more.

## License

MIT
