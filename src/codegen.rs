use inkwell::basic_block::BasicBlock;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};

use crate::analysis::{
    lexer::*,
    parser::*,
};

use inkwell::passes::PassManager;
use inkwell::types::{BasicMetadataTypeEnum, FunctionType};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue};
use inkwell::AddressSpace::*;

struct Function<'fun>
{
    ty: FunctionType<'fun>,
    pub val: FunctionValue<'fun>,
}

impl<'fun> Function<'fun>
{
    fn new(
        name: &'fun str,
        ty: FunctionType<'fun>,
        linkage: Option<Linkage>,
        module: &Module<'fun>,
    ) -> Self
    {
        Function { ty, val: module.add_function(name, ty, linkage) }
    }
}

struct LibC<'lc>
{
    calloc: Function<'lc>,
    free: Function<'lc>,
    putchar: Function<'lc>,
    getchar: Function<'lc>,
}

impl<'lc> LibC<'lc>
{
    pub fn link_to_module(context: &'lc Context, module: &Module<'lc>) -> LibC<'lc>
    {
        let i8_ty = context.i8_type().to_owned();
        let i32_ty = context.i32_type().to_owned();
        let void_ty = context.void_type().to_owned();

        let calloc_ty = i8_ty
            .ptr_type(Generic)
            .fn_type(
                &[BasicMetadataTypeEnum::IntType(i32_ty), BasicMetadataTypeEnum::IntType(i32_ty)],
                false,
            )
            .to_owned();
        let calloc = Function::new("calloc", calloc_ty, Some(Linkage::External), &module);

        let free_ty = void_ty
            .fn_type(&[BasicMetadataTypeEnum::PointerType(i8_ty.ptr_type(Generic))], false)
            .to_owned();
        let free = Function::new("free", free_ty, Some(Linkage::External), &module);

        let putchar_ty =
            void_ty.fn_type(&[BasicMetadataTypeEnum::IntType(i8_ty)], false).to_owned();
        let putchar = Function::new("putchar", putchar_ty, Some(Linkage::External), &module);

        let getchar_ty = i8_ty.fn_type(&[], false);
        let getchar = Function::new("getchar", getchar_ty, Some(Linkage::External), &module);

        LibC { calloc, free, putchar, getchar }
    }
}

// Book-keeping for loop generation.
#[derive(Clone, Copy)]
struct LoopLabels<'lc>
{
    condition: BasicBlock<'lc>,
    body: BasicBlock<'lc>,
    end: BasicBlock<'lc>,
}

// As described in https://llvm.org/doxygen/group__LLVMCCoreContext.html#details
// LLVM Contexts are not thread safe.
pub struct CodegenCtx<'cg>
{
    source: &'cg ParsedRoutine<'cg>,
    ctx: Context,
}

impl<'cg> CodegenCtx<'cg>
{
    pub fn new(parse_result: &'cg ParsedRoutine<'cg>) -> Self
    {
        CodegenCtx { source: parse_result, ctx: Context::create() }
    }

    pub fn compile_to_module(&'cg self) -> Module<'cg>
    {
        let context = &self.ctx;
        let builder = context.create_builder();
        let routine_name = self.source.name.to_owned();
        let module = context.create_module(&routine_name);
        let fpm: PassManager<FunctionValue> = PassManager::create(&module);

        let i8_ty = context.i8_type().to_owned();
        let i32_ty = context.i32_type().to_owned();
        let routine_ty = i8_ty.fn_type(&[], false).to_owned();

        let LibC { calloc, free, putchar, getchar } = LibC::link_to_module(context, &module);

        let routine_fn =
            module.add_function(routine_name.as_str(), routine_ty, Some(Linkage::External));
        let routine_bb = context.append_basic_block(routine_fn, &routine_name);

        builder.position_at_end(routine_bb);

        // Allocate this routine's cells.
        let stack_size = i32_ty.const_int(30_000, false);
        let cell_width = i32_ty.const_int(1, false);
        let calloc_call = builder
            .build_call(
                calloc.val,
                &[
                    BasicMetadataValueEnum::IntValue(stack_size),
                    BasicMetadataValueEnum::IntValue(cell_width),
                ],
                "routine_cells",
            )
            .try_as_basic_value()
            .unwrap_left()
            .into_pointer_value(); // Get a pointer from this.
        let cells_idx = builder.build_alloca(i32_ty, "cells_idx");
        builder.build_store(cells_idx, i32_ty.const_int(0, false));

        let mut loop_stack: Vec<LoopLabels> = Vec::new();

        let get_cells_idx = || builder.build_load(cells_idx, "").into_int_value();

        // Build the routine
        let mut sblock_iter = self.source.data.iter();
        while let Some(sblock) = sblock_iter.next() {
            match sblock.parent {
                TokenKind::Right => {
                    let right =
                        builder.build_int_add(i32_ty.const_int(1, false), get_cells_idx(), "");
                    builder.build_store(cells_idx, right);
                }
                TokenKind::Left => {
                    let left =
                        builder.build_int_sub(get_cells_idx(), i32_ty.const_int(1, false), "");
                    builder.build_store(cells_idx, left);
                }

                TokenKind::Output => {
                    let cell = unsafe { builder.build_gep(calloc_call, &[get_cells_idx()], "") };
                    let cell_val = builder.build_load(cell, "").into_int_value();
                    builder.build_call(
                        putchar.val,
                        &[BasicMetadataValueEnum::IntValue(cell_val)],
                        "",
                    );
                }
                TokenKind::Input => {
                    let idx = builder.build_load(cells_idx, "").into_int_value();
                    let getchar_call = builder
                        .build_call(getchar.val, &[], "in")
                        .try_as_basic_value()
                        .left()
                        .unwrap();
                    let cell = unsafe { builder.build_gep(calloc_call, &[idx], "") };
                    builder.build_store(cell, getchar_call);
                }

                TokenKind::Inc => {
                    let idx = builder.build_load(cells_idx, "").into_int_value();
                    let cell = unsafe { builder.build_gep(calloc_call, &[idx], "") };
                    let mut cell_val = builder.build_load(cell, "").into_int_value();
                    cell_val = builder.build_int_add(cell_val, i8_ty.const_int(1, false), "");
                    builder.build_store(cell, cell_val);
                }
                TokenKind::Dec => {
                    let idx = builder.build_load(cells_idx, "").into_int_value();
                    let cell = unsafe { builder.build_gep(calloc_call, &[idx], "") };
                    let mut cell_val = builder.build_load(cell, "").into_int_value();
                    cell_val = builder.build_int_sub(cell_val, i8_ty.const_int(1, false), "");
                    builder.build_store(cell, cell_val);
                }

                TokenKind::BeginLoop => {
                    let this_loop = LoopLabels {
                        condition: context.append_basic_block(routine_fn, "l.cond"),
                        body: context.append_basic_block(routine_fn, "l.body"),
                        end: context.append_basic_block(routine_fn, "l.end"),
                    };

                    loop_stack.push(this_loop);

                    // We must find a way into the condition.
                    builder.build_unconditional_branch(this_loop.condition);
                    builder.position_at_end(this_loop.condition);

                    let cell =
                        unsafe { builder.build_gep(calloc_call, &[get_cells_idx()], "cell") };
                    let cell_val = builder.build_load(cell, "cell_val").into_int_value();

                    let zero_test = builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        cell_val,
                        i8_ty.const_int(0, false),
                        "l.zero_cmp",
                    );

                    builder.build_conditional_branch(zero_test, this_loop.end, this_loop.body);
                    builder.position_at_end(this_loop.body);
                }

                TokenKind::EndLoop => {
                    let latest_loop = loop_stack.pop().unwrap();
                    builder.build_unconditional_branch(latest_loop.condition);
                    builder.position_at_end(latest_loop.end);
                }

                _ => {}
            }
        }

        // Free the routine stack.
        let _free_call = builder.build_call(
            free.val,
            &[BasicMetadataValueEnum::PointerValue(calloc_call)],
            "free the tape",
        );

        builder.build_return(Some(&i8_ty.const_int(0, false)));

        fpm.add_dead_store_elimination_pass();
        fpm.add_promote_memory_to_register_pass();
        fpm.add_early_cse_mem_ssa_pass();
        fpm.add_licm_pass();
        fpm.add_reassociate_pass();
        fpm.run_on(&routine_fn);
        println!("Function Pass Manager changed the module: {}", fpm.initialize());

        let debug_filename = self.source.name.clone() + ".ll";
        module.print_to_file(debug_filename).expect("failed to compile");
        module
    }
}
