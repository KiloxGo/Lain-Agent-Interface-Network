//! Explicit local presentation asset. Never fetched, sent to Kernel or used as persona.
use anyhow::{Context, Result, ensure};
use std::{io::Read, path::PathBuf};
use unicode_width::UnicodeWidthChar;

pub(super) const WIDTH: u16 = 29;
pub(super) const HEIGHT: u16 = 18;

pub(super) fn parse(text: &str) -> Result<String> {
    let text = text.replace("\r\n", "\n");
    let text = text.trim_end_matches('\n');
    let rows: Vec<_> = text.split('\n').collect();
    ensure!(
        rows.len() == usize::from(HEIGHT),
        "portrait must contain 18 rows"
    );
    ensure!(
        rows.iter()
            .all(|row| row.chars().count() == usize::from(WIDTH)
                && row.chars().all(|c| !c.is_control()
                    && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
                    && UnicodeWidthChar::width(c) == Some(1))),
        "portrait rows must contain 29 single-column printable characters"
    );
    Ok(text.to_owned())
}

pub(super) fn load(path: Option<PathBuf>) -> Result<Option<String>> {
    let Some(path) = path.or_else(|| std::env::var_os("LAIN_PORTRAIT_FILE").map(PathBuf::from))
    else {
        return Ok(None);
    };
    let file = std::fs::File::open(&path).context("open selected local portrait")?;
    ensure!(
        file.metadata()?.is_file(),
        "portrait must be a regular text file"
    );
    let mut bytes = Vec::new();
    file.take(8193).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 8192, "portrait exceeds 8 KiB");
    let text = std::str::from_utf8(&bytes).context("portrait must be UTF-8 text")?;
    parse(text).map(Some)
}
