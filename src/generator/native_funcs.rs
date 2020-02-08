use aspects::complexity::model::{ComplexityAnalytics, ComplexityAnalyticsValue};
use aspects::model::AnalyticsFields;
use aspects::register::AnalyticsWrapper;
use aspects::side_effect::analytics::{create_complexity_analytics, create_side_effect_analytics};
use aspects::side_effect::model::SideEffectAnalyticsValue;
use generator::llvm_wrapper::llvm_sys::prelude::*;
use generator::llvm_wrapper::*;
use parsing::model::Ident;
use semantic::model::{FnWithAnalytics, FuncName, FunctionSignature};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct FnWithAnalyticsAndLLVM {
    pub func_with_analytics: FnWithAnalytics,
    pub llvm: LLVMValueRef,
}

pub struct NativeFuncsGenerator {
    native_funcs: Vec<FnWithAnalyticsAndLLVM>,
}

impl NativeFuncsGenerator {
    pub fn new(llvm: &mut LLVMWrapper) -> NativeFuncsGenerator {
        unsafe {
            let native_funcs = add_native_funcs(llvm);
            NativeFuncsGenerator { native_funcs }
        }
    }

    pub fn native_funcs(&self) -> Vec<FnWithAnalyticsAndLLVM> {
        self.native_funcs.clone()
    }
}

pub fn to_native_funcs_map(
    native_funcs: &[FnWithAnalyticsAndLLVM],
) -> HashMap<FuncName, FnWithAnalytics> {
    native_funcs
        .iter()
        .map(|f| {
            (
                f.func_with_analytics.sig.name.clone(),
                f.func_with_analytics.clone(),
            )
        })
        .collect()
}

pub unsafe fn add_native_funcs(llvm: &mut LLVMWrapper) -> Vec<FnWithAnalyticsAndLLVM> {
    let mut external_funcs = mk_external_funcs(llvm);
    external_funcs.extend(mk_native_funcs(&external_funcs, llvm));
    external_funcs
}

unsafe fn mk_external_funcs(llvm: &mut LLVMWrapper) -> Vec<FnWithAnalyticsAndLLVM> {
    vec![
        mk_printf(llvm),
        mk_fgetc(llvm),
        mk_malloc(llvm),
        mk_mp_toradix(llvm),
        mk_mp_init(llvm),
        mk_mp_clear(llvm),
        mk_mp_read_radix(llvm),
        mk_mp_radix_size(llvm),
        mk_two_ar_num_func(&FuncName::new("mp_add"), llvm),
        mk_two_ar_num_func(&FuncName::new("mp_sub"), llvm),
        mk_two_ar_num_func(&FuncName::new("mp_mul"), llvm),
        mk_two_ar_num_func(&FuncName::new("mp_div"), llvm),
    ]
}

unsafe fn mk_native_funcs(
    external_funcs: &[FnWithAnalyticsAndLLVM],
    llvm: &mut LLVMWrapper,
) -> Vec<FnWithAnalyticsAndLLVM> {
    let external_funcs_map = mk_funcs_map(external_funcs);
    vec![
        add_println_func(&external_funcs_map, llvm),
        add_getdgt_func(&external_funcs_map, llvm),
        add_add_func(&external_funcs_map, llvm),
        add_sub_func(&external_funcs_map, llvm),
        add_mul_func(&external_funcs_map, llvm),
        add_div_func(&external_funcs_map, llvm),
    ]
}

unsafe fn mk_funcs_map(
    external_funcs: &[FnWithAnalyticsAndLLVM],
) -> HashMap<FuncName, &FnWithAnalyticsAndLLVM> {
    external_funcs
        .iter()
        .map(|f| (f.func_with_analytics.sig.name.clone(), f))
        .collect()
}

unsafe fn add_println_func(
    external_funcs: &HashMap<FuncName, &FnWithAnalyticsAndLLVM>,
    llvm: &mut LLVMWrapper,
) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("println");
    let mut func_args = vec![ptr_t(types.mp_struct)];
    let println_func = llvm.add_function(
        name,
        function_type(ptr_t(types.mp_struct), &mut func_args, false),
    );
    let basic_block = llvm.append_basic_block_in_context(println_func, "entrypoint");
    llvm.position_builder_at_end(basic_block);
    let str_size_ptr = llvm.build_alloca(types.i32t, "str_size_ptr");
    let const_10 = const_int(types.i32t, 10, 0);
    let mp_struct_ptr = get_param(println_func, 0);
    let mp_radix_size_func = external_funcs.get(&FuncName::new("mp_radix_size")).unwrap();
    llvm.build_call(
        mp_radix_size_func.llvm,
        &mut [mp_struct_ptr, const_10, str_size_ptr],
        "mp_radix_size_res",
    );
    let str_size_32 = llvm.build_load(str_size_ptr, "str_size_32");
    let str_size_64 = llvm.build_s_ext(str_size_32, types.i64t, "str_size_64");
    let ptr_to_str = llvm.build_call(
        external_funcs.get(&FuncName::new("malloc")).unwrap().llvm,
        &mut [str_size_64],
        "ptr_to_str",
    );
    let mp_toradix_func = external_funcs.get(&FuncName::new("mp_toradix")).unwrap();
    let const_10 = const_int(types.i32t, 10, 0);
    llvm.build_call(
        mp_toradix_func.llvm,
        &mut [mp_struct_ptr, ptr_to_str, const_10],
        "call_mp_toradix",
    );
    let template = llvm.build_global_string("%s\n", "template");
    let template_ptr = llvm.build_struct_gep(template, 0, "template_ptr");
    let printf_func = external_funcs.get(&FuncName::new("printf")).unwrap();
    llvm.build_call(
        printf_func.llvm,
        &mut [template_ptr, ptr_to_str],
        "call_printf",
    );
    llvm.build_ret(mp_struct_ptr);
    let args = mk_str_args(&func_args);

    FnWithAnalyticsAndLLVM {
        func_with_analytics: FnWithAnalytics {
            sig: Rc::new(FunctionSignature {
                name: name.clone(),
                args: args.clone(),
            }),
            analytics: vec![
                create_side_effect_analytics(SideEffectAnalyticsValue::ConsoleOutput),
                create_complexity_analytics(&args[0], ComplexityAnalyticsValue::ON),
            ],
        },
        llvm: println_func,
    }
}

unsafe fn add_getc_func(
    external_funcs: &HashMap<FuncName, &FnWithAnalyticsAndLLVM>,
    llvm: &mut LLVMWrapper,
) -> LLVMValueRef {
    let types = llvm.types;
    let getc_func = llvm.add_function(
        &FuncName::new("getc"),
        function_type(types.i8t, &mut [], false),
    );
    let basic_block = llvm.append_basic_block_in_context(getc_func, "entrypoint");
    llvm.position_builder_at_end(basic_block);
    let file_struct_ptr = llvm.add_global(ptr_t(types.file_struct), "__stdinp");
    let file_struct_ptr = llvm.build_load(file_struct_ptr, "file_struct_ptr");
    let fgetc_func = external_funcs.get(&FuncName::new("fgetc")).unwrap();
    let ch = llvm.build_call(fgetc_func.llvm, &mut [file_struct_ptr], "ch");
    let trunc_ch = llvm.build_trunc(ch, types.i8t, "trunc_ch");
    llvm.build_ret(trunc_ch);
    getc_func
}

unsafe fn add_getdgt_func(
    external_funcs: &HashMap<FuncName, &FnWithAnalyticsAndLLVM>,
    llvm: &mut LLVMWrapper,
) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let getc_func = add_getc_func(external_funcs, llvm);
    let mut func_args = vec![];
    let name = "getdgt";
    let getdgt_func = llvm.add_function(
        &FuncName::new(name),
        function_type(ptr_t(types.mp_struct), &mut func_args, false),
    );
    let basic_block = llvm.append_basic_block_in_context(getdgt_func, "entrypoint");
    llvm.position_builder_at_end(basic_block);
    let in_str_ptr = llvm.build_alloca(arr_t(types.i8t, 2), "in_str_ptr");
    let const_24 = const_int(types.i64t, 24, 0);
    let ptr_for_num = llvm.build_call(
        external_funcs.get(&FuncName::new("malloc")).unwrap().llvm,
        &mut [const_24],
        "ptr_for_num",
    );
    let mp_struct_ptr = llvm.build_bit_cast(ptr_for_num, ptr_t(types.mp_struct), "mp_struct_ptr");
    let mp_init_func = external_funcs.get(&FuncName::new("mp_init")).unwrap();
    llvm.build_call(mp_init_func.llvm, &mut [mp_struct_ptr], "mp_init_res");
    let end_of_in_str = llvm.build_struct_gep(in_str_ptr, 1, "end_of_in_str");
    let const_0 = const_int(types.i8t, 0, 0);
    llvm.build_store(const_0, end_of_in_str);
    let ch = llvm.build_call(getc_func, &mut [], "ch");
    llvm.build_call(getc_func, &mut [], "new_line_reader");
    let begin_of_in_str = llvm.build_struct_gep(in_str_ptr, 0, "begin_of_in_str");
    llvm.build_store(ch, begin_of_in_str);
    let mp_read_radix_func = external_funcs.get(&FuncName::new("mp_read_radix")).unwrap();
    let const_10 = const_int(types.i32t, 10, 0);
    let begin_of_in_str = llvm.build_struct_gep(in_str_ptr, 0, "begin_of_in_str");
    llvm.build_call(
        mp_read_radix_func.llvm,
        &mut [mp_struct_ptr, begin_of_in_str, const_10],
        "mp_read_radix_result",
    );
    llvm.build_ret(mp_struct_ptr);
    let args = mk_str_args(&func_args);
    FnWithAnalyticsAndLLVM {
        func_with_analytics: FnWithAnalytics {
            sig: Rc::new(FunctionSignature {
                name: FuncName::new(name),
                args,
            }),
            analytics: vec![
                create_side_effect_analytics(SideEffectAnalyticsValue::ConsoleInput),
                //                Some(create_complexity_analytics(ComplexityAnalytics::ON)),
            ],
        },
        llvm: getdgt_func,
    }
}

unsafe fn add_add_func(
    external_funcs: &HashMap<FuncName, &FnWithAnalyticsAndLLVM>,
    llvm: &mut LLVMWrapper,
) -> FnWithAnalyticsAndLLVM {
    two_ar_num_func(&FuncName::new("add"), external_funcs, llvm)
}

unsafe fn add_sub_func(
    external_funcs: &HashMap<FuncName, &FnWithAnalyticsAndLLVM>,
    llvm: &mut LLVMWrapper,
) -> FnWithAnalyticsAndLLVM {
    two_ar_num_func(&FuncName::new("sub"), external_funcs, llvm)
}

unsafe fn add_mul_func(
    external_funcs: &HashMap<FuncName, &FnWithAnalyticsAndLLVM>,
    llvm: &mut LLVMWrapper,
) -> FnWithAnalyticsAndLLVM {
    two_ar_num_func(&FuncName::new("mul"), external_funcs, llvm)
}

unsafe fn add_div_func(
    external_funcs: &HashMap<FuncName, &FnWithAnalyticsAndLLVM>,
    llvm: &mut LLVMWrapper,
) -> FnWithAnalyticsAndLLVM {
    two_ar_num_func(&FuncName::new("div"), external_funcs, llvm)
}

unsafe fn two_ar_num_func(
    name: &FuncName,
    external_funcs: &HashMap<FuncName, &FnWithAnalyticsAndLLVM>,
    llvm: &mut LLVMWrapper,
) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let mut func_args = vec![ptr_t(types.mp_struct), ptr_t(types.mp_struct)];
    let mp_func = llvm.add_function(
        name,
        function_type(ptr_t(types.mp_struct), &mut func_args, false),
    );
    let basic_block = llvm.append_basic_block_in_context(mp_func, "entrypoint");
    llvm.position_builder_at_end(basic_block);
    let const_24 = const_int(types.i64t, 24, 0);
    let ptr_for_num = llvm.build_call(
        external_funcs.get(&FuncName::new("malloc")).unwrap().llvm,
        &mut [const_24],
        "ptr_for_num",
    );
    let ptr_to_num = llvm.build_bit_cast(ptr_for_num, ptr_t(types.mp_struct), "ptr_to_num");
    let mp_init_func = external_funcs.get(&FuncName::new("mp_init")).unwrap();
    llvm.build_call(mp_init_func.llvm, &mut [ptr_to_num], "mp_init_res");
    let mp_sub_func = external_funcs
        .get(&FuncName::new(&format!("mp_{}", name)))
        .unwrap();
    llvm.build_call(
        mp_sub_func.llvm,
        &mut [get_param(mp_func, 0), get_param(mp_func, 1), ptr_to_num],
        &format!("call_mp_{}", name),
    );
    llvm.build_ret(ptr_to_num);
    let args = mk_str_args(&func_args);
    FnWithAnalyticsAndLLVM {
        func_with_analytics: FnWithAnalytics {
            sig: Rc::new(FunctionSignature {
                name: name.clone(),
                args: args.clone(),
            }),
            analytics: vec![
                create_side_effect_analytics(SideEffectAnalyticsValue::None),
                {
                    let mut values = HashMap::new();
                    values.insert(args[0].clone(), ComplexityAnalyticsValue::ON);
                    values.insert(args[1].clone(), ComplexityAnalyticsValue::ON);
                    AnalyticsWrapper::Complexity(Rc::new(ComplexityAnalytics { values }))
                },
            ],
        },
        llvm: mp_func,
    }
}

unsafe fn mk_malloc(llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("malloc");
    let mut llvm_args = vec![types.i64t];
    let llvm = llvm.add_function(
        &name,
        function_type(ptr_t(types.i8t), &mut llvm_args, false),
    );
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::None),
            //            create_complexity_analytics(ComplexityAnalytics::OC),
        ],
    )
}

unsafe fn mk_printf(llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("printf");
    let mut llvm_args = vec![ptr_t(types.i8t)];
    let llvm = llvm.add_function(&name, function_type(types.i32t, &mut llvm_args, true));
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::ConsoleOutput),
            create_complexity_analytics(&Ident::new("arg"), ComplexityAnalyticsValue::ON),
        ],
    )
}

unsafe fn mk_mp_toradix(llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("mp_toradix");
    let mut llvm_args = vec![ptr_t(types.mp_struct), ptr_t(types.i8t), types.i32t];
    let llvm = llvm.add_function(&name, function_type(types.i32t, &mut llvm_args, false));
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::None),
            //            create_complexity_analytics(ComplexityAnalytics::ON),
        ],
    )
}

unsafe fn mk_mp_init(llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("mp_init");
    let mut llvm_args = vec![ptr_t(types.mp_struct)];
    let llvm = llvm.add_function(&name, function_type(types.i32t, &mut llvm_args, false));
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::None),
            //            create_complexity_analytics(ComplexityAnalytics::OC),
        ],
    )
}

unsafe fn mk_mp_clear(llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("mp_clear");
    let mut llvm_args = vec![ptr_t(types.mp_struct)];
    let llvm = llvm.add_function(&name, function_type(types.void, &mut llvm_args, false));
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::None),
            //            create_complexity_analytics(ComplexityAnalytics::OC),
        ],
    )
}

unsafe fn mk_mp_read_radix(llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("mp_read_radix");
    let mut llvm_args = vec![ptr_t(types.mp_struct), ptr_t(types.i8t), types.i32t];
    let llvm = llvm.add_function(&name, function_type(types.i32t, &mut llvm_args, false));
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::None),
            //            create_complexity_analytics(ComplexityAnalytics::ON),
        ],
    )
}

unsafe fn mk_mp_radix_size(llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("mp_radix_size");
    let mut llvm_args = vec![ptr_t(types.mp_struct), types.i32t, ptr_t(types.i32t)];
    let llvm = llvm.add_function(&name, function_type(types.i32t, &mut llvm_args, false));
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::None),
            //            create_complexity_analytics(ComplexityAnalytics::ON),
        ],
    )
}

unsafe fn mk_two_ar_num_func(name: &FuncName, llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let mut llvm_args = vec![
        ptr_t(types.mp_struct),
        ptr_t(types.mp_struct),
        ptr_t(types.mp_struct),
    ];
    let llvm = llvm.add_function(&name, function_type(types.i32t, &mut llvm_args, false));
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::None),
            //            create_complexity_analytics(ComplexityAnalytics::ON),
        ],
    )
}

unsafe fn mk_fgetc(llvm: &mut LLVMWrapper) -> FnWithAnalyticsAndLLVM {
    let types = llvm.types;
    let name = &FuncName::new("fgetc");
    let mut llvm_args = vec![ptr_t(types.file_struct)];
    let llvm = llvm.add_function(&name, function_type(types.i32t, &mut llvm_args, true));
    mk_fn_with_llvm(
        name,
        llvm_args,
        llvm,
        vec![
            create_side_effect_analytics(SideEffectAnalyticsValue::ConsoleInput),
            //            create_complexity_analytics(ComplexityAnalytics::OC),
        ],
    )
}

fn mk_fn_with_llvm(
    name: &FuncName,
    llvm_args: Vec<LLVMTypeRef>,
    llvm: LLVMValueRef,
    analytics: AnalyticsFields,
) -> FnWithAnalyticsAndLLVM {
    let args = mk_str_args(&llvm_args);
    FnWithAnalyticsAndLLVM {
        func_with_analytics: FnWithAnalytics {
            sig: Rc::new(FunctionSignature {
                name: name.clone(),
                args,
            }),
            analytics,
        },
        llvm,
    }
}

fn mk_str_args(func_args: &[LLVMTypeRef]) -> Vec<Ident> {
    let mut n = 0;
    func_args
        .iter()
        .map(|_| {
            n += 1;
            Ident::new(&format!("a{}", n))
        })
        .collect()
}

#[cfg(test)]
mod test_complexity {
    use generator::native_funcs::mk_str_args;
    use llvm_sys::core::{LLVMContextCreate, LLVMInt32TypeInContext};
    use parsing::model::Ident;

    #[test]
    pub fn mk_str_args_mapping() {
        unsafe {
            let context = LLVMContextCreate();
            let result = mk_str_args(&[
                LLVMInt32TypeInContext(context),
                LLVMInt32TypeInContext(context),
                LLVMInt32TypeInContext(context),
            ]);
            assert_eq!(
                result,
                &[Ident::new("a1"), Ident::new("a2"), Ident::new("a3")]
            );
        };
    }
}
