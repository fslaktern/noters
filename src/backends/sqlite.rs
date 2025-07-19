use super::*;
use rusqlite::{params, Connection, Error as SqliteError, ErrorCode, OptionalExtension};

#[derive(Debug)]
pub struct SqliteBackend {
    connection: Connection,
}

impl SqliteBackend {
    pub fn new(path: String) -> Self {
        let connection = Connection::open(&path)
            .unwrap_or_else(|e| panic!("Failed opening DB at '{}': {}", path, e));

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

// Maps rusqlite errors into BackendError
// Can't Impl From when BackendError belongs to another file, so this'll do
fn map_sqlite_error(e: rusqlite::Error) -> NoteError {
    match e {
        SqliteError::SqliteFailure(code, _) => match code.code {
            ErrorCode::DatabaseBusy => NoteError::Backend(BackendError::Timeout),
            ErrorCode::PermissionDenied => NoteError::Backend(BackendError::PermissionDenied),
            ErrorCode::NotADatabase => {
                NoteError::Backend(BackendError::Other(anyhow::anyhow!("Not a database")))
            }
            ErrorCode::SchemaChanged => {
                NoteError::Backend(BackendError::Other(anyhow::anyhow!("Schema changed")))
            }
            _ => NoteError::Backend(BackendError::Other(anyhow::anyhow!(
                "SQLite error: {:?}",
                code
            ))),
        },
        SqliteError::QueryReturnedNoRows => {
            NoteError::Backend(BackendError::Other(anyhow::anyhow!("No rows found")))
        }
        other => NoteError::Backend(BackendError::Other(anyhow::Error::new(other))),
    }
}

impl NoteBackend for SqliteBackend {
    fn create(&self, note: Note) -> Result<u16> {
        self.connection
            .execute(
                "INSERT INTO notes (id, name, owner, content) VALUES (?1, ?2, ?3, ?4)",
                params![note.id, note.name, note.owner, note.content],
            )
            .map_err(map_sqlite_error)?;
        Ok(note.id)
    }

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
            .ok_or(NoteError::Backend(BackendError::NoRows(id)))
    }

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
            .ok_or(NoteError::Backend(BackendError::NoRows(id)))
    }

    fn update(&self, note: Note) -> Result<()> {
        let rows = self
            .connection
            .execute(
                "UPDATE notes SET name = ?1, owner = ?2, content = ?3 WHERE id = ?4",
                params![note.name, note.owner, note.content, note.id],
            )
            .map_err(map_sqlite_error)?;

        if rows == 0 {
            Err(NoteError::Backend(BackendError::NoRows(note.id)))
        } else {
            Ok(())
        }
    }

    fn delete(&self, id: u16) -> Result<()> {
        let rows = self
            .connection
            .execute("DELETE FROM notes WHERE id = ?1", [id])
            .map_err(map_sqlite_error)?;

        if rows == 0 {
            Err(NoteError::Backend(BackendError::NoRows(id)))
        } else {
            Ok(())
        }
    }

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
