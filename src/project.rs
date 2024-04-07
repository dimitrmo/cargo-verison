use crate::error::{Result, VerisonError};
use git2::Repository;
use serde::Deserialize;
use std::env::current_dir;
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

#[derive(Deserialize, Clone, Debug)]
struct Package {
    pub version: String,
}

#[derive(Deserialize, Clone, Debug)]
struct Config {
    pub package: Package,
}

#[derive(Deserialize, Clone, Debug)]
struct Workspace {
    pub package: Package,
}

#[derive(Deserialize, Clone, Debug)]
struct WorkspaceConfig {
    pub workspace: Workspace,
}

pub struct Project {
    workspace: bool,
    semver: semver::Version,
    repository: std::result::Result<Repository, git2::Error>,
}

impl Project {
    pub fn create(workspace: bool, directory: Option<String>) -> Result<Self> {
        let mut path = match directory {
            None => current_dir()?,
            Some(dir) => PathBuf::from(dir),
        };

        let repository = Repository::open(&path);
        path.push("Cargo.toml");
        let contents = read_to_string(&path)?;

        let semver = match workspace {
            true => {
                let config: WorkspaceConfig = toml::from_str(&contents)?;
                semver::Version::parse(&config.workspace.package.version)
            }
            false => {
                let config: Config = toml::from_str(&contents)?;
                semver::Version::parse(&config.package.version)
            }
        }?;

        Ok(Project {
            workspace,
            semver,
            repository,
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

    pub fn update_version(&self, document: &mut DocumentMut) {
        match self.workspace {
            true => {
                document["workspace"]["package"]["version"] =
                    toml_edit::value(self.get_current_version())
            }
            false => document["package"]["version"] = toml_edit::value(self.get_current_version()),
        }
    }

    pub fn write(&self) -> Result<()> {
        let mut path = current_dir()?;
        path.push("Cargo.toml");
        let file = read_to_string(&path)?;
        let mut document = file.parse::<DocumentMut>()?;
        self.update_version(&mut document);
        let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
        file.write_all(document.to_string().as_bytes())?;
        Ok(())
    }

    #[inline(always)]
    fn find_last_commit(repo: &Repository) -> std::result::Result<git2::Commit, git2::Error> {
        let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
        obj.into_commit()
            .map_err(|_| git2::Error::from_str("Couldn't find commit"))
    }

    pub fn commit(&self, message: Option<String>) -> Result<()> {
        let version = self.get_current_version();
        let commit = match message {
            Some(msg) => msg.replace("%s", &version),
            None => version.clone(),
        };

        let repo = match self.repository.as_ref() {
            Ok(repo) => repo,
            Err(git_error) => {
                return Err(VerisonError::Git(git2::Error::new(
                    git_error.code(),
                    git_error.class(),
                    git_error.message(),
                )));
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
            Some("HEAD"),      // point HEAD to our new commit
            &signature,        // author
            &signature,        // committer
            &commit,           // message
            &tree,             // tree
            &[&parent_commit], // parents
        )?;

        let object = repo.find_object(new_oid, Some(git2::ObjectType::Commit))?;
        repo.tag_lightweight(version.clone().as_str(), &object, false)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::project::Project;

    #[test]
    fn it_can_create_a_project() {
        let project = Project::create(false, None).unwrap();
        assert_eq!(project.workspace, false);
        assert!(project.repository.is_ok());
        assert!(!project.semver.to_string().is_empty());
    }

    #[test]
    fn it_cannot_create_a_project() {
        let project = Project::create(false, Some(String::from("/tmp")));
        assert!(project.is_err());
    }

    #[test]
    fn it_can_create_a_project_from_path() {
        let project = Project::create(false, Some(String::from("tests/standalone")));
        assert!(project.is_ok());
        assert_eq!(project.as_ref().unwrap().workspace, false);
        assert!(project.as_ref().unwrap().repository.is_ok());
        assert!(!project.as_ref().unwrap().semver.to_string().is_empty());
    }

    #[test]
    fn it_can_read_version_from_a_path() {
        let project = Project::create(false, Some(String::from("tests/standalone")));
        assert_eq!(project.unwrap().semver.to_string(), "1.2.3");
    }

    #[test]
    fn it_can_create_a_project_from_path_with_no_repo() {
        let project = Project::create(false, Some(String::from("tests/standalone-no-repo")));
        assert!(project.is_ok());
        assert_eq!(project.as_ref().unwrap().workspace, false);
        assert!(project.as_ref().unwrap().repository.is_err());
        assert!(!project.as_ref().unwrap().semver.to_string().is_empty());
    }

    #[test]
    fn it_can_fail_to_create_a_project_from_a_workspace() {
        let project = Project::create(false, Some(String::from("tests/workspace")));
        assert!(project.is_err());
    }

    #[test]
    fn it_can_fail_to_create_a_project_from_a_workspace_member() {
        let project = Project::create(
            false,
            Some(String::from("tests/workspace/workspace-member")),
        );
        assert!(project.is_err());
    }

    #[test]
    fn it_can_create_a_project_from_a_workspace() {
        let project = Project::create(true, Some(String::from("tests/workspace")));
        assert!(project.is_ok());
        assert_eq!(project.as_ref().unwrap().workspace, true);
        assert!(project.as_ref().unwrap().repository.is_ok());
        assert!(!project.as_ref().unwrap().semver.to_string().is_empty());
    }

    #[test]
    fn it_cannot_create_a_workspace_project_from_standalone() {
        let project = Project::create(true, Some(String::from("tests/standalone")));
        assert!(project.is_err());
    }

    #[test]
    fn it_can_read_version_a_project_from_a_workspace() {
        let project = Project::create(true, Some(String::from("tests/workspace")));
        assert!(project.is_ok());
        assert_eq!(project.as_ref().unwrap().semver.to_string(), "3.2.1");
    }

    #[test]
    fn it_can_read_and_calculate_version_a_project_from_a_workspace() {
        let mut project = Project::create(true, Some(String::from("tests/workspace")));
        assert!(project.is_ok());
        assert_eq!(project.as_ref().unwrap().semver.to_string(), "3.2.1");
        assert_eq!(project.as_ref().unwrap().get_current_version(), "3.2.1");
        assert_eq!(project.as_mut().unwrap().next_patch(), "3.2.2");
        assert_eq!(project.as_ref().unwrap().get_current_version(), "3.2.2");
    }
}
