use crate::backends::{FilesystemBackend, NoteRepository, SqliteBackend};
use crate::NoteService;

use clap::{Parser, Subcommand};
use log::error;

#[derive(Parser, Debug)]
#[command(version, about, long_about)]
struct Args {
    #[arg(short, long)]
    user: String,
    #[arg(long, default_value_t = 32)]
    max_name_size: u8,
    #[arg(long, default_value_t = 1024)]
    max_content_size: u16,
    #[arg(long, default_value_t = 100)]
    max_note_count: u16,
    #[command(subcommand)]
    backend: Backend,
}

#[derive(Subcommand, Debug)]
enum Backend {
    Filesystem {
        #[arg(short, long)]
        path: String,
    },
    Sqlite {
        #[arg(short, long)]
        path: String,
    },
}

pub fn handle_args() -> NoteService {
    let args = Args::parse();

    // Hard limit on 32 characters per username
    if args.user.len() > 32 {
        error!("The chosen username is too long. It should be less than or equal to 32 characters");
    }

    dbg!(&args);

    // Allow any struct that implements NoteRepository, and store on heap because size is unknown at compile time
    let repo: Box<dyn NoteRepository> = match args.backend {
        Backend::Filesystem { path } => Box::new(FilesystemBackend::new(path)),
        Backend::Sqlite { path } => Box::new(SqliteBackend::new(path)),
    };

    NoteService::new(
        repo,
        args.user,
        args.max_name_size,
        args.max_content_size,
        args.max_note_count,
    )
}
