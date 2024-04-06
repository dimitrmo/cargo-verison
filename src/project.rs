use serde::Deserialize;
use crate::error::{Result, VerisonError};
use std::env::current_dir;
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use git2::Repository;
use std::path::Path;

#[derive(Deserialize, Clone, Debug)]
struct Package {
    // pub name: String,
    pub version: String,
}

#[derive(Deserialize, Clone, Debug)]
struct Config {
    pub package: Package,
}

pub struct Project {
    // config: Config,
    semver: semver::Version,
    repository: std::result::Result<Repository, git2::Error>,
}

impl Project {
    pub fn new() -> Result<Self> {
        let mut path = current_dir()?;
        let repo = Repository::open(&path);
        path.push("Cargo.toml");
        let file = read_to_string(&path)?;
        let config: Config = toml::from_str(&file)?;
        let version = semver::Version::parse(&config.package.version)?;
        Ok(Project{
            // config,
            semver: version,
            repository: repo,
        })
    }

    pub fn next_patch(&mut self) -> String {
        self.semver.patch += 1;
        return self.get_current_version();
    }

    pub fn get_current_version(&self) -> String {
        return self.semver.to_string();
    }

    pub fn cargo_update(&self) -> Result<String> {
        std::process::Command::new("cargo")
            .arg("generate-lockfile")
            .arg("--verbose")
            // .arg("--locked")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .map_err(|e| e.into())
    }

    pub fn write(&self) -> Result<()> {
        let mut path = current_dir()?;
        path.push("Cargo.toml");
        let file = read_to_string(&path)?;
        let mut document = file.parse::<toml_edit::DocumentMut>()?;
        document["package"]["version"] = toml_edit::value(self.get_current_version());
        let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
        file.write_all(document.to_string().as_bytes())?;
        Ok(())
    }

    #[inline(always)]
    fn find_last_commit(repo: &Repository) -> std::result::Result<git2::Commit, git2::Error> {
        let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
        obj.into_commit().map_err(|_| git2::Error::from_str("Couldn't find commit"))
    }

    pub fn commit(&self, message: Option<String>) -> Result<()> {
        let version = self.get_current_version();
        let commit = match message {
            Some(msg) => msg.replace("%s", &version),
            None => version.clone()
        };

        let repo = match self.repository.as_ref() {
            Ok(repo) => repo,
            Err(git_error) => {
                return Err(VerisonError::Git(git2::Error::new(git_error.code(), git_error.class(), git_error.message())));
            }
        };

        let mut index = repo.index()?;

        let cargo_manifest = "Cargo.toml";
        let cargo_lock = "Cargo.lock";
        index.add_path(Path::new(&cargo_manifest))?;
        index.add_path(Path::new(&cargo_lock))?;
        index.write()?;

        let oid = index.write_tree()?;
        let parent_commit = Self::find_last_commit(&repo)?;
        let tree = repo.find_tree(oid)?;
        let signature = repo.signature()?;

        let new_oid = repo.commit(
            Some("HEAD"),       // point HEAD to our new commit
            &signature,         // author
            &signature,         // committer
            &commit,            // message
            &tree,              // tree
            &[&parent_commit]   // parents
        )?;

        let object = repo.find_object(new_oid, Some(git2::ObjectType::Commit))?;
        repo.tag_lightweight(version.clone().as_str(), &object, false)?;

        Ok(())
    }
}
