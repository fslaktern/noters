#![deny(clippy::cargo)]
#![deny(clippy::complexity)]
#![deny(clippy::correctness)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![deny(clippy::suspicious)]
// l#![warn(clippy::restriction)]

use std::io;
use tabled::Tabled;
use thiserror::Error;

pub mod app;
pub mod backends;
pub mod setup;
pub mod ui;

// More convenient Result type
pub type Result<T> = std::result::Result<T, NoteError>;

#[derive(Tabled, Debug)]
pub struct Note {
    pub id: u16,
    pub owner: String,
    pub name: String,
    pub content: String,
}

// Partial note data. Displayed in lists and for shallow reads
#[derive(Tabled)]
pub struct PartialNote {
    pub id: u16,
    pub owner: String,
    pub name: String,
}

// Trait to be implemented by all backends
pub trait NoteBackend {
    fn create(&self, note: Note) -> Result<u16>;
    fn read(&self, id: u16) -> Result<Note>;
    fn read_partial(&self, id: u16) -> Result<PartialNote>;
    fn update(&self, note: Note) -> Result<()>;
    fn delete(&self, id: u16) -> Result<()>;
    fn list(&self) -> Result<Vec<PartialNote>>;
}

// Enum for all possible validation or repository-related errors
#[derive(Debug, Error)]
pub enum NoteError {
    #[error(transparent)]
    Validation(#[from] NoteValidationError),

    #[error(transparent)]
    Backend(#[from] BackendError),

    #[error(transparent)]
    Menu(#[from] MenuError),
}

// Enum for all possible menu input errors
#[derive(Debug, Error)]
pub enum MenuError {
    #[error("Failed to read from stdin: {0}")]
    StdinReadError(io::Error),

    #[error("Couldn't convert '{0}' to a number. Please enter a number 1-6")]
    ParseError(String),

    #[error("Couldn't convert '{0}' to a MenuOption. Please enter a number 1-6")]
    InvalidOption(u8),

    #[error("Failed writing to stdout")]
    StdoutWriteError(io::Error),
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
pub enum BackendError {
    #[error("Database file not found")]
    DatabaseNotFound,

    #[error("Notes table not found")]
    TableNotFound,

    #[error("Database is locked or busy")]
    DatabaseBusy,

    #[error("Database corruption or file I/O error")]
    DatabaseCorruptOrIo,

    #[error("SQL logic error or misuse of SQLite")]
    SqlLogicError,

    #[error("Backend connection timed out")]
    Timeout,

    #[error("Didn't find any notes with that ID")]
    NoRows,

    #[error("Database file is not a valid SQLite databasee")]
    NotADatabase,

    #[error("Database schema has changed unexpectedly")]
    SchemaChanged,

    #[error("No files with ID: {0}")]
    FileNotFound(u16),

    #[error("Can't find notes directory: {0}")]
    DirectoryNotFound(String),

    #[error("Insufficient permissions")]
    PermissionDenied,

    #[error(transparent)]
    Other(#[from] anyhow::Error), // Used as fallback
}
