use generator::llvm_wrapper::llvm_sys::prelude::*;
use generator::llvm_wrapper::*;
use generator::native_funcs::FnWithAnalyticsAndLLVM;
use parsing::model::Ident;
use semantic::model::*;
use std::collections::*;
use std::rc::Rc;

pub struct CodeGenerator<'a> {
    llvm: &'a mut LLVMWrapper,
    native_funcs: &'a [FnWithAnalyticsAndLLVM],
}

impl<'a> CodeGenerator<'a> {
    pub fn new(
        llvm: &'a mut LLVMWrapper,
        native_funcs: &'a [FnWithAnalyticsAndLLVM],
    ) -> CodeGenerator<'a> {
        CodeGenerator { llvm, native_funcs }
    }
    pub fn gen_program(&mut self, func_ast: HashMap<Rc<FunctionSignature>, FnWithHints>) {
        unsafe {
            let main_sig = gen_main_sig(&mut self.llvm);
            let mut funcs = gen_funcs_sig(&func_ast, &mut self.llvm);
            let mut signatures: HashMap<FuncName, LLVMValueRef> =
                into_funcs_sig_map(self.native_funcs);
            let code_func_signatures: HashMap<FuncName, LLVMValueRef> =
                funcs.iter().map(|(n, fs)| (n.clone(), fs.llvm)).collect();
            signatures.extend(code_func_signatures);
            gen_func_impls(func_ast, &mut self.llvm, &mut funcs, &signatures);
            gen_main(
                &mut self.llvm,
                main_sig,
                *signatures.get(&FuncName::new("main")).unwrap(),
            );
        }
        self.llvm.dump("output");
        self.llvm.mk_object_file("output");
    }
}

#[derive(Clone)]
struct FnWithHintsAndLLVM {
    pub func_with_hints: FnWithHints,
    pub llvm: LLVMValueRef,
}

unsafe fn gen_func_impls(
    func_ast: HashMap<Rc<FunctionSignature>, FnWithHints>,
    mut llvm: &mut LLVMWrapper,
    funcs: &mut HashMap<FuncName, FnWithHintsAndLLVM>,
    signatures: &HashMap<FuncName, LLVMValueRef>,
) {
    func_ast.iter().for_each(|(sig, func)| {
        generate_func(sig, func, &funcs, signatures, &mut llvm);
    });
}

//fn print_funcs(funcs: &mut HashMap<String, FnWithLLVM>) {
//    println!("Function signatures: ");
//    funcs.iter().for_each(|(n, _)| {
//        println!("{}", n);
//    });
//}

unsafe fn gen_funcs_sig(
    func_impls: &HashMap<Rc<FunctionSignature>, FnWithHints>,
    llvm: &mut LLVMWrapper,
) -> HashMap<FuncName, FnWithHintsAndLLVM> {
    func_impls
        .iter()
        .map(|(_, fwa)| gen_func_sig(&fwa, llvm))
        .map(|f| (f.func_with_hints.sig.name.clone(), f))
        .collect()
}

unsafe fn gen_func_sig(fwa: &FnWithHints, llvm: &mut LLVMWrapper) -> FnWithHintsAndLLVM {
    let types = llvm.types;
    let mut args: Vec<LLVMTypeRef> = fwa
        .sig
        .args
        .iter()
        .map(|_| ptr_t(types.mp_struct))
        .collect();
    let ret_type = ptr_t(types.mp_struct);
    let func_type = function_type(ret_type, &mut args, false);
    let function: LLVMValueRef = llvm.add_function(&fwa.sig.name, func_type);
    FnWithHintsAndLLVM {
        func_with_hints: FnWithHints {
            sig: fwa.clone().sig,
            body: fwa.clone().body,
            hints: fwa.clone().hints,
        },
        llvm: function,
    }
}

unsafe fn gen_main_sig(llvm: &mut LLVMWrapper) -> LLVMValueRef {
    let types = llvm.types;
    llvm.add_function(
        &FuncName::new("main"),
        function_type(types.i32t, &mut [], false),
    )
}

unsafe fn gen_main(
    llvm: &mut LLVMWrapper,
    main_sig: LLVMValueRef,
    internal_main_sig: LLVMValueRef,
) {
    let types = llvm.types;
    let basic_block = llvm.append_basic_block_in_context(main_sig, "entrypoint");
    llvm.position_builder_at_end(basic_block);
    //    funcs.iter().for_each(|v|println!("{:?}",v));
    llvm.build_call(internal_main_sig, &mut [], "call_internal_main");
    llvm.build_ret(const_int(types.i32t, 0, 0));
}

fn into_funcs_sig_map(func_sigs: &[FnWithAnalyticsAndLLVM]) -> HashMap<FuncName, LLVMValueRef> {
    func_sigs
        .iter()
        .map(|fn_with_llvm| {
            (
                fn_with_llvm.func_with_analytics.sig.name.clone(),
                fn_with_llvm.llvm,
            )
        })
        .collect()
}

unsafe fn generate_func(
    sig: &FunctionSignature,
    func: &FnWithHints,
    funcs: &HashMap<FuncName, FnWithHintsAndLLVM>,
    signatures: &HashMap<FuncName, LLVMValueRef>,
    llvm: &mut LLVMWrapper,
) {
    let loaded_func = funcs.get(&sig.name).unwrap();
    let block = llvm.append_basic_block_in_context(loaded_func.llvm, "entrypoint");
    llvm.position_builder_at_end(block);
    let result: LLVMValueRef = gen_expr(sig, funcs, signatures, llvm, &func.body);
    llvm.build_ret(result);
}

unsafe fn gen_expr(
    func: &FunctionSignature,
    funcs: &HashMap<FuncName, FnWithHintsAndLLVM>,
    signatures: &HashMap<FuncName, LLVMValueRef>,
    llvm: &mut LLVMWrapper,
    expr: &Expression,
) -> LLVMValueRef {
    use semantic::model::Expression::*;
    match expr {
        FunctionCall(sig) => gen_func_call(func, &sig, funcs, signatures, llvm),
        Constant(int) => gen_read_const(&int, signatures, llvm),
        FunctionArgument(name) => gen_function_argument(&func.name, name.str(), funcs, signatures),
        SubExpression(exprs) => {
            let results: Vec<LLVMValueRef> = exprs
                .iter()
                .map(|e| gen_expr(&func, funcs, signatures, llvm, e))
                .collect();
            match results.last() {
                Some(e) => *e,
                None => llvm.build_ret_void(),
            }
        }
    }
}

unsafe fn gen_func_call(
    func: &FunctionSignature,
    sig: &FunctionCallSignature,
    funcs: &HashMap<FuncName, FnWithHintsAndLLVM>,
    signatures: &HashMap<FuncName, LLVMValueRef>,
    llvm: &mut LLVMWrapper,
) -> LLVMValueRef {
    let mut args_result: Vec<LLVMValueRef> = sig
        .args
        .iter()
        .map(|expr| gen_expr(func, funcs, signatures, llvm, expr))
        .collect();
    llvm.build_call(
        signatures.get(&sig.name).unwrap().clone(),
        &mut args_result,
        &format!("call_{}", &sig.name),
    )
}

unsafe fn gen_read_const(
    int: &Ident,
    signatures: &HashMap<FuncName, LLVMValueRef>,
    llvm: &mut LLVMWrapper,
) -> LLVMValueRef {
    let types = llvm.types;
    let mp_struct_ptr_to_ptr = llvm.build_alloca(ptr_t(types.mp_struct), "mp_struct_ptr_to_ptr");
    let const_24 = const_int(types.i64t, 24, 0);
    let ptr_for_num = llvm.build_call(
        signatures.get(&FuncName::new("malloc")).unwrap().clone(),
        &mut [const_24],
        "ptr_for_num",
    );
    let ptr_to_num = llvm.build_bit_cast(ptr_for_num, ptr_t(types.mp_struct), "ptr_to_num");
    llvm.build_store(ptr_to_num, mp_struct_ptr_to_ptr);
    let loaded_ptr_to_num = llvm.build_load(mp_struct_ptr_to_ptr, "loaded_ptr_to_num");
    let mp_init_func = *signatures.get(&FuncName::new("mp_init")).unwrap();
    llvm.build_call(mp_init_func, &mut [loaded_ptr_to_num], "mp_init_res");
    let str_num_const = llvm.build_global_string(int.str(), &format!("str_num_const_{}", int));
    let num_str_ptr = llvm.build_struct_gep(str_num_const, 0, "str_num_const_ptr");
    let const_10 = const_int(types.i32t, 10, 0);
    llvm.build_call(
        signatures
            .get(&FuncName::new("mp_read_radix"))
            .unwrap()
            .clone(),
        &mut [loaded_ptr_to_num, num_str_ptr, const_10],
        "mp_read_radix",
    );
    loaded_ptr_to_num
}

unsafe fn gen_function_argument(
    func_name: &FuncName,
    arg_name: &str,
    funcs: &HashMap<FuncName, FnWithHintsAndLLVM>,
    signatures: &HashMap<FuncName, LLVMValueRef>,
) -> LLVMValueRef {
    let func = signatures.get(func_name).unwrap();
    let args: Vec<String> = funcs
        .get(func_name)
        .unwrap()
        .func_with_hints
        .sig
        .args
        .iter()
        .map(|a| a.str().to_owned())
        .collect();
    let index = args.iter().position(|a| a == arg_name).unwrap();
    get_param(*func, index as u32)
}
