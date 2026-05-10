use clap::{Args, ValueEnum};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum GitAttrSource {
    #[default]
    Worktree,
    Index,
}

#[derive(Debug, Default, Args)]
pub struct FilterArgs {
    #[arg(long, help = "Include vendored files")]
    pub include_vendored: bool,

    #[arg(long, help = "Include generated files")]
    pub include_generated: bool,

    #[arg(long, help = "Include submodules")]
    pub include_submodules: bool,

    #[arg(
        long,
        value_enum,
        value_name = "SOURCE",
        help = "Git attribute source for vendored/generated detection (worktree or index)"
    )]
    pub git_attr_source: Option<GitAttrSource>,

    #[arg(
        long,
        value_name = "GLOB",
        help = "Only include paths matching this glob (repeatable)"
    )]
    pub include: Vec<String>,

    #[arg(
        long,
        value_name = "GLOB",
        help = "Exclude paths matching this glob (repeatable)"
    )]
    pub exclude: Vec<String>,
}

impl FilterArgs {
    pub fn git_attr_source(&self) -> GitAttrSource {
        self.git_attr_source.unwrap_or_default()
    }
}
