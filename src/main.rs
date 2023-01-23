use std::path::PathBuf;
mod analysis;
mod codegen;

use crate::analysis::{
    lexer::*,
    parser::*,
};
use crate::codegen::*;

fn main()
{
    use std::env;
    let _argv = env::args().collect::<Vec<_>>();

    /* Move to the LexResult/ ParseResult bundled-source API */
    let path = PathBuf::from("src/main.nbf");
    let contents = std::fs::read_to_string(&path.as_path()).unwrap();
    let contents = contents.as_str();

    let name = path.file_stem().unwrap().to_str().unwrap();
    let piped = [(name, contents)];

    let lexed_files = LexResult::new(&piped);

    lexed_files.lexed_routines[0].tokens.iter().for_each(|token| println!("{:#?}", token));

    let parsed = ParsedRoutine::from(&lexed_files.lexed_routines[0]);
    parsed.data.iter().for_each(|block| println!("{:#?}, {:#?}", block.parent, block.children));

    let a = CodegenCtx::new(&parsed);
    a.compile_to_module();

    // &std::fs::read_to_string(source.as_path()).unwrap()
    //                 let this_name = source.file_stem().unwrap().to_str().unwrap();
}
