use crate::filter::{FilterArgs, FilterError, GitAttrSource};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const GIT_ATTR_BATCH_SIZE: usize = 10_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct AttrFlags {
    pub(super) vendored: bool,
    pub(super) generated: bool,
}

pub(super) fn resolve_git_attributes(
    root: &Path,
    candidates: &[PathBuf],
    args: &FilterArgs,
) -> Result<HashMap<PathBuf, AttrFlags>, FilterError> {
    if candidates.is_empty() {
        return Ok(HashMap::new());
    }

    let cwd = std::env::current_dir()?;
    let root_abs = absolutize(root, &cwd);
    let root_repo = git_toplevel(root).map_err(|_| FilterError::GitRepoRequired {
        root: root_abs.clone(),
    })?;

    let groups = group_paths_by_repo(&root_abs, candidates, Some(&root_repo), args, &cwd)?;

    let mut attr_flags = HashMap::new();
    for (repo_root, files) in groups {
        query_repo_attributes(&repo_root, &files, args.git_attr_source(), &mut attr_flags)?;
    }

    Ok(attr_flags)
}

fn group_paths_by_repo(
    root_abs: &Path,
    candidates: &[PathBuf],
    root_repo: Option<&Path>,
    args: &FilterArgs,
    cwd: &Path,
) -> Result<BTreeMap<PathBuf, Vec<(PathBuf, String)>>, FilterError> {
    let mut groups: BTreeMap<PathBuf, Vec<(PathBuf, String)>> = BTreeMap::new();
    let mut repo_cache: HashMap<PathBuf, Option<PathBuf>> = HashMap::new();

    for path in candidates {
        let path_abs = absolutize(path, cwd);
        let repo_root =
            find_repo_root_for_path(root_abs, &path_abs, root_repo, args, &mut repo_cache)?;
        let Some(repo_root) = repo_root else {
            continue;
        };

        let Ok(relative) = path_abs.strip_prefix(&repo_root) else {
            continue;
        };

        groups
            .entry(repo_root)
            .or_default()
            .push((path.clone(), git_path(relative)));
    }

    Ok(groups)
}

fn find_repo_root_for_path(
    root_abs: &Path,
    path_abs: &Path,
    root_repo: Option<&Path>,
    args: &FilterArgs,
    repo_cache: &mut HashMap<PathBuf, Option<PathBuf>>,
) -> Result<Option<PathBuf>, FilterError> {
    if !args.include_submodules {
        return Ok(root_repo.map(Path::to_path_buf));
    }

    let ceiling = root_repo.unwrap_or(root_abs);
    let Some(start_dir) = path_abs.parent() else {
        return Ok(root_repo.map(Path::to_path_buf));
    };

    let mut visited = Vec::new();
    let mut current = Some(start_dir);

    while let Some(dir) = current {
        if let Some(found) = repo_cache.get(dir) {
            let found = found.clone();
            for visited_dir in visited {
                repo_cache.insert(visited_dir, found.clone());
            }
            return Ok(found);
        }

        visited.push(dir.to_path_buf());

        if dir.join(".git").exists() {
            let repo_root = Some(git_toplevel(dir)?);
            for visited_dir in visited {
                repo_cache.insert(visited_dir, repo_root.clone());
            }
            return Ok(repo_root);
        }

        if dir == ceiling {
            break;
        }

        current = dir.parent();
    }

    let resolved = root_repo
        .filter(|repo| path_abs.starts_with(repo))
        .map(Path::to_path_buf);
    for visited_dir in visited {
        repo_cache.insert(visited_dir, resolved.clone());
    }
    Ok(resolved)
}

fn query_repo_attributes(
    repo_root: &Path,
    files: &[(PathBuf, String)],
    source: GitAttrSource,
    attr_flags: &mut HashMap<PathBuf, AttrFlags>,
) -> Result<(), FilterError> {
    for chunk in files.chunks(GIT_ATTR_BATCH_SIZE) {
        let output = git_check_attr(repo_root, chunk, source)?;
        let parsed = parse_check_attr_output(&output)?;

        let path_lookup: HashMap<&str, &PathBuf> = chunk
            .iter()
            .map(|(path, relative)| (relative.as_str(), path))
            .collect();

        for (path, attr, value) in parsed {
            let Some(original_path) = path_lookup.get(path.as_str()) else {
                continue;
            };

            let flags = attr_flags.entry((**original_path).clone()).or_default();
            match attr.as_str() {
                "linguist-vendored" => flags.vendored = git_attr_value_is_truthy(&value),
                "linguist-generated" => flags.generated = git_attr_value_is_truthy(&value),
                _ => {}
            }
        }
    }

    Ok(())
}

fn git_check_attr(
    repo_root: &Path,
    files: &[(PathBuf, String)],
    source: GitAttrSource,
) -> Result<Vec<u8>, FilterError> {
    let args = check_attr_args(source);

    let mut command = Command::new("git");
    command.arg("-C").arg(repo_root).args(&args);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| FilterError::GitCommandFailed {
                cmd: git_command_string(repo_root, &args),
                stderr: "failed to open stdin".to_string(),
            })?;

        for (_, relative) in files {
            stdin.write_all(relative.as_bytes())?;
            stdin.write_all(&[0])?;
        }
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(FilterError::GitCommandFailed {
            cmd: git_command_string(repo_root, &args),
            stderr: String::from_utf8(output.stderr)?.trim().to_string(),
        });
    }

    Ok(output.stdout)
}

fn check_attr_args(source: GitAttrSource) -> Vec<&'static str> {
    let mut args = vec!["check-attr", "--stdin", "-z"];
    if matches!(source, GitAttrSource::Index) {
        args.push("--cached");
    }
    args.extend(["linguist-vendored", "linguist-generated"]);
    args
}

fn parse_check_attr_output(output: &[u8]) -> Result<Vec<(String, String, String)>, FilterError> {
    let fields = output
        .split(|b| *b == 0)
        .filter(|field| !field.is_empty())
        .map(|field| String::from_utf8(field.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;

    let mut parsed = Vec::new();
    for chunk in fields.chunks_exact(3) {
        parsed.push((chunk[0].clone(), chunk[1].clone(), chunk[2].clone()));
    }

    if fields.len() % 3 != 0 {
        return Err(FilterError::GitCommandFailed {
            cmd: "git check-attr --stdin -z linguist-vendored linguist-generated".to_string(),
            stderr: "unexpected git check-attr output".to_string(),
        });
    }

    Ok(parsed)
}

fn git_attr_value_is_truthy(value: &str) -> bool {
    !matches!(value, "unset" | "unspecified" | "false" | "0" | "no")
}

fn git_toplevel(path: &Path) -> Result<PathBuf, FilterError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        return Err(FilterError::GitCommandFailed {
            cmd: git_command_string(path, &["rev-parse", "--show-toplevel"]),
            stderr: String::from_utf8(output.stderr)?.trim().to_string(),
        });
    }

    let path = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    Ok(fs::canonicalize(&path).unwrap_or(path))
}

fn absolutize(path: &Path, cwd: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    fs::canonicalize(&absolute).unwrap_or(absolute)
}

fn git_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn git_command_string(cwd: &Path, args: &[&str]) -> String {
    format!("git -C {} {}", cwd.display(), args.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        git(dir.path(), &["init", "-b", "main"]);
        git(dir.path(), &["config", "user.email", "test@example.com"]);
        git(dir.path(), &["config", "user.name", "Test"]);
        dir
    }

    fn write_file(repo: &Path, rel: &str, content: &str) {
        let path = repo.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn parses_null_delimited_check_attr_output() {
        let output =
            b"src/lib.rs\0linguist-vendored\0set\0src/lib.rs\0linguist-generated\0unspecified\0";
        let parsed = parse_check_attr_output(output).unwrap();
        assert_eq!(
            parsed,
            vec![
                (
                    "src/lib.rs".to_string(),
                    "linguist-vendored".to_string(),
                    "set".to_string()
                ),
                (
                    "src/lib.rs".to_string(),
                    "linguist-generated".to_string(),
                    "unspecified".to_string()
                )
            ]
        );
    }

    #[test]
    fn git_attribute_filtering_uses_nested_gitattributes() {
        let repo = init_repo();
        write_file(
            repo.path(),
            "nested/.gitattributes",
            "vendor/** linguist-vendored\n",
        );
        write_file(repo.path(), "nested/vendor/lib.rs", "// REQ-1\n");
        write_file(repo.path(), "nested/src/lib.rs", "// REQ-2\n");
        git(
            repo.path(),
            &[
                "add",
                "nested/.gitattributes",
                "nested/vendor/lib.rs",
                "nested/src/lib.rs",
            ],
        );
        git(repo.path(), &["commit", "-m", "init"]);

        let args = FilterArgs::default();
        let files = crate::filter::collect_files(&repo.path().join("nested"), &args).unwrap();

        assert!(
            !files
                .iter()
                .any(|path| path.ends_with("nested/vendor/lib.rs"))
        );
        assert!(files.iter().any(|path| path.ends_with("nested/src/lib.rs")));
    }

    #[test]
    fn git_attribute_source_can_use_index() {
        let repo = init_repo();
        write_file(repo.path(), ".gitattributes", "");
        write_file(repo.path(), "src/lib.rs", "// REQ-1\n");
        git(repo.path(), &["add", ".gitattributes", "src/lib.rs"]);
        git(repo.path(), &["commit", "-m", "init"]);

        write_file(repo.path(), ".gitattributes", "src/** linguist-generated\n");

        let worktree_args = FilterArgs::default();
        let worktree_files = crate::filter::collect_files(repo.path(), &worktree_args).unwrap();
        assert!(
            !worktree_files
                .iter()
                .any(|path| path.ends_with("src/lib.rs"))
        );

        let index_args = FilterArgs {
            git_attr_source: Some(GitAttrSource::Index),
            ..Default::default()
        };
        let index_files = crate::filter::collect_files(repo.path(), &index_args).unwrap();
        assert!(index_files.iter().any(|path| path.ends_with("src/lib.rs")));
    }
}
