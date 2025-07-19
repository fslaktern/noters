use super::*;

use crate::ui::input::IO;
use colored::Colorize;
use log::{debug, error, info, trace, warn};
use std::io::{self, Write};
use tabled::{settings::Style, Table};

pub struct Cli;

impl IO for Cli {
    // Prompt for user input, read until  and trim result
    fn get_input(&self) -> Result<String> {
        <Cli as IO>::get_input_until(self, "\n")
    }

    // Prompt for user input, read until [stop_at] and trim result
    fn get_input_until(&self, stop_at: &str) -> Result<String> {
        let mut input = String::new();
        loop {
            print!("> ");
            io::stdout()
                .flush()
                .map_err(|e| NoteError::Menu(MenuError::StdoutWriteError(e)))?;
            self.show_trace("Flushed stdout");

            let mut line = String::new();
            io::stdin()
                .read_line(&mut line)
                .map_err(|e| NoteError::Menu(MenuError::StdinReadError(e)))?;
            self.show_trace(&format!("Got input: {}", line.trim_end()));

            if line.trim() == stop_at {
                input = input.trim().to_string();
                break;
            } else {
                input += &line;
            }
        }

        Ok(input)
    }

    fn show_menu(&self, options: &[impl std::fmt::Display]) {
        println!("Please choose an option:");
        for o in options {
            println!("{}", o);
        }
        println!()
    }

    fn show_table(&self, data: Vec<PartialNote>) {
        let mut table = Table::new(data);
        table.with(Style::psql());
        println!("{}", table);
    }

    fn show_title(&self, title: &str) {
        println!("{}\n", title.to_string().bold());
    }

    fn show_text(&self, msg: &str) {
        println!("{msg}");
    }

    fn show_error(&self, msg: &str) {
        error!("{msg}");
    }

    fn show_warn(&self, msg: &str) {
        warn!("{msg}");
    }

    fn show_info(&self, msg: &str) {
        info!("{msg}");
    }

    fn show_debug(&self, msg: &str) {
        debug!("{msg}");
    }

    fn show_trace(&self, msg: &str) {
        trace!("{msg}");
    }
}
