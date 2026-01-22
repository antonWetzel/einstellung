use std::{fs, io::Write, ops::Deref};

use console::{Style, Term, style};
use similar::{ChangeTag, TextDiff};

const CONFIG_PATHS: &[&str] = &[".einstellung", "einstellung"];

#[derive(Debug, thiserror::Error)]
enum EinstellungError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("The configuration file is missing")]
    ConfigurationMissing,

    #[error("The synced file {0} is missing ({1})")]
    SyncFileMissing(String, std::io::Error),

    #[error("The synced file {0} could not be saved ({1})")]
    FailedToSaveSyncFile(String, std::io::Error),

    #[error("The configuration file {0} could not be saved ({1})")]
    FailedToSaveConfigurationFile(String, std::io::Error),
}

fn main() -> Result<(), EinstellungError> {
    match std::env::args()
        .into_iter()
        .nth(1)
        .map(|arg| arg.to_lowercase())
        .as_deref()
        .map(|arg| arg.trim())
    {
        Some("read") => read(),
        Some("write") => write(),
        _ => help(),
    }
}

fn help() -> Result<(), EinstellungError> {
    const HELP_TEXT: &str = include_str!("../readme.md");
    println!("{HELP_TEXT}");
    return Ok(());
}

fn read_configuration() -> Result<Vec<(String, Vec<String>)>, EinstellungError> {
    let configuration = CONFIG_PATHS
        .iter()
        .find_map(|name| fs::read_to_string(name).ok())
        .ok_or(EinstellungError::ConfigurationMissing)?;

    let syncs = configuration
        .lines()
        .flat_map(|line| {
            let mut parts = line.split_whitespace();
            let Some(original_file) = parts.next() else {
                return None;
            };
            if original_file.starts_with('#') {
                return None;
            }

            Some((
                original_file.to_owned(),
                parts.map(|part| part.to_owned()).collect(),
            ))
        })
        .collect();

    Ok(syncs)
}

fn read() -> Result<(), EinstellungError> {
    let mut term = Term::stdout();
    term.show_cursor()?;

    let configuration = read_configuration()?;

    for (original_file, other_files) in configuration {
        writeln!(term, "Read for {original_file}")?;

        let original_content = fs::read_to_string(&original_file)
            .map_err(|err| EinstellungError::SyncFileMissing(original_file.to_owned(), err))?;
        for other_file in other_files {
            let Ok(other_file) = shellexpand::full(&other_file) else {
                writeln!(term, "  Invalid file name {}", other_file)?;
                continue;
            };

            let Ok(other_content) = fs::read_to_string(other_file.deref()) else {
                writeln!(term, "  Not found {}", other_file)?;
                continue;
            };
            writeln!(term, "  Compare with {}", other_file)?;
            let content = compare_files(&mut term, &original_content, &other_content)?;
            if let Some(content) = content {
                fs::write(&original_file, content).map_err(|err| {
                    EinstellungError::FailedToSaveSyncFile(original_file.to_owned(), err)
                })?;
            }
        }
    }

    write!(term, "\r")?;

    Ok(())
}

fn compare_files(
    term: &mut Term,
    original_content: &str,
    other_content: &str,
) -> Result<Option<String>, EinstellungError> {
    let diff = TextDiff::from_lines(original_content, &other_content);
    if diff
        .iter_all_changes()
        .all(|change| change.tag() == ChangeTag::Equal)
    {
        return Ok(None);
    }

    let hint = style("> A: keep | S: remove | D: keep block | F: remove block").bold();
    writeln!(term, "{hint}")?;

    // print changes and move the cursor to the top
    let mut resulting_content = String::new();
    let mut changed = false;
    let mut lines = 0;
    for change in diff.iter_all_changes() {
        lines += 1;
        let style = tag_style(change.tag());
        let prefix = tag_prefix(change.tag());
        let text = style.apply_to(change.value().trim_ascii_end());
        writeln!(term, " {prefix} {text}")?;
    }
    term.move_cursor_up(lines)?;

    // go through the lines and build the resulting content and
    // dim removed lines
    let mut automatic = None;
    for change in diff.iter_all_changes() {
        if let ChangeTag::Equal = change.tag() {
            resulting_content.push_str(change.value());
            term.move_cursor_down(1)?;
            continue;
        }

        let style = tag_style(change.tag());
        let prefix = tag_prefix(change.tag());
        term.clear_line()?;
        let text = style
            .clone()
            .bold()
            .apply_to(change.value().trim_ascii_end());
        write!(term, " {prefix} {text}\r",)?;

        let accept = if let Some((tag, accept)) = automatic
            && tag == change.tag()
        {
            accept
        } else {
            let (accept, auto_accept) = read_accept_input(term)?;
            automatic = auto_accept.map(|accept| (change.tag(), accept));
            accept
        };

        let style = if accept {
            resulting_content.push_str(change.value());
            style
        } else {
            style.dim()
        };
        let text = style.apply_to(change.value().trim_ascii_end());
        term.clear_line()?;
        write!(term, " {prefix} {text}",)?;

        changed |= accept == matches!(change.tag(), ChangeTag::Insert);

        term.move_cursor_down(1)?;
    }
    Ok((changed && read_save_question(term)?).then_some(resulting_content))
}

fn tag_style(change: ChangeTag) -> Style {
    match change {
        ChangeTag::Equal => Style::new(),
        ChangeTag::Insert => Style::new().green(),
        ChangeTag::Delete => Style::new().red(),
    }
}

fn tag_prefix(change: ChangeTag) -> char {
    match change {
        ChangeTag::Equal => ' ',
        ChangeTag::Insert => '+',
        ChangeTag::Delete => '-',
    }
}

fn read_accept_input(term: &mut Term) -> Result<(bool, Option<bool>), EinstellungError> {
    loop {
        let res = match term.read_char()?.to_ascii_lowercase() {
            'a' => (true, None),
            's' => (false, None),
            'd' => (true, Some(true)),
            'f' => (false, Some(false)),
            _ => continue,
        };
        return Ok(res);
    }
}

fn read_save_question(term: &mut Term) -> Result<bool, EinstellungError> {
    let hint = style("> S: save | D: discard").bold();
    writeln!(term, "{hint}")?;
    loop {
        let res = match term.read_char()?.to_ascii_lowercase() {
            's' => true,
            'd' => false,
            _ => continue,
        };
        return Ok(res);
    }
}

fn write() -> Result<(), EinstellungError> {
    let mut term = Term::stdout();
    term.show_cursor()?;

    let configuration = read_configuration()?;

    for (original_file, other_files) in configuration {
        writeln!(term, "Write for {original_file}")?;

        let original_content = fs::read_to_string(&original_file)
            .map_err(|err| EinstellungError::SyncFileMissing(original_file.to_owned(), err))?;
        for other_file in other_files {
            let Ok(other_file) = shellexpand::full(&other_file) else {
                writeln!(term, "  Invalid file name {}", other_file)?;
                continue;
            };

            let Ok(other_content) = fs::read_to_string(other_file.deref()) else {
                writeln!(term, "  Not found {}", other_file)?;
                continue;
            };
            writeln!(term, "  Compare to {}", other_file)?;
            if original_content == other_content {
                continue;
            }

            if read_save_question(&mut term)? {
                fs::write(other_file.deref(), &original_content).map_err(|err| {
                    EinstellungError::FailedToSaveConfigurationFile(original_file.to_owned(), err)
                })?;
            }
        }
    }

    write!(term, "\r")?;

    Ok(())
}
