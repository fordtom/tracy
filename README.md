# Tracy

Scans codebases for requirement references in comments and outputs JSON.

Docs:

- [CLI](docs/cli.md)
- [Config](docs/config.md)

## Usage

```bash
tracy --slug REQ --root .
```

If `tracy.toml` is present (searched from the current directory upwards), Tracy will load it by default. CLI flags override config.

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

## Options

| Flag                   | Description                                    |
| ---------------------- | ---------------------------------------------- |
| `--slug`, `-s`         | Slug pattern to match (e.g., `REQ`, `LIN`)     |
| `--root`               | Root directory to scan (default: config dir or `.`) |
| `--format`             | Output format (`json`, `jsonl`, `csv`, `sarif`) |
| `--config`             | Path to config file (default: search for `tracy.toml`) |
| `--no-config`          | Disable config file loading                    |
| `--output`, `-o`       | Write output to file                           |
| `--quiet`, `-q`        | Suppress stdout output                         |
| `--fail-on-empty`      | Exit with error if no matches found            |
| `--include-git-meta`   | Include git repository metadata in output      |
| `--include-blame`      | Include git blame metadata when resolvable; omit it for untracked or otherwise unblamable files |
| `--include-vendored`   | Include files marked `linguist-vendored` by Git |
| `--include-generated`  | Include files marked `linguist-generated` by Git |
| `--include-submodules` | Include git submodules                         |
| `--git-attr-source`    | Git attribute source for vendored/generated detection (`worktree` or `index`) |
| `--include`            | Only include paths matching this glob (repeatable) |
| `--exclude`            | Exclude paths matching this glob (repeatable)  |

## Config

Create a `tracy.toml` at your repo root (or pass `--config path`):

```toml
format = "sarif"
include_git_meta = true
include_blame = true

[scan]
slug = ["REQ"]

[filter]
git_attr_source = "index"
include = ["src/**"]
exclude = ["**/generated/**"]
```

## Supported Languages

Vendored/generated filtering is Git-backed. If Tracy needs to resolve those
attributes, the scan root must be inside a Git repository.

All languages supported by [ast-grep](https://ast-grep.github.io/guide/introduction.html#supported-languages), including Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, and more.

## License

MIT
