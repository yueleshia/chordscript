use wasm_bindgen::prelude::*;

use chordscript::templates::{PreallocPush, Templates};
use chordscript::parser::parse_to_shortcuts;


//run: wasm-pack build --target web
#[wasm_bindgen]
pub fn add(a: String) -> String {
    let owner = parse_to_shortcuts(&a).unwrap();
    let output = String::new();
    let mut buffer = Vec::with_capacity(Templates::ShellScript.len(&owner));
    Templates::ShellScript.push_into(&owner, &mut buffer);
    buffer.join("")
}
