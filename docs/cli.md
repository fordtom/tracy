# CLI

## Basics

```bash
tracy --slug REQ --root .
```

## Output formats

- `--format json` (default): JSON object keyed by requirement id
- `--format jsonl`: JSON Lines stream (`type=meta` then `type=match`)
- `--format csv`: CSV rows (one match per row)
- `--format sarif`: SARIF 2.1.0 (for GitHub code scanning, editors)

## Common flags

- `--slug/-s <SLUG>` (repeatable): requirement prefixes, e.g. `REQ`, `LIN`
- `--root <DIR>`: scan root (default: config dir or `.`)
- `--output/-o <PATH>`: write output file (still prints unless `--quiet`)
- `--quiet/-q`: suppress stdout
- `--fail-on-empty`: exit non-zero if no matches found

## Git metadata (optional)

- `--include-git-meta`: top-level `meta` in JSON; extra columns in CSV; run-level properties in SARIF
- `--include-blame`: per-match `blame` object (commit/author/time/summary) when resolvable; omitted for untracked or otherwise unblamable files

## Filtering

- `--include <GLOB>` (repeatable): allowlist
- `--exclude <GLOB>` (repeatable): blocklist
- `--include-vendored`: include files marked `linguist-vendored`
- `--include-generated`: include files marked `linguist-generated`
- `--include-submodules`: include submodules
- `--git-attr-source <worktree|index>`: resolve vendored/generated attributes from the working tree (default) or Git index

Vendored/generated filtering is Git-backed. When Tracy needs to resolve those
attributes, the scan root must be inside a Git repository.

## Examples

SARIF for PR annotations:

```bash
tracy -s REQ --format sarif --output tracy.sarif
```

JSONL for streaming ingestion:

```bash
tracy -s REQ --format jsonl --include-git-meta
```
