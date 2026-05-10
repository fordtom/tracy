# Config (`tracy.toml`)

## Discovery

- Default: search for `tracy.toml` from the scan start upward
- Override: `--config path/to/tracy.toml`
- Disable: `--no-config`

The scan start is `--root` when provided, otherwise the current directory.

CLI overrides config.

## Example

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

## Keys

Top-level:

- `root` (string): scan root (relative paths resolved vs config dir)
- `format` (`json|jsonl|csv|sarif`)
- `output` (string)
- `quiet` (bool)
- `fail_on_empty` (bool)
- `include_git_meta` (bool)
- `include_blame` (bool): add per-match blame metadata when resolvable; omit it for untracked or otherwise unblamable files

`[scan]`:

- `slug` (string array)

`[filter]`:

- `include_vendored` (bool)
- `include_generated` (bool)
- `include_submodules` (bool)
- `git_attr_source` (`worktree|index`): Git attribute source for vendored/generated detection
- `include` (string array, glob)
- `exclude` (string array, glob)

Vendored/generated filtering is Git-backed. When Tracy needs to resolve those
attributes, the scan root must be inside a Git repository.
