use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_native;
use std::collections::HashMap;
use std::mem;

use super::dataflow::{BinaryOp, DataflowExpr, UnaryOp};

/// JIT compiler for ultra-fast node execution
/// Compiles deterministic nodes to native code for 20-50ns execution
pub struct JITCompiler {
    /// The JIT module
    module: JITModule,
    /// Context for code generation
    ctx: codegen::Context,
    /// Function builder context
    func_ctx: FunctionBuilderContext,
    /// Compiled function IDs
    compiled_funcs: HashMap<String, FuncId>,
}

impl JITCompiler {
    /// Create new JIT compiler
    pub fn new() -> Result<Self, String> {
        // Get native target
        let isa = cranelift_native::builder()
            .map_err(|e| format!("Failed to create ISA builder: {}", e))?
            .finish(settings::Flags::new(settings::builder()))
            .map_err(|e| format!("Failed to create ISA: {}", e))?;

        // Create JIT builder
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Create module
        let module = JITModule::new(builder);

        // Create contexts
        let ctx = module.make_context();
        let func_ctx = FunctionBuilderContext::new();

        Ok(Self {
            module,
            ctx,
            func_ctx,
            compiled_funcs: HashMap::new(),
        })
    }

    /// Compile a simple arithmetic dataflow node
    /// This demonstrates compiling a node that does: output = input * 2 + offset
    pub fn compile_arithmetic_node(
        &mut self,
        name: &str,
        multiply_factor: i64,
        offset: i64,
    ) -> Result<*const u8, String> {
        // Clear the context for a fresh function
        self.ctx.clear();

        // Define function signature: fn(input: i64) -> i64
        let int_type = types::I64;
        self.ctx.func.signature.params.push(AbiParam::new(int_type));
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(int_type));

        // Declare the function
        let func_id = self
            .module
            .declare_function(name, Linkage::Local, &self.ctx.func.signature)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        {
            // Build the function body
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.func_ctx);

            // Create entry block
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Get the input parameter
            let input = builder.block_params(entry_block)[0];

            // Perform computation: result = input * multiply_factor + offset
            let factor = builder.ins().iconst(int_type, multiply_factor);
            let multiplied = builder.ins().imul(input, factor);
            let offset_val = builder.ins().iconst(int_type, offset);
            let result = builder.ins().iadd(multiplied, offset_val);

            // Return the result
            builder.ins().return_(&[result]);

            // Finalize
            builder.seal_all_blocks();
            builder.finalize();
        }

        // Define the function
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define function: {}", e))?;

        // Clear the context to free resources
        self.module.clear_context(&mut self.ctx);

        // Compile the function
        self.module
            .finalize_definitions()
            .map_err(|e| format!("Failed to finalize: {}", e))?;

        // Get function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // Store function ID
        self.compiled_funcs.insert(name.to_string(), func_id);

        Ok(code_ptr)
    }

    /// Compile a more complex dataflow with multiple operations
    /// Computes: output = (a + b) * (c - d)
    pub fn compile_dataflow_combiner(&mut self, name: &str) -> Result<*const u8, String> {
        // Clear the context for a fresh function
        self.ctx.clear();

        // Define function signature: fn(a: i64, b: i64, c: i64, d: i64) -> i64
        let int_type = types::I64;
        for _ in 0..4 {
            self.ctx.func.signature.params.push(AbiParam::new(int_type));
        }
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(int_type));

        // Declare the function
        let func_id = self
            .module
            .declare_function(name, Linkage::Local, &self.ctx.func.signature)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        {
            // Build the function body
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.func_ctx);

            // Create entry block
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Get parameters
            let params = builder.block_params(entry_block);
            let a = params[0];
            let b = params[1];
            let c = params[2];
            let d = params[3];

            // Compute: (a + b) * (c - d)
            let sum = builder.ins().iadd(a, b);
            let diff = builder.ins().isub(c, d);
            let result = builder.ins().imul(sum, diff);

            // Return the result
            builder.ins().return_(&[result]);

            // Finalize
            builder.seal_all_blocks();
            builder.finalize();
        }

        // Define the function
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define function: {}", e))?;

        // Clear the context to free resources
        self.module.clear_context(&mut self.ctx);

        // Compile the function
        self.module
            .finalize_definitions()
            .map_err(|e| format!("Failed to finalize: {}", e))?;

        // Get function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // Store function ID
        self.compiled_funcs.insert(name.to_string(), func_id);

        Ok(code_ptr)
    }

    /// Execute a compiled arithmetic function
    ///
    /// # Safety
    /// The caller must ensure that `func_ptr` points to valid JIT-compiled code
    /// that was generated by this compiler with the correct signature `fn(i64) -> i64`.
    pub unsafe fn execute_arithmetic(&self, func_ptr: *const u8, input: i64) -> i64 {
        // Cast to function pointer
        let func: fn(i64) -> i64 = mem::transmute(func_ptr);
        func(input)
    }

    /// Execute a compiled dataflow combiner
    ///
    /// # Safety
    /// The caller must ensure that `func_ptr` points to valid JIT-compiled code
    /// that was generated by this compiler with the correct signature `fn(i64, i64, i64, i64) -> i64`.
    pub unsafe fn execute_combiner(
        &self,
        func_ptr: *const u8,
        a: i64,
        b: i64,
        c: i64,
        d: i64,
    ) -> i64 {
        // Cast to function pointer
        let func: fn(i64, i64, i64, i64) -> i64 = mem::transmute(func_ptr);
        func(a, b, c, d)
    }

    /// Compile a DataflowExpr AST to native code
    ///
    /// Takes a DataflowExpr and compiles it to a function: fn(input: i64) -> i64
    /// The "input" named variable in the expression tree maps to the function parameter.
    pub fn compile_dataflow_expr(
        &mut self,
        name: &str,
        expr: &DataflowExpr,
    ) -> Result<*const u8, String> {
        // Clear the context for a fresh function
        self.ctx.clear();

        // Define function signature: fn(input: i64) -> i64
        let int_type = types::I64;
        self.ctx.func.signature.params.push(AbiParam::new(int_type));
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(int_type));

        // Declare the function
        let func_id = self
            .module
            .declare_function(name, Linkage::Local, &self.ctx.func.signature)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        {
            // Build the function body
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.func_ctx);

            // Create entry block
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Get the input parameter (maps to "input" variable in AST)
            let input_param = builder.block_params(entry_block)[0];

            // Recursively compile the expression AST to Cranelift IR
            let result = Self::compile_expr_recursive(&mut builder, expr, input_param, int_type)?;

            // Return the result
            builder.ins().return_(&[result]);

            // Finalize
            builder.seal_all_blocks();
            builder.finalize();
        }

        // Define the function
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define function: {}", e))?;

        // Clear the context to free resources
        self.module.clear_context(&mut self.ctx);

        // Compile the function
        self.module
            .finalize_definitions()
            .map_err(|e| format!("Failed to finalize: {}", e))?;

        // Get function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // Store function ID
        self.compiled_funcs.insert(name.to_string(), func_id);

        Ok(code_ptr)
    }

    /// Recursively compile a DataflowExpr to Cranelift IR values
    fn compile_expr_recursive(
        builder: &mut FunctionBuilder,
        expr: &DataflowExpr,
        input_param: Value,
        int_type: Type,
    ) -> Result<Value, String> {
        match expr {
            DataflowExpr::Const(value) => Ok(builder.ins().iconst(int_type, *value)),

            DataflowExpr::Input(_name) => {
                // All input variables map to the single input parameter
                // In a more complex system, we could have a HashMap of named inputs
                Ok(input_param)
            }

            DataflowExpr::BinOp { op, left, right } => {
                let left_val = Self::compile_expr_recursive(builder, left, input_param, int_type)?;
                let right_val =
                    Self::compile_expr_recursive(builder, right, input_param, int_type)?;

                let result = match op {
                    BinaryOp::Add => builder.ins().iadd(left_val, right_val),
                    BinaryOp::Sub => builder.ins().isub(left_val, right_val),
                    BinaryOp::Mul => builder.ins().imul(left_val, right_val),
                    BinaryOp::Div => builder.ins().sdiv(left_val, right_val),
                    BinaryOp::Mod => builder.ins().srem(left_val, right_val),
                    BinaryOp::And => builder.ins().band(left_val, right_val),
                    BinaryOp::Or => builder.ins().bor(left_val, right_val),
                    BinaryOp::Xor => builder.ins().bxor(left_val, right_val),
                };
                Ok(result)
            }

            DataflowExpr::UnaryOp { op, expr: inner } => {
                let inner_val =
                    Self::compile_expr_recursive(builder, inner, input_param, int_type)?;

                let result = match op {
                    UnaryOp::Neg => builder.ins().ineg(inner_val),
                    UnaryOp::Not => builder.ins().bnot(inner_val),
                    UnaryOp::Abs => {
                        // abs(x) = x >= 0 ? x : -x
                        let zero = builder.ins().iconst(int_type, 0);
                        let is_neg = builder.ins().icmp(IntCC::SignedLessThan, inner_val, zero);
                        let negated = builder.ins().ineg(inner_val);
                        builder.ins().select(is_neg, negated, inner_val)
                    }
                };
                Ok(result)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_dataflow_expr_simple() {
        let mut compiler = JITCompiler::new().expect("Failed to create JIT compiler");

        // Build AST: input * 3 + 7
        let expr = DataflowExpr::BinOp {
            op: BinaryOp::Add,
            left: Box::new(DataflowExpr::BinOp {
                op: BinaryOp::Mul,
                left: Box::new(DataflowExpr::Input("x".into())),
                right: Box::new(DataflowExpr::Const(3)),
            }),
            right: Box::new(DataflowExpr::Const(7)),
        };

        let func_ptr = compiler
            .compile_dataflow_expr("test_mul_add", &expr)
            .expect("Failed to compile expression");

        // Execute: 10 * 3 + 7 = 37
        let result = unsafe {
            let func: fn(i64) -> i64 = mem::transmute(func_ptr);
            func(10)
        };
        assert_eq!(result, 37);

        // Execute: 0 * 3 + 7 = 7
        let result = unsafe {
            let func: fn(i64) -> i64 = mem::transmute(func_ptr);
            func(0)
        };
        assert_eq!(result, 7);
    }

    #[test]
    fn test_compile_dataflow_expr_unary() {
        let mut compiler = JITCompiler::new().expect("Failed to create JIT compiler");

        // Build AST: -input
        let expr = DataflowExpr::UnaryOp {
            op: UnaryOp::Neg,
            expr: Box::new(DataflowExpr::Input("x".into())),
        };

        let func_ptr = compiler
            .compile_dataflow_expr("test_neg", &expr)
            .expect("Failed to compile expression");

        // Execute: -42 = -42
        let result = unsafe {
            let func: fn(i64) -> i64 = mem::transmute(func_ptr);
            func(42)
        };
        assert_eq!(result, -42);
    }

    #[test]
    fn test_compile_dataflow_expr_abs() {
        let mut compiler = JITCompiler::new().expect("Failed to create JIT compiler");

        // Build AST: abs(input)
        let expr = DataflowExpr::UnaryOp {
            op: UnaryOp::Abs,
            expr: Box::new(DataflowExpr::Input("x".into())),
        };

        let func_ptr = compiler
            .compile_dataflow_expr("test_abs", &expr)
            .expect("Failed to compile expression");

        // Execute: abs(-42) = 42
        let result = unsafe {
            let func: fn(i64) -> i64 = mem::transmute(func_ptr);
            func(-42)
        };
        assert_eq!(result, 42);

        // Execute: abs(42) = 42
        let result = unsafe {
            let func: fn(i64) -> i64 = mem::transmute(func_ptr);
            func(42)
        };
        assert_eq!(result, 42);
    }

    #[test]
    fn test_compile_dataflow_expr_complex() {
        let mut compiler = JITCompiler::new().expect("Failed to create JIT compiler");

        // Build AST: (input + 5) * (input - 3)
        let expr = DataflowExpr::BinOp {
            op: BinaryOp::Mul,
            left: Box::new(DataflowExpr::BinOp {
                op: BinaryOp::Add,
                left: Box::new(DataflowExpr::Input("x".into())),
                right: Box::new(DataflowExpr::Const(5)),
            }),
            right: Box::new(DataflowExpr::BinOp {
                op: BinaryOp::Sub,
                left: Box::new(DataflowExpr::Input("x".into())),
                right: Box::new(DataflowExpr::Const(3)),
            }),
        };

        let func_ptr = compiler
            .compile_dataflow_expr("test_complex", &expr)
            .expect("Failed to compile expression");

        // Execute: (10 + 5) * (10 - 3) = 15 * 7 = 105
        let result = unsafe {
            let func: fn(i64) -> i64 = mem::transmute(func_ptr);
            func(10)
        };
        assert_eq!(result, 105);
    }
}
