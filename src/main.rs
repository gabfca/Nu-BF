pub(crate) mod analysis;
pub(crate) mod codegen;

use clap::Parser;

mod driver
{
    use clap::Parser;
    use std::path::Path;

    use crate::{
        analysis::{lexer::LexedProgram, parser::ParsedProgram},
        codegen::ir_gen::IRContext,
    };
    use inkwell::{
        targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
        OptimizationLevel,
    };

    #[derive(Parser)]
    #[command(author, version, about, long_about = None)]
    pub(crate) struct Args
    {
        /// Input path
        #[arg(short, long)]
        pub(crate) r#in: Vec<String>,

        /// Output path
        #[arg(short, long)]
        pub(crate) out: String,

        /// Optimizaton level
        #[clap(short, long, default_value_t = 0)]
        pub(crate) level: u8,
    }

    pub(crate) fn get_target_machine(args: &Args) -> TargetMachine
    {
        Target::initialize_all(&InitializationConfig::default());

        let target_triple = TargetMachine::get_default_triple();
        let cpu = TargetMachine::get_host_cpu_name().to_string();
        let features = TargetMachine::get_host_cpu_features().to_string();

        let target = Target::from_triple(&target_triple).unwrap();

        target
            .create_target_machine(
                &target_triple,
                &cpu,
                &features,
                match args.level {
                    0 => OptimizationLevel::None,
                    1 => OptimizationLevel::Default,
                    2 => OptimizationLevel::Aggressive,
                    _ => OptimizationLevel::Aggressive,
                },
                RelocMode::Default,
                CodeModel::Default,
            )
            .unwrap()
    }

    pub(crate) fn compile<'c>(args: Args)
    {
        use inkwell::targets::FileType;

        let in_paths = &args.r#in;
        let out_path = &args.out;

        let mut ir_context = IRContext::new();
        let target_machine = get_target_machine(&args);

        let names_and_contents = in_paths
            .iter()
            .map(|path| {
                let path = Path::new(path);
                let name = path.file_stem().unwrap().to_str().unwrap();
                let contents = std::fs::read_to_string(path);
                (name.to_string(), contents.unwrap())
            })
            .collect::<Vec<_>>();
        let lex = LexedProgram::new(names_and_contents.as_slice());
        let parse = ParsedProgram::from(&lex);

        let ir = ir_context.compile(&parse);

        ir.routines.iter().for_each(|routine| {
            target_machine
                .write_to_file(&routine.module, FileType::Object, Path::new(out_path))
                .unwrap()
        })
    }
}

fn main()
{
    let args = driver::Args::parse();
    driver::compile(args);
}
