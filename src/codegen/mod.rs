pub mod ir_gen;

use inkwell::module::{Linkage, Module};
use inkwell::types::FunctionType;
use inkwell::values::FunctionValue;

pub struct Function<'fun>
{
    pub ty: FunctionType<'fun>,
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
        Self { ty, val: module.add_function(name, ty, linkage) }
    }
}

pub mod detail
{
    use super::Function;
    use inkwell::module::{Linkage, Module};
    use inkwell::types::BasicMetadataTypeEnum;
    use inkwell::AddressSpace;

    pub struct LibC<'mni>
    {
        pub calloc: Function<'mni>,
        pub free: Function<'mni>,
        pub putchar: Function<'mni>,
        pub getchar: Function<'mni>,
    }

    impl<'lc> LibC<'lc>
    {
        pub fn link_to_module(module: &Module<'lc>) -> LibC<'lc>
        {
            let context = module.get_context();

            let i8_ty = context.i8_type().to_owned();
            let i32_ty = context.i32_type().to_owned();
            let void_ty = context.void_type().to_owned();

            let calloc_ty = i8_ty
                .ptr_type(AddressSpace::default())
                .fn_type(
                    &[
                        BasicMetadataTypeEnum::IntType(i32_ty),
                        BasicMetadataTypeEnum::IntType(i32_ty),
                    ],
                    false,
                )
                .to_owned();
            let calloc = Function::new("calloc", calloc_ty, Some(Linkage::External), module);

            let free_ty = void_ty
                .fn_type(
                    &[BasicMetadataTypeEnum::PointerType(i8_ty.ptr_type(AddressSpace::default()))],
                    false,
                )
                .to_owned();
            let free = Function::new("free", free_ty, Some(Linkage::External), module);

            let putchar_ty =
                void_ty.fn_type(&[BasicMetadataTypeEnum::IntType(i8_ty)], false).to_owned();
            let putchar = Function::new("putchar", putchar_ty, Some(Linkage::External), module);

            let getchar_ty = i8_ty.fn_type(&[], false);
            let getchar = Function::new("getchar", getchar_ty, Some(Linkage::External), module);

            LibC { calloc, free, putchar, getchar }
        }
    }
}
