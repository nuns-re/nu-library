use std::fmt;

pub type Result<T> = std::result::Result<T, NuError>;

#[derive(Debug)]
pub enum NuError {
    Unsupported,
    Truncated(&'static str),
    Invalid(&'static str),
    Limit(&'static str),
    Io(std::io::Error),
    Context(String),
}

impl fmt::Display for NuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("unsupported format"),
            Self::Truncated(section) => write!(f, "truncated {section}"),
            Self::Invalid(field) => write!(f, "invalid {field}"),
            Self::Limit(limit) => write!(f, "limit exceeded: {limit}"),
            Self::Io(error) => error.fmt(f),
            Self::Context(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for NuError {}

impl From<std::io::Error> for NuError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<anyhow::Error> for NuError {
    fn from(value: anyhow::Error) -> Self {
        Self::Context(format!("{value:#}"))
    }
}
