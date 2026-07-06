use std::fmt;
use std::io::{self, Write};

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

pub fn info(out: &mut dyn Write, message: fmt::Arguments<'_>) -> io::Result<()> {
    writeln!(out, "[INFO] {message}")
}

pub fn warn(out: &mut dyn Write, message: fmt::Arguments<'_>) -> io::Result<()> {
    writeln!(out, "{YELLOW}[WARN] {message}{RESET}")
}

pub fn error(out: &mut dyn Write, message: fmt::Arguments<'_>) -> io::Result<()> {
    writeln!(out, "{RED}[ERROR] {message}{RESET}")
}

pub fn print_info(message: fmt::Arguments<'_>) {
    let mut stdout = io::stdout();
    let _ = info(&mut stdout, message);
}

pub fn print_warn(message: fmt::Arguments<'_>) {
    let mut stdout = io::stdout();
    let _ = warn(&mut stdout, message);
}

pub fn eprint_error(message: fmt::Arguments<'_>) {
    let mut stderr = io::stderr();
    let _ = error(&mut stderr, message);
}
