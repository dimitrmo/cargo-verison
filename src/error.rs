use std::fmt;

pub type Result<T> = std::result::Result<T, VerisonError>;

#[derive(Debug)]
pub enum VerisonError {
    Other(String),
    IO(std::io::Error),
    Toml(toml::de::Error),
    TomlEdit(toml_edit::TomlError),
    SemVer(semver::Error),
    Git(git2::Error),
}

impl fmt::Display for VerisonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VerisonError::Other(err) => write!(f, "{}", err.to_string()),
            VerisonError::IO(err) => write!(f, "{}", err.to_string()),
            VerisonError::Toml(err) => write!(f, "{}", err.to_string()),
            VerisonError::TomlEdit(err) => write!(f, "{}", err.to_string()),
            VerisonError::SemVer(err) => write!(f, "{}", err.to_string()),
            VerisonError::Git(err) => write!(f, "{}", err.to_string()),
        }
    }
}

impl From<String> for VerisonError {
    fn from(err: String) -> Self {
        VerisonError::Other(err)
    }
}

impl From<&str> for VerisonError {
    fn from(err: &str) -> Self {
        VerisonError::Other(err.to_owned())
    }
}

impl From<std::io::Error> for VerisonError {
    fn from(err: std::io::Error) -> Self {
        VerisonError::IO(err)
    }
}

impl From<toml::de::Error> for VerisonError {
    fn from(err: toml::de::Error) -> Self {
        VerisonError::Toml(err)
    }
}

impl From<std::ffi::OsString> for VerisonError {
    fn from(_original_os_string: std::ffi::OsString) -> Self {
        VerisonError::Other("error converting OsString to String".to_owned())
    }
}

impl From<semver::Error> for VerisonError {
    fn from(err: semver::Error) -> Self {
        VerisonError::SemVer(err)
    }
}

impl From<toml_edit::TomlError> for VerisonError {
    fn from(err: toml_edit::TomlError) -> Self {
        VerisonError::TomlEdit(err)
    }
}

impl From<git2::Error> for VerisonError {
    fn from(err: git2::Error) -> Self {
        VerisonError::Git(err)
    }
}
