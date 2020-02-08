use generator::llvm_wrapper::llvm_sys::prelude::*;
use semantic::model::*;

#[derive(Clone)]
pub struct FnWithLLVM {
    pub sig: FunctionSignature,
    pub llvm: LLVMValueRef,
}
