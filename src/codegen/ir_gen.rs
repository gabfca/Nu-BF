use inkwell::{
    basic_block::BasicBlock,
    context::Context,
    module::{Linkage, Module},
    values::BasicMetadataValueEnum,
};

use crate::analysis::{lexer::*, parser::*};

use crate::codegen::detail::LibC;

#[derive(Clone, Copy)]
pub(crate) struct LoopLabels<'lc>
{
    pub(crate) condition: BasicBlock<'lc>,
    pub(crate) body: BasicBlock<'lc>,
    pub(crate) end: BasicBlock<'lc>,
}

pub(crate) struct IRRoutine<'r>
{
    name: String,
    pub(crate) module: Module<'r>,
}

pub(crate) struct IRProgram<'p>
{
    pub(crate) routines: Vec<IRRoutine<'p>>,
}

// It's best to keep ownership of the context outside of the IRProgram itself.
pub(crate) struct IRContext
{
    ctx: Context,
}

impl IRContext
{
    pub(crate) fn new() -> IRContext
    {
        IRContext { ctx: Context::create() }
    }

    pub(crate) fn compile<'irc>(&'irc mut self, parsed_program: &ParsedProgram) -> IRProgram<'irc>
    {
        let context = &self.ctx;

        let mut routines = Vec::new();

        for parsed_routine in &parsed_program.routines {
            let builder = context.create_builder();
            let routine_name = parsed_routine.name.to_owned();
            let module = context.create_module(&parsed_routine.name);

            let i8_ty = context.i8_type().to_owned();
            let i32_ty = context.i32_type().to_owned();
            let routine_ty = i8_ty.fn_type(&[], false).to_owned();

            let LibC { calloc, free, putchar, getchar } = LibC::link_to_module(&module);

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

            let tokens_iter = parsed_routine.data.iter();

            for tok in tokens_iter {
                match tok.kind {
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
                        let cell: inkwell::values::PointerValue =
                            unsafe { builder.build_gep(calloc_call, &[get_cells_idx()], "") };
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

            let _free_call = builder.build_call(
                free.val,
                &[BasicMetadataValueEnum::PointerValue(calloc_call)],
                "",
            );

            builder.build_return(Some(&i8_ty.const_int(0, false)));

            routines.push(IRRoutine { name: parsed_routine.name.clone(), module })
        }

        IRProgram { routines }
    }
}
