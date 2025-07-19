# Hexagonal architecture

## Flow

1. Inbound adapter (src/ui/cli.rs):
   - User chooses from CLI menu
   - Calls app logic, e.g. `service.create_note(...)`

2. Application service (struct NoteService in app.rs):
   - Validates input
   - Applies rules
   - Delegates to `self.repo.create(...)`

3. Port / Interface (trait NoteRepository):
   - Defined in lib.rs
   - Core abstraction for storing notes

4. Outbound adapter (FilesystemBackend, SqliteBackend):
   - Backend-specific logic
   - Implements and conforms with the NoteRepository trait

## Structure

```text
src/
├── app.rs              ← Application service (NoteService)
├── lib.rs              ← Domain: Note, PartialNote, trait NoteRepository
├── main.rs             ← Entry point (CLI startup)
├── backends.rs
├── backends/           ← Outbound interfaces
│   ├── filesystem.rs   ← FilesystemBackend (implements NoteRepository)
│   └── sqlite.rs       ← SqliteBackend (implements NoteRepository)
├── ui.rs
├── ui/                 ← Inbound interfaces
│   ├── input.rs        ← Input handling and requirement for inbound adapters
│   └── cli.rs          ← CLI user interface (input handling)
├── setup.rs
└── setup/              ← Runtime setup & configuration
    ├── arguments.rs    ← CLI args (Backend::Sqlite etc.)
    └── logging.rs      ← Setup for tracing/logging
```
