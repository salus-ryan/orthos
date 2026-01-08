use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use z3::ast::{Ast, Bool, Int};
use z3::{Config, Context, Model, SatResult, Solver};

use crate::ast::*;
use crate::transpiler::MAX_LIST_SIZE;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FluxValue {
    Int(i64),
    Bool(bool),
    String(String),
    List(Vec<FluxValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<HashMap<String, FluxValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsat_core: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SolveResult {
    pub fn sat(model: HashMap<String, FluxValue>) -> Self {
        Self {
            status: "SAT".to_string(),
            model: Some(model),
            unsat_core: None,
            error: None,
        }
    }
    
    pub fn unsat(core: Vec<String>) -> Self {
        Self {
            status: "UNSAT".to_string(),
            model: None,
            unsat_core: Some(core),
            error: None,
        }
    }
    
    pub fn error(msg: String) -> Self {
        Self {
            status: "ERROR".to_string(),
            model: None,
            unsat_core: None,
            error: Some(msg),
        }
    }
    
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    }
}

pub struct OrthosSolver<'ctx> {
    ctx: &'ctx Context,
    solver: Solver<'ctx>,
    int_vars: HashMap<String, Int<'ctx>>,
    bool_vars: HashMap<String, Bool<'ctx>>,
    law_names: Vec<String>,
    boundary_defs: HashMap<String, Boundary>,
}

impl<'ctx> OrthosSolver<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        let solver = Solver::new(ctx);
        Self {
            ctx,
            solver,
            int_vars: HashMap::new(),
            bool_vars: HashMap::new(),
            law_names: Vec::new(),
            boundary_defs: HashMap::new(),
        }
    }
    
    pub fn load_program(&mut self, program: &Program) -> Result<(), String> {
        // First pass: collect boundary definitions
        for boundary in &program.boundaries {
            self.boundary_defs.insert(boundary.name.clone(), boundary.clone());
        }
        
        // Second pass: process all boundaries
        for boundary in &program.boundaries {
            self.process_boundary(boundary, "")?;
        }
        
        Ok(())
    }
    
    fn process_boundary(&mut self, boundary: &Boundary, prefix: &str) -> Result<(), String> {
        let boundary_prefix = if prefix.is_empty() {
            boundary.name.clone()
        } else {
            format!("{}_{}", prefix, boundary.name)
        };
        
        // Declare Flux variables
        for flux in &boundary.flux {
            let var_name = format!("{}_{}", boundary_prefix, flux.name);
            self.declare_variable(&var_name, &flux.typ)?;
            
            // Handle initializers
            if let Some(ref init) = flux.init {
                let constraint = self.build_equality(&var_name, init, &boundary_prefix)?;
                self.solver.assert(&constraint);
            }
        }
        
        // Process Laws
        for law in &boundary.laws {
            let constraint = self.build_bool_expr(&law.constraint, &boundary_prefix)?;
            self.law_names.push(law.name.clone());
            self.solver.assert(&constraint);
        }
        
        // Process Manifest
        if let Some(ref manifest) = boundary.manifest {
            self.process_manifest(manifest, &boundary_prefix)?;
        }
        
        // Process nested boundaries
        for nested in &boundary.nested_boundaries {
            self.process_boundary(nested, &boundary_prefix)?;
        }
        
        Ok(())
    }
    
    fn process_manifest(&mut self, manifest: &ManifestBlock, prefix: &str) -> Result<(), String> {
        for stmt in &manifest.statements {
            match stmt {
                Statement::FluxDecl(decl) => {
                    let var_name = format!("{}_{}", prefix, decl.name);
                    self.declare_variable(&var_name, &decl.typ)?;
                    
                    if let Some(ref init) = decl.init {
                        let constraint = self.build_equality(&var_name, init, prefix)?;
                        self.solver.assert(&constraint);
                    }
                }
                Statement::Assignment { target, value } => {
                    let var_name = format!("{}_{}", prefix, target);
                    // Ensure variable exists
                    if !self.int_vars.contains_key(&var_name) && !self.bool_vars.contains_key(&var_name) {
                        self.declare_variable(&var_name, &Type::Int)?;
                    }
                    let constraint = self.build_equality(&var_name, value, prefix)?;
                    self.solver.assert(&constraint);
                }
            }
        }
        Ok(())
    }
    
    fn declare_variable(&mut self, name: &str, typ: &Type) -> Result<(), String> {
        match typ {
            Type::Int => {
                let var = Int::new_const(self.ctx, name.to_string());
                self.int_vars.insert(name.to_string(), var);
            }
            Type::Bool => {
                let var = Bool::new_const(self.ctx, name.to_string());
                self.bool_vars.insert(name.to_string(), var);
            }
            Type::List(inner) => {
                // For lists, we create individual element variables up to MAX_LIST_SIZE
                // and a length variable
                let len_name = format!("{}_len", name);
                let len_var = Int::new_const(self.ctx, len_name.clone());
                
                // Bound the length
                let zero = Int::from_i64(self.ctx, 0);
                let max = Int::from_i64(self.ctx, MAX_LIST_SIZE as i64);
                self.solver.assert(&len_var.ge(&zero));
                self.solver.assert(&len_var.le(&max));
                
                self.int_vars.insert(len_name, len_var);
                
                // Declare element variables
                for i in 0..MAX_LIST_SIZE {
                    let elem_name = format!("{}_{}", name, i);
                    self.declare_variable(&elem_name, inner)?;
                }
            }
            Type::String => {
                // Strings as bounded byte arrays
                let len_name = format!("{}_len", name);
                let len_var = Int::new_const(self.ctx, len_name.clone());
                let zero = Int::from_i64(self.ctx, 0);
                let max = Int::from_i64(self.ctx, MAX_LIST_SIZE as i64);
                self.solver.assert(&len_var.ge(&zero));
                self.solver.assert(&len_var.le(&max));
                self.int_vars.insert(len_name, len_var);
                
                for i in 0..MAX_LIST_SIZE {
                    let byte_name = format!("{}_{}", name, i);
                    let byte_var = Int::new_const(self.ctx, byte_name.clone());
                    // Bytes are 0-255
                    self.solver.assert(&byte_var.ge(&Int::from_i64(self.ctx, 0)));
                    self.solver.assert(&byte_var.le(&Int::from_i64(self.ctx, 255)));
                    self.int_vars.insert(byte_name, byte_var);
                }
            }
            Type::Custom(boundary_name) => {
                // Instantiate the boundary's flux as sub-variables
                if let Some(boundary) = self.boundary_defs.get(boundary_name).cloned() {
                    for flux in &boundary.flux {
                        let sub_name = format!("{}_{}", name, flux.name);
                        self.declare_variable(&sub_name, &flux.typ)?;
                    }
                }
            }
        }
        Ok(())
    }
    
    fn build_equality(&mut self, var_name: &str, expr: &Expr, prefix: &str) -> Result<Bool<'ctx>, String> {
        // Handle field access on instantiation specially
        if let Expr::FieldAccess(base, field) = expr {
            if let Expr::Instantiate { boundary, args } = base.as_ref() {
                // Create instance and get the field
                let instance_prefix = format!("{}_{}_inst", prefix, boundary);
                self.instantiate_boundary(boundary, args, prefix, &instance_prefix)?;
                
                let field_var_name = format!("{}_{}", instance_prefix, field);
                if let Some(var) = self.int_vars.get(var_name) {
                    if let Some(field_var) = self.int_vars.get(&field_var_name) {
                        return Ok(var._eq(field_var));
                    }
                }
                if let Some(var) = self.bool_vars.get(var_name) {
                    if let Some(field_var) = self.bool_vars.get(&field_var_name) {
                        return Ok(var._eq(field_var));
                    }
                }
                return Err(format!("Cannot find variable {} or {}", var_name, field_var_name));
            }
        }
        
        // Try as int expression
        if let Some(var) = self.int_vars.get(var_name).cloned() {
            let value = self.build_int_expr(expr, prefix)?;
            return Ok(var._eq(&value));
        }
        
        // Try as bool expression
        if let Some(var) = self.bool_vars.get(var_name).cloned() {
            let value = self.build_bool_expr(expr, prefix)?;
            return Ok(var._eq(&value));
        }
        
        Err(format!("Variable {} not found", var_name))
    }
    
    fn instantiate_boundary(
        &mut self,
        boundary_name: &str,
        args: &[(String, Expr)],
        caller_prefix: &str,
        instance_prefix: &str,
    ) -> Result<(), String> {
        // Check if already instantiated
        let marker = format!("{}_instantiated", instance_prefix);
        if self.bool_vars.contains_key(&marker) {
            return Ok(());
        }
        
        // Mark as instantiated
        let marker_var = Bool::new_const(self.ctx, marker.clone());
        self.solver.assert(&marker_var);
        self.bool_vars.insert(marker, marker_var);
        
        let boundary = self.boundary_defs.get(boundary_name)
            .ok_or_else(|| format!("Boundary {} not found", boundary_name))?
            .clone();
        
        // Declare flux variables for this instance
        for flux in &boundary.flux {
            let var_name = format!("{}_{}", instance_prefix, flux.name);
            self.declare_variable(&var_name, &flux.typ)?;
        }
        
        // Apply argument constraints
        for (arg_name, arg_value) in args {
            let var_name = format!("{}_{}", instance_prefix, arg_name);
            let constraint = self.build_equality(&var_name, arg_value, caller_prefix)?;
            self.solver.assert(&constraint);
        }
        
        // Apply laws from the boundary
        for law in &boundary.laws {
            let constraint = self.build_bool_expr(&law.constraint, instance_prefix)?;
            self.law_names.push(format!("{}.{}", boundary_name, law.name));
            self.solver.assert(&constraint);
        }
        
        Ok(())
    }
    
    fn build_int_expr(&mut self, expr: &Expr, prefix: &str) -> Result<Int<'ctx>, String> {
        match expr {
            Expr::IntLit(n) => Ok(Int::from_i64(self.ctx, *n)),
            Expr::Var(name) => {
                let var_name = format!("{}_{}", prefix, name);
                self.int_vars.get(&var_name)
                    .cloned()
                    .ok_or_else(|| format!("Int variable {} not found", var_name))
            }
            Expr::FieldAccess(base, field) => {
                if let Expr::Var(base_name) = base.as_ref() {
                    let var_name = format!("{}_{}_{}", prefix, base_name, field);
                    self.int_vars.get(&var_name)
                        .cloned()
                        .ok_or_else(|| format!("Int variable {} not found", var_name))
                } else if let Expr::Instantiate { boundary, args } = base.as_ref() {
                    let instance_prefix = format!("{}_{}_inst", prefix, boundary);
                    self.instantiate_boundary(boundary, args, prefix, &instance_prefix)?;
                    let var_name = format!("{}_{}", instance_prefix, field);
                    self.int_vars.get(&var_name)
                        .cloned()
                        .ok_or_else(|| format!("Int variable {} not found", var_name))
                } else {
                    Err("Complex field access not supported".to_string())
                }
            }
            Expr::IndexAccess(array, index) => {
                let _idx = self.build_int_expr(index, prefix)?;
                // For now, we need to handle this with concrete indices
                // This is a limitation - we'd need array theory for full support
                if let Expr::IntLit(i) = index.as_ref() {
                    if let Expr::Var(name) = array.as_ref() {
                        let elem_name = format!("{}_{}_{}", prefix, name, i);
                        return self.int_vars.get(&elem_name)
                            .cloned()
                            .ok_or_else(|| format!("Array element {} not found", elem_name));
                    }
                }
                Err("Dynamic array indexing requires concrete indices in v0.1".to_string())
            }
            Expr::Add(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left + right)
            }
            Expr::Sub(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left - right)
            }
            Expr::Mul(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left * right)
            }
            Expr::Div(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left / right)
            }
            Expr::Mod(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left % right)
            }
            Expr::ListLen(list) => {
                if let Expr::Var(name) = list.as_ref() {
                    let len_name = format!("{}_{}_len", prefix, name);
                    self.int_vars.get(&len_name)
                        .cloned()
                        .ok_or_else(|| format!("List length {} not found", len_name))
                } else {
                    Err("List length on complex expressions not supported".to_string())
                }
            }
            Expr::ListFirst(list) => {
                if let Expr::Var(name) = list.as_ref() {
                    let elem_name = format!("{}_{}_0", prefix, name);
                    self.int_vars.get(&elem_name)
                        .cloned()
                        .ok_or_else(|| format!("List first element {} not found", elem_name))
                } else {
                    Err("List first on complex expressions not supported".to_string())
                }
            }
            _ => Err(format!("Expression {:?} cannot be converted to Int", expr)),
        }
    }
    
    fn build_bool_expr(&mut self, expr: &Expr, prefix: &str) -> Result<Bool<'ctx>, String> {
        match expr {
            Expr::BoolLit(b) => Ok(Bool::from_bool(self.ctx, *b)),
            Expr::Var(name) => {
                let var_name = format!("{}_{}", prefix, name);
                self.bool_vars.get(&var_name)
                    .cloned()
                    .ok_or_else(|| format!("Bool variable {} not found", var_name))
            }
            Expr::Eq(l, r) => {
                // Try int comparison first
                if let (Ok(left), Ok(right)) = (
                    self.build_int_expr(l, prefix),
                    self.build_int_expr(r, prefix)
                ) {
                    return Ok(left._eq(&right));
                }
                // Try bool comparison
                let left = self.build_bool_expr(l, prefix)?;
                let right = self.build_bool_expr(r, prefix)?;
                Ok(left._eq(&right))
            }
            Expr::Ne(l, r) => {
                let eq = self.build_bool_expr(&Expr::Eq(l.clone(), r.clone()), prefix)?;
                Ok(eq.not())
            }
            Expr::Lt(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left.lt(&right))
            }
            Expr::Le(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left.le(&right))
            }
            Expr::Gt(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left.gt(&right))
            }
            Expr::Ge(l, r) => {
                let left = self.build_int_expr(l, prefix)?;
                let right = self.build_int_expr(r, prefix)?;
                Ok(left.ge(&right))
            }
            Expr::And(l, r) => {
                let left = self.build_bool_expr(l, prefix)?;
                let right = self.build_bool_expr(r, prefix)?;
                Ok(Bool::and(self.ctx, &[&left, &right]))
            }
            Expr::Or(l, r) => {
                let left = self.build_bool_expr(l, prefix)?;
                let right = self.build_bool_expr(r, prefix)?;
                Ok(Bool::or(self.ctx, &[&left, &right]))
            }
            Expr::Not(e) => {
                let inner = self.build_bool_expr(e, prefix)?;
                Ok(inner.not())
            }
            Expr::Implies(l, r) => {
                let left = self.build_bool_expr(l, prefix)?;
                let right = self.build_bool_expr(r, prefix)?;
                Ok(left.implies(&right))
            }
            _ => Err(format!("Expression {:?} cannot be converted to Bool", expr)),
        }
    }
    
    pub fn solve(&self) -> SolveResult {
        match self.solver.check() {
            SatResult::Sat => {
                let model = self.solver.get_model().unwrap();
                let values = self.extract_model(&model);
                SolveResult::sat(values)
            }
            SatResult::Unsat => {
                SolveResult::unsat(self.law_names.clone())
            }
            SatResult::Unknown => {
                SolveResult::error("Solver returned UNKNOWN".to_string())
            }
        }
    }
    
    fn extract_model(&self, model: &Model) -> HashMap<String, FluxValue> {
        let mut values = HashMap::new();
        
        // Extract int variables
        for (name, var) in &self.int_vars {
            // Skip internal variables
            if name.ends_with("_instantiated") || name.contains("_len") {
                continue;
            }
            
            if let Some(val) = model.eval(var, true) {
                if let Some(i) = val.as_i64() {
                    values.insert(name.clone(), FluxValue::Int(i));
                }
            }
        }
        
        // Extract bool variables
        for (name, var) in &self.bool_vars {
            if name.ends_with("_instantiated") {
                continue;
            }
            
            if let Some(val) = model.eval(var, true) {
                if let Some(b) = val.as_bool() {
                    values.insert(name.clone(), FluxValue::Bool(b));
                }
            }
        }
        
        values
    }
}

pub fn solve_program(program: &Program) -> SolveResult {
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let mut solver = OrthosSolver::new(&ctx);
    
    if let Err(e) = solver.load_program(program) {
        return SolveResult::error(e);
    }
    
    solver.solve()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    
    #[test]
    fn test_simple_solve() {
        let source = r#"
            Boundary Test {
                Flux x : Int
                Law Constraint : x == 42
            }
        "#;
        
        let program = parse(source).unwrap();
        let result = solve_program(&program);
        
        assert_eq!(result.status, "SAT");
        let model = result.model.unwrap();
        assert_eq!(model.get("Test_x"), Some(&FluxValue::Int(42)));
    }
    
    #[test]
    fn test_unsat() {
        let source = r#"
            Boundary Test {
                Flux x : Int
                Law A : x > 0
                Law B : x < 0
            }
        "#;
        
        let program = parse(source).unwrap();
        let result = solve_program(&program);
        
        assert_eq!(result.status, "UNSAT");
    }
}
