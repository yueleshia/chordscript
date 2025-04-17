use wasm_bindgen::prelude::*;

//use chordscript::templates::{PreallocPush, Templates};
use chordscript::parser::parse_to_shortcuts;
use chordscript::Format;


//run: ../make.sh
#[wasm_bindgen]
pub fn parse(a: String, format_id: usize) -> Result<String, String> {
    let owner = parse_to_shortcuts(&a).map_err(|err| format!("{}", err))?;

    let format = Format {
        id: format_id,
        runner: "<shortcuts.sh>",
    };

    Ok(format.pipe_to_string(&owner))
}
