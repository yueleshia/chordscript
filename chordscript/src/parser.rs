pub mod keyspaces;
pub mod shortcuts;
pub mod lexemes;
use crate::reporter::MarkupError;

//run: cargo test -- --nocapture
pub fn parse_to_shortcuts(input: &str) -> Result<shortcuts::ShortcutOwner, MarkupError>  {
    let lexemes = lexemes::lex(input)?;
    shortcuts::parse_unsorted(lexemes)
}

