use super::*;

use crate::app::NoteService;
use crate::setup::{arguments, logging};
use crate::ui::cli;
use std::fmt;

pub trait IO {
    fn get_input(&self) -> Result<String>;
    fn get_input_until(&self, stop_at: &str) -> Result<String>;

    fn show_menu(&self, options: &[impl std::fmt::Display]);
    fn show_title(&self, title: &str);
    fn show_table(&self, table: Vec<PartialNote>);

    fn show_text(&self, msg: &str);
    fn show_error(&self, msg: &str);
    fn show_warn(&self, msg: &str);
    fn show_info(&self, msg: &str);
    fn show_debug(&self, msg: &str);
    fn show_trace(&self, msg: &str);
}

// All UI menu options
#[derive(Debug, Clone, Copy)]
pub enum MenuOption {
    Create = 1,
    Read = 2,
    Update = 3,
    Delete = 4,
    List = 5,
    AddFlag = 6,
}

// Array of all menu options. Used for iteration
pub const ALL_MENU_OPTIONS: [MenuOption; 6] = [
    MenuOption::Create,
    MenuOption::Read,
    MenuOption::Update,
    MenuOption::Delete,
    MenuOption::List,
    MenuOption::AddFlag,
];

// Convert integer to MenuOption
impl TryFrom<u8> for MenuOption {
    type Error = ();

    fn try_from(n: u8) -> std::result::Result<Self, Self::Error> {
        match n {
            1 => Ok(MenuOption::Create),
            2 => Ok(MenuOption::Read),
            3 => Ok(MenuOption::Update),
            4 => Ok(MenuOption::Delete),
            5 => Ok(MenuOption::List),
            6 => Ok(MenuOption::AddFlag),
            _ => Err(()),
        }
    }
}

// Display a menu option with number and label
impl fmt::Display for MenuOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            MenuOption::Create => "Create note",
            MenuOption::Read => "Read note",
            MenuOption::Update => "Update note",
            MenuOption::Delete => "Delete note",
            MenuOption::List => "List notes",
            MenuOption::AddFlag => "Add note with flag",
        };
        write!(f, "({}) {}", *self as u8, label)
    }
}

// Route selected menu option to its handler function
fn handle_menu_option(io: &impl IO, service: &NoteService, option: MenuOption) {
    match option {
        MenuOption::Create => handle_create(io, service),
        MenuOption::Read => handle_read(io, service),
        MenuOption::Update => handle_update(io, service),
        MenuOption::Delete => handle_delete(io, service),
        MenuOption::List => handle_list(io, service),
        MenuOption::AddFlag => handle_add_flag(io, service),
    }
}

pub fn run() {
    logging::setup_log();
    let service = arguments::handle_args();
    let io = cli::Cli;

    loop {
        io.show_menu(&ALL_MENU_OPTIONS);
        match get_menu_input(&io) {
            Ok(opt) => handle_menu_option(&io, &service, opt),
            Err(NoteError::Menu(e)) => io.show_error(&format!("{}", e)),
            Err(_) => unreachable!(),
        }
    }
}

// Prompt for menu option input and return parsed option
fn get_menu_input(io: &impl IO) -> Result<MenuOption> {
    loop {
        io.show_title("Choose option:");
        let input = io.get_input()?;

        match input.parse::<u8>() {
            Ok(n) => match MenuOption::try_from(n) {
                Ok(option) => return Ok(option),
                Err(_) => {
                    io.show_error(&format!("{}", MenuError::InvalidOption(n)));
                    continue;
                }
            },
            Err(_) => {
                io.show_error(&format!("{}", MenuError::ParseError(input)));
                continue;
            }
        }
    }
}

// fn handle_menu_error(e: crate::MenuError) {
//     use crate::MenuError::*;
//     match e {
//         ParseError => show_error("Failed to parse your input"),
//         StdinReadError(err) => show_error("Error reading menu choice: {}", err),
//     }
// }

// Handle "create note" menu option
fn handle_create(io: &impl IO, service: &NoteService) {
    io.show_title("Create note");

    let name: String = loop {
        io.show_text("Name:");
        let input = io.get_input().expect("Failed getting note name");
        match NoteService::validate_name(&input, service.max_name_size) {
            Ok(()) => {
                io.show_trace(&format!("Got valid name: {}", &input));
                break input;
            }
            Err(e) => io.show_error(&format!("Got invalid name: {}", e)),
        }
    };

    let content: String = loop {
        // Stop when getting a "." alone on a line
        io.show_text("Content (end with '.' on last line):");
        let input = io
            .get_input_until(".")
            .expect("Failed getting note content");
        match NoteService::validate_content(&input, service.max_content_size) {
            Ok(()) => {
                io.show_trace(&format!("Got valid content: {}", &input));
                break input;
            }
            Err(e) => io.show_error(&format!("Got invalid content: {}", e)),
        }
    };

    match service.create_note(name, content) {
        Ok(id) => io.show_info(&format!("Note saved with ID: {}", id)),
        Err(NoteError::Backend(e)) => io.show_error(&format!("Backend error: {}", e)),
        Err(e) => io.show_error(&format!("{}", e)),
    };
}

// Handle "read note" menu option
fn handle_read(io: &impl IO, service: &NoteService) {
    io.show_title("Read note");

    let id: u16 = loop {
        io.show_text("ID:");
        let input = io.get_input().expect("Failed getting note ID");
        match input.parse::<u16>() {
            Ok(id) => {
                io.show_trace(&format!("Got valid ID: {}", id));
                break id;
            }
            Err(e) => io.show_error(&format!("Got invalid ID: {}", e)),
        }
    };

    match service.read_note(id) {
        Ok(note) => {
            io.show_text("{:->20}");
            io.show_text(&format!("#{}: {}\n", note.id, note.name));
            io.show_text(&note.content);
            io.show_text("{:->20}");
        }
        Err(NoteError::Backend(e)) => io.show_error(&format!("Backend error: {}", e)),
        Err(e) => io.show_error(&format!("{}", e)),
    }
}

// Handle "update note" menu option
fn handle_update(io: &impl IO, service: &NoteService) {
    io.show_title("Update note");

    let mut note = loop {
        io.show_text("ID:");
        let input = io.get_input().expect("Failed getting note ID");
        let id = match input.parse::<u16>() {
            Ok(id) => id,
            Err(e) => {
                io.show_error(&format!("{}", e));
                continue;
            }
        };
        match service.read_note(id) {
            Ok(note) => break note,
            Err(e) => io.show_error(&format!("{}", e)),
        };
    };

    let name: String = loop {
        io.show_text("Name:");
        let input = io.get_input().expect("Failed getting note name");
        match NoteService::validate_name(&input, service.max_name_size) {
            Ok(()) => {
                io.show_trace(&format!("Got valid name: {}", &input));
                break input;
            }
            Err(e) => io.show_error(&format!("Got invalid name: {}", e)),
        }
    };

    let content: String = loop {
        // Stop when getting a "." alone on a line
        io.show_text("Content (end with '.' on last line):");
        let input = io
            .get_input_until(".")
            .expect("Failed getting note content");
        match NoteService::validate_content(&input, service.max_content_size) {
            Ok(()) => {
                io.show_trace(&format!("Got valid content: {}", &input));
                break input;
            }
            Err(e) => io.show_error(&format!("Got invalid content: {}", e)),
        }
    };

    note.name = name;
    note.content = content;

    match service.update_note(note) {
        Ok(()) => io.show_info("Successfully updated note"),
        Err(NoteError::Backend(e)) => io.show_error(&format!("Backend error {}", e)),
        Err(e) => io.show_error(&format!("{}", e)),
    };
}

// Handle "delete note" menu option with confirmation prompt
fn handle_delete(io: &impl IO, service: &NoteService) {
    io.show_title("Delete note");

    let id: u16 = loop {
        io.show_text("ID:");
        let input = io.get_input().expect("Failed getting note ID");
        match input.parse::<u16>() {
            Ok(id) => break id,
            Err(e) => io.show_error(&format!("{}", e)),
        }
    };

    loop {
        io.show_text("Are you absolutely sure? (y/n):");
        let input = io.get_input().expect("Failed getting delete confirmation");
        match input.to_lowercase().as_str() {
            "y" | "ye" | "yes" | "ya" | "yuh" | "yarr" | "fuck yeah" => break,
            "n" | "nu uh" | "no" | "nah" | "hell naw" | "get yo bitchass outta here" => {
                io.show_info(&format!("Exiting. Not deleting note with ID: {}", id));
                return;
            }
            _ => continue,
        }
    }

    match service.delete_note(id) {
        Ok(()) => io.show_info(&format!("Successfully deleted note with ID: {}", id)),
        Err(NoteError::Backend(e)) => io.show_error(&format!("Backend error {}", e)),
        Err(e) => io.show_error(&format!("{}", e)),
    };
}

// Handle "list notes" menu option
fn handle_list(io: &impl IO, service: &NoteService) {
    let partial_notes: Vec<PartialNote> = match service.list_notes() {
        Ok(n) => n,
        Err(e) => {
            io.show_error(&format!("{}", e));
            return;
        }
    };
    io.show_table(partial_notes);
}

// Handle "add note with flag" menu option
fn handle_add_flag(io: &impl IO, service: &NoteService) {
    match service.create_flag_note() {
        Ok(id) => io.show_info(&format!(
            "Successfully added note containing flag, with ID: {}",
            id
        )),
        Err(e) => io.show_error(&format!("Failed adding note containing flag: {}", e)),
    }
}
