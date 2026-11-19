//! File input and output.
//!
//! This module is available only with the `std` feature.

#![cfg(feature = "std")]

pub mod json;

use std::fs;
use std::path::{Path, PathBuf};

/// Read a file as UTF-8 text.
pub fn read_string(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))
}

/// Write UTF-8 text to a file, replacing its contents.
pub fn write_string(path: &str, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("cannot write {path}: {error}"))
}

/// Report whether a path exists.
pub fn exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// List the entries of a directory.
pub fn file_names(path: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let entries = fs::read_dir(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read {path}: {error}"))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(names)
}

/// Return the parent directory of a path.
pub fn directory_name(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

/// Join path components.
pub fn file_name_join(parts: &[String]) -> String {
    let mut path = PathBuf::new();
    for part in parts {
        path.push(part);
    }
    path.to_string_lossy().into_owned()
}

/// Create a directory and its missing parents.
pub fn create_directory(path: &str) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("cannot create {path}: {error}"))
}

/// Delete a file.
pub fn delete_file(path: &str) -> Result<(), String> {
    fs::remove_file(path).map_err(|error| format!("cannot delete {path}: {error}"))
}

/// Parse delimited text into rows of fields.
pub fn parse_delimited(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    while let Some(character) = chars.next() {
        if quoted {
            if character == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(character);
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            character if character == delimiter => {
                row.push(core::mem::take(&mut field));
            }
            '\n' => {
                row.push(core::mem::take(&mut field));
                rows.push(core::mem::take(&mut row));
            }
            '\r' => {}
            character => field.push(character),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Render rows as delimited text.
pub fn write_delimited(rows: &[Vec<String>], delimiter: char) -> String {
    let mut out = String::new();
    for row in rows {
        for (index, field) in row.iter().enumerate() {
            if index > 0 {
                out.push(delimiter);
            }
            let needs_quotes = field.contains(delimiter)
                || field.contains('"')
                || field.contains('\n')
                || field.contains('\r');
            if needs_quotes {
                out.push('"');
                out.push_str(&field.replace('"', "\"\""));
                out.push('"');
            } else {
                out.push_str(field);
            }
        }
        out.push('\n');
    }
    out
}
