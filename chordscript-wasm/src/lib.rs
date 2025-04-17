use wasm_bindgen::prelude::*;

//run: wasm-pack build --target web
#[wasm_bindgen]
pub fn add(a: String) -> String {
    chordscript::add(&a)
}
