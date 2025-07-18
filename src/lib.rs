use std::io;
use thiserror::Error;

// More convenient Result type
pub type Result<T> = std::result::Result<T, NoteError>;

// Enum for all possible validation or repository-related errors
#[derive(Debug, Error)]
pub enum NoteError {
    #[error(transparent)]
    Validation(#[from] NoteValidationError),

    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error(transparent)]
    Menu(#[from] MenuError),

    #[error("Unexpected error: {0}")]
    Unexpected(#[from] anyhow::Error),
}

// Enum for all possible menu input errors
#[derive(Debug, Error)]
pub enum MenuError {
    #[error("Failed to read from stdin: {0}")]
    StdinReadError(io::Error),
    #[error("Invalid input. Please enter a number 1-6")]
    ParseError,
}

// Enum for all possible data and input validation errors
#[derive(Debug, Error)]
pub enum NoteValidationError {
    #[error("Name is empty")]
    NameEmpty,

    #[error("Content is empty")]
    ContentEmpty,

    #[error("Name is too large. Max: {max}, Got: {got}")]
    NameTooLarge { max: u8, got: usize },

    #[error("Content is too large. Max: {max}, Got: {got}")]
    ContentTooLarge { max: u16, got: usize },

    #[error("Note count rate limit exceeded. Max: {max}")]
    NoteCountRateLimit { max: u16 },

    #[error("Sorry! You're not the owner the note with ID: {0}")]
    PermissionDenied(u16),

    #[error("Note not found with ID: {0}")]
    NoteNotFound(u16),

    #[error("Note is referenced by: {0:?}")]
    NoteIsReferenced(Vec<u16>),

    #[error("Reference not found with ID: {0}")]
    ReferenceNotFound(u16),
}

// Enum for all possible repository/backend errors
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database file not found")]
    DatabaseNotFound,

    #[error("Table not found")]
    TableNotFound,

    #[error("Database is locked or busy")]
    DatabaseBusy,

    #[error("Database corruption or file I/O error")]
    DatabaseCorruptOrIo,

    #[error("SQL logic error or misuse of SQLite")]
    SqlLogicError,

    #[error("Connection timed out")]
    Timeout,

    #[error("No rows with ID: {0}")]
    NoRows(u16),

    #[error("No files with ID: {0}")]
    FileNotFound(u16),

    #[error("Can't find notes directory: {0}")]
    DirectoryNotFound(String),

    #[error("Insufficient permissions")]
    PermissionDenied,

    #[error(transparent)]
    Other(#[from] anyhow::Error), // Used as fallback
}
