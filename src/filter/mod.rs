pub mod args;
mod error;
mod git_attrs;

pub use args::{FilterArgs, GitAttrSource};
pub use error::FilterError;

use git_attrs::resolve_git_attributes;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

struct GlobFilters {
    include: Vec<glob::Pattern>,
    exclude: Vec<glob::Pattern>,
}

pub fn collect_files(root: &Path, args: &FilterArgs) -> Result<Vec<PathBuf>, FilterError> {
    let filters = parse_globs(args)?;
    let mut candidates = Vec::new();

    for entry in WalkBuilder::new(root)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(!args.include_submodules)
        .build()
    {
        let entry = entry?;

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.path();
        if is_glob_excluded(path, root, &filters) {
            continue;
        }

        candidates.push(path.to_path_buf());
    }

    if args.include_vendored && args.include_generated {
        return Ok(candidates);
    }

    let attr_flags = resolve_git_attributes(root, &candidates, args)?;
    Ok(candidates
        .into_iter()
        .filter(|path| {
            let excluded = attr_flags.get(path).is_some_and(|flags| {
                (!args.include_vendored && flags.vendored)
                    || (!args.include_generated && flags.generated)
            });
            !excluded
        })
        .collect())
}

fn parse_globs(args: &FilterArgs) -> Result<GlobFilters, FilterError> {
    let include = args
        .include
        .iter()
        .map(|g| {
            glob::Pattern::new(g).map_err(|e| FilterError::InvalidGlob {
                pattern: g.clone(),
                source: e,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let exclude = args
        .exclude
        .iter()
        .map(|g| {
            glob::Pattern::new(g).map_err(|e| FilterError::InvalidGlob {
                pattern: g.clone(),
                source: e,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(GlobFilters { include, exclude })
}

fn is_glob_excluded(path: &Path, root: &Path, filters: &GlobFilters) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let relative_str = relative.to_string_lossy();

    if !filters.include.is_empty() && !filters.include.iter().any(|p| p.matches(&relative_str)) {
        return true;
    }

    filters.exclude.iter().any(|p| p.matches(&relative_str))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn include_globs_filter_paths() {
        let filters = parse_globs(&FilterArgs {
            include: vec!["src/**".to_string()],
            ..Default::default()
        })
        .unwrap();
        let root = Path::new("/repo");
        let path_in = Path::new("/repo/src/main.rs");
        let path_out = Path::new("/repo/tests/test.rs");

        assert!(!is_glob_excluded(path_in, root, &filters));
        assert!(is_glob_excluded(path_out, root, &filters));
    }

    #[test]
    fn exclude_globs_filter_paths() {
        let filters = parse_globs(&FilterArgs {
            exclude: vec!["src/gen/**".to_string()],
            ..Default::default()
        })
        .unwrap();
        let root = Path::new("/repo");
        let path_in = Path::new("/repo/src/main.rs");
        let path_out = Path::new("/repo/src/gen/types.rs");

        assert!(!is_glob_excluded(path_in, root, &filters));
        assert!(is_glob_excluded(path_out, root, &filters));
    }

    #[test]
    fn collect_files_requires_git_repo_for_attribute_filtering() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "// REQ-1\n").unwrap();

        let err = collect_files(dir.path(), &FilterArgs::default()).unwrap_err();
        assert!(matches!(err, FilterError::GitRepoRequired { .. }));
    }
}
