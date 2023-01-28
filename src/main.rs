use std::ops::Deref;
use std::path::PathBuf;
mod analysis;
mod codegen;

use crate::analysis::{lexer::*, parser::*};
use crate::codegen::preopt::*;
use crate::codegen::ir_gen::*;

fn build(input: &PathBuf, output: &PathBuf)
{
    let name = input.file_stem().unwrap().to_str().unwrap();
    let contents = std::fs::read_to_string(&input.as_path()).unwrap();
    let contents = contents.as_str();
    let input_file = [(name, contents)];

    let lexed = LexResult::new(&input_file);
    let parsed = ParseResult::from(&lexed);

    let l = PreOptRoutine::from(&parsed.parsed_routines[0]); 
    l.data.iter().for_each(|preopt_tok| {});
    for token in l.data { println!("{:#?}", token)}

    let codegen: CodegenCtx = CodegenCtx::new(&parsed);

    let result = codegen.compile_all();

    Target::initialize_all(&InitializationConfig::default());

    let target_triple = TargetMachine::get_default_triple();
    let cpu = TargetMachine::get_host_cpu_name().to_string();
    let features = TargetMachine::get_host_cpu_features().to_string();

    let target = Target::from_triple(&target_triple).unwrap();

    use inkwell::targets::{
        CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
    };
    use inkwell::OptimizationLevel;

    let target_machine = target
        .create_target_machine(
            &target_triple,
            &cpu,
            &features,
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
    .unwrap();

    for compiled_module in result 
    { target_machine.write_to_file(&compiled_module.deref(), FileType::Object, output.as_path()).unwrap();}
}

fn main()
{
    use std::env;
    let argv = env::args().collect::<Vec<_>>();

    let infile = &argv[1];
    let outfile = &argv[2];
    /* Move to the LexResult/ ParseResult bundled-source API */
    let inpath = PathBuf::from(infile.as_str());
    let outpath = PathBuf::from(outfile.as_str());

    build(&inpath, &outpath);
}
