use super::{BackendError, Note, NoteBackend, NoteError, PartialNote, Result};
use rusqlite::{params, Connection, Error as SqliteError, ErrorCode, OptionalExtension};

#[derive(Debug)]
pub struct SqliteBackend {
    connection: Connection,
}

impl SqliteBackend {
    /// Creates a new `SqliteBackend` by opening the `SQLite` database at the given path.
    /// Also ensures that the `notes` table exists.
    ///
    /// # Panics
    ///
    /// Panics if the database file cannot be opened or the `notes` table cannot be created.
    /// This is intended to fail fast during startup.
    #[must_use]
    pub fn new(path: &str) -> Self {
        let connection =
            Connection::open(path).unwrap_or_else(|e| panic!("Failed opening DB at '{path}': {e}"));

        // Create notes table if it doesn't exist
        connection
            .execute(
                "
                CREATE TABLE IF NOT EXISTS notes (
                    id      INTEGER PRIMARY KEY,
                    name    TEXT NOT NULL,
                    owner   TEXT NOT NULL,
                    content TEXT NOT NULL
                )
                ",
                [],
            )
            .expect("Failed to create notes table");
        Self { connection }
    }
}

/// Maps a `rusqlite::Error` into a `NoteError`, wrapping known SQLite-specific codes into domain-specific variants.
///
/// This function is used internally by all database operations.
///
/// # Errors
///
/// Always returns a `NoteError::Backend` variant. Specific known `SQLite` error codes
/// are converted to more descriptive errors; all others are wrapped in `BackendError::Other`.
fn map_sqlite_error(e: rusqlite::Error) -> NoteError {
    match e {
        SqliteError::SqliteFailure(code, _) => match code.code {
            ErrorCode::DatabaseBusy => NoteError::Backend(BackendError::Timeout),
            ErrorCode::PermissionDenied => NoteError::Backend(BackendError::PermissionDenied),
            ErrorCode::NotADatabase => NoteError::Backend(BackendError::NotADatabase),
            ErrorCode::SchemaChanged => NoteError::Backend(BackendError::SchemaChanged),
            _ => NoteError::Backend(BackendError::Other(anyhow::anyhow!(
                "SQLite error: {:?}",
                code
            ))),
        },
        SqliteError::QueryReturnedNoRows => NoteError::Backend(BackendError::NoRows),
        other => NoteError::Backend(BackendError::Other(anyhow::Error::new(other))),
    }
}

impl NoteBackend for SqliteBackend {
    /// Inserts a new note into the `SQLite` database.
    ///
    /// # Errors
    ///
    /// Returns:
    /// - `BackendError::Timeout`, `PermissionDenied`, `NotADatabase`, or other mapped SQLite-specific errors.
    /// - `BackendError::Other` if an unknown `SQLite` error occurs.
    fn create(&self, note: Note) -> Result<u16> {
        self.connection
            .execute(
                "INSERT INTO notes (id, name, owner, content) VALUES (?1, ?2, ?3, ?4)",
                params![note.id, note.name, note.owner, note.content],
            )
            .map_err(map_sqlite_error)?;
        Ok(note.id)
    }

    /// Reads a note by ID, returning only its ID, name, and owner (no content).
    ///
    /// # Errors
    ///
    /// Returns:
    /// - `BackendError::NoRows` if no note with the given ID exists.
    /// - Other mapped `SQLite` errors for query failure.
    fn read(&self, id: u16) -> Result<Note> {
        self.connection
            .query_row(
                "SELECT id, name, owner, content FROM notes WHERE id = ?1",
                [id],
                |row| {
                    Ok(Note {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        owner: row.get(2)?,
                        content: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(map_sqlite_error)?
            .ok_or(NoteError::Backend(BackendError::NoRows))
    }

    /// Reads a note by ID, returning only its ID, name, and owner (no content).
    ///
    /// # Errors
    ///
    /// Returns:
    /// - `BackendError::NoRows` if no note with the given ID exists.
    /// - Other mapped `SQLite` errors for query failure.
    fn read_partial(&self, id: u16) -> Result<PartialNote> {
        self.connection
            .query_row(
                "SELECT id, name, owner FROM notes WHERE id = ?1",
                [id],
                |row| {
                    Ok(PartialNote {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        owner: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(map_sqlite_error)?
            .ok_or(NoteError::Backend(BackendError::NoRows))
    }

    /// Updates an existing note's name, owner, and content.
    ///
    /// # Errors
    ///
    /// Returns:
    /// - `BackendError::NoRows` if no note with the given ID exists.
    /// - Other backend errors if the update fails due to `SQLite` issues.
    fn update(&self, note: Note) -> Result<()> {
        let rows = self
            .connection
            .execute(
                "UPDATE notes SET name = ?1, owner = ?2, content = ?3 WHERE id = ?4",
                params![note.name, note.owner, note.content, note.id],
            )
            .map_err(map_sqlite_error)?;

        if rows == 0 {
            Err(NoteError::Backend(BackendError::NoRows))
        } else {
            Ok(())
        }
    }
    /// Deletes a note by ID from the database.
    ///
    /// # Errors
    ///
    /// Returns:
    /// - `BackendError::NoRows` if the note was not found.
    /// - Other backend errors if the deletion operation fails.
    fn delete(&self, id: u16) -> Result<()> {
        let rows = self
            .connection
            .execute("DELETE FROM notes WHERE id = ?1", [id])
            .map_err(map_sqlite_error)?;

        if rows == 0 {
            Err(NoteError::Backend(BackendError::NoRows))
        } else {
            Ok(())
        }
    }

    /// Returns a list of all notes in the database, sorted by ID. The notes include only metadata: ID, name, and owner.
    ///
    /// # Errors
    ///
    /// Returns:
    /// - A backend error if the query fails or the data cannot be retrieved.
    fn list(&self) -> Result<Vec<PartialNote>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, name, owner FROM notes ORDER BY id ASC")
            .map_err(map_sqlite_error)?;

        let notes_iter = stmt
            .query_map([], |row| {
                Ok(PartialNote {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner: row.get(2)?,
                })
            })
            .map_err(map_sqlite_error)?;

        notes_iter
            .collect::<std::result::Result<_, _>>()
            .map_err(map_sqlite_error)
    }
}
