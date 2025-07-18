pub mod filesystem;
pub mod sqlite;

pub use filesystem::FilesystemBackend;
pub use sqlite::SqliteBackend;

use crate::{Note, PartialNote};
use nnsctf_pwn_1::Result;

// Trait to be implemented by all backends
pub trait NoteRepository {
    fn create(&self, note: Note) -> Result<u16>;
    fn read(&self, id: u16) -> Result<Note>;
    fn read_partial(&self, id: u16) -> Result<PartialNote>;
    fn update(&self, note: Note) -> Result<()>;
    fn delete(&self, id: u16) -> Result<()>;
    fn list(&self) -> Result<Vec<PartialNote>>;
}
