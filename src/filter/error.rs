use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FilterError {
    #[error("failed to walk directory: {0}")]
    Walk(#[from] ignore::Error),

    #[error("invalid glob pattern {pattern}: {source}")]
    InvalidGlob {
        pattern: String,
        source: glob::PatternError,
    },

    #[error("failed to run git: {0}")]
    GitRun(#[from] std::io::Error),

    #[error("git command failed ({cmd}): {stderr}")]
    GitCommandFailed { cmd: String, stderr: String },

    #[error("git output was not valid utf-8: {0}")]
    GitOutputUtf8(#[from] std::string::FromUtf8Error),

    #[error(
        "git-backed vendored/generated filtering requires the scan root to be inside a git repository: {root}"
    )]
    GitRepoRequired { root: PathBuf },
}
