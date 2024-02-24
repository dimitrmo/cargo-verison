mod error;
mod project;

use project::Project;
use crate::error::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command()]
struct Args {
    #[command(subcommand)]
    cmd: Commands
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Current,
    Patch {
        /// If supplied with -m or --message config option, cargo will use it as a commit message when creating a version commit.
        /// If the message config contains %s then that will be replaced with the resulting version number. For example:
        ///
        /// cargo verison patch -m "Upgrade to %s for reasons"
        ///
        #[clap(short, long)]
        message: Option<String>,

        /// Tag the commit when using the cargo verison command. Setting this to false results in no commit being made at all.
        #[clap(long = "git-tag-version")]
        add_git_tag: Option<bool>,
    }
}

pub fn main() -> Result<()> {
    let args = Args::parse();
    let mut project = Project::new()?;
    match args.cmd {
        Commands::Current => {
            println!("{}", project.get_current_version())
        }
        Commands::Patch{
            message,
            add_git_tag
        } => {
            let patch = project.next_patch();
            println!(">> new patch found {}", patch);

            println!(">> writing manifest");
            project.write()?;

            println!(">> updating packages");
            project.cargo_update()?;

            let add_git_tag = add_git_tag.unwrap_or(true);
            println!(">> add git tag: {}", add_git_tag);

            if add_git_tag {
                if let Some(commit) = message.clone() {
                    println!(">> committing with message: {commit}");
                }

                project.commit(message)?;
            }

            println!("{}", patch);
        }
    };
    Ok(())
}
