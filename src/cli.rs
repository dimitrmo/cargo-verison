mod error;
mod project;

use crate::error::Result;
use clap::{Parser, Subcommand};
use project::Project;

#[derive(Parser, Debug)]
#[clap(version)]
#[command()]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Current {
        /// Update the workspace version
        #[clap(long = "workspace")]
        workspace: Option<bool>,

        /// Project directory. Defaults to current_dir.
        #[clap(long = "directory")]
        directory: Option<String>,
    },
    Patch {
        /// If supplied with -m or --message config option, cargo will use it as a commit message when creating a version commit.
        /// If the message config contains %s then that will be replaced with the resulting version number. For example:
        ///
        /// Cargo verison patch -m "Upgrade to %s for reasons"
        ///
        #[clap(short, long)]
        message: Option<String>,

        /// Tag the commit when using the cargo verison command. Setting this to false results in no commit being made at all.
        #[clap(long = "git-tag-version")]
        add_git_tag: Option<bool>,

        /// Update the workspace version
        #[clap(long = "workspace")]
        workspace: Option<bool>,

        /// Project directory. Defaults to current_dir.
        #[clap(long = "directory")]
        directory: Option<String>,
    },
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::Current {
            workspace,
            directory,
        } => {
            let workspace = workspace.unwrap_or(false);
            let project = Project::create(workspace, directory)?;

            println!("{}", project.get_current_version())
        }
        Commands::Patch {
            message,
            add_git_tag,
            workspace,
            directory,
        } => {
            let workspace = workspace.unwrap_or(false);
            let mut project = Project::create(workspace, directory)?;
            let new_version = project.next_patch()?;
            project.write()?;
            project.cargo_update()?;
            let add_git_tag = add_git_tag.unwrap_or(true);

            if add_git_tag {
                project.commit(message)?;
            }

            println!("{}", new_version);
        }
    };
    Ok(())
}
