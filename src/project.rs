use git2::{Commit, Repository};
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
    repository: Result<Repository, git2::Error>,
    directory: Option<String>,
}

impl Project {
    pub fn create(workspace: bool, directory: Option<String>) -> anyhow::Result<Self> {
        let mut path = match directory.as_ref() {
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
            directory,
        })
    }

    pub fn set_version(&mut self, version: &str) -> anyhow::Result<()> {
        self.semver = semver::Version::parse(version)?;
        Ok(())
    }

    pub fn next_patch(&mut self) -> anyhow::Result<String> {
        let mut next = self.semver.clone();
        next.patch += 1;
        self.set_version(next.to_string().as_str())?;
        return Ok(self.get_current_version());
    }

    pub fn get_current_version(&self) -> String {
        return self.semver.to_string();
    }

    pub fn cargo_update(&self) -> anyhow::Result<String> {
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

    pub fn write(&self) -> anyhow::Result<()> {
        let mut path = match self.directory.as_ref() {
            None => current_dir()?,
            Some(dir) => PathBuf::from(dir),
        };

        path.push("Cargo.toml");
        let file = read_to_string(&path)?;
        let mut document = file.parse::<DocumentMut>()?;
        self.update_version(&mut document);
        let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
        file.write_all(document.to_string().as_bytes())?;

        Ok(())
    }

    #[inline(always)]
    fn find_last_commit(repo: &Repository) -> anyhow::Result<Commit> {
        let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
        let commit = obj
            .into_commit()
            .map_err(|_| git2::Error::from_str("Couldn't find commit"))?;
        Ok(commit)
    }

    pub fn commit(&self, message: Option<String>) -> anyhow::Result<()> {
        let version = self.get_current_version();
        let commit = match message {
            Some(msg) => msg.replace("%s", &version),
            None => version.clone(),
        };

        let repo = match self.repository.as_ref() {
            Ok(repo) => repo,
            Err(git_error) => {
                return Err(git2::Error::new(
                    git_error.code(),
                    git_error.class(),
                    git_error.message(),
                )
                .into());
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
        assert!(!project.as_ref().unwrap().semver.to_string().is_empty());
    }

    #[test]
    fn it_can_read_version_from_a_path() {
        let project = Project::create(false, Some(String::from("tests/standalone")));
        assert!(project.is_ok());
        assert_eq!(project.unwrap().semver.to_string(), "1.2.3");
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
        assert_eq!(project.as_mut().unwrap().next_patch().unwrap(), "3.2.2");
        assert_eq!(project.as_ref().unwrap().get_current_version(), "3.2.2");
    }

    #[test]
    fn it_can_read_and_calculate_and_write_a_project() {
        let mut project = Project::create(false, Some(String::from("tests/standalone")));
        assert!(project.is_ok());
        assert_eq!(project.as_ref().unwrap().semver.to_string(), "1.2.3");
        assert_eq!(project.as_ref().unwrap().get_current_version(), "1.2.3");
        assert_eq!(project.as_mut().unwrap().next_patch().unwrap(), "1.2.4");
        assert_eq!(project.as_ref().unwrap().get_current_version(), "1.2.4");
        project.as_mut().unwrap().write().unwrap();
        let mut project2 = Project::create(false, Some(String::from("tests/standalone")));
        assert_eq!(project2.as_ref().unwrap().get_current_version(), "1.2.4");
        project2.as_mut().unwrap().set_version("1.2.3").unwrap();
        project2.as_mut().unwrap().write().unwrap();
        assert_eq!(project2.as_ref().unwrap().get_current_version(), "1.2.3");
    }

    #[test]
    fn it_can_read_and_calculate_and_write_a_workspace_project() {
        let mut project = Project::create(true, Some(String::from("tests/workspace")));
        assert!(project.is_ok());
        assert_eq!(project.as_ref().unwrap().semver.to_string(), "3.2.1");
        assert_eq!(project.as_ref().unwrap().get_current_version(), "3.2.1");
        assert_eq!(project.as_mut().unwrap().next_patch().unwrap(), "3.2.2");
        assert_eq!(project.as_ref().unwrap().get_current_version(), "3.2.2");
        project.as_mut().unwrap().write().unwrap();
        let mut project2 = Project::create(true, Some(String::from("tests/workspace")));
        assert_eq!(project2.as_ref().unwrap().get_current_version(), "3.2.2");
        project2.as_mut().unwrap().set_version("3.2.1").unwrap();
        project2.as_mut().unwrap().write().unwrap();
        assert_eq!(project2.as_ref().unwrap().get_current_version(), "3.2.1");
    }
}
