use crate::ast::*;
use std::collections::HashMap;

pub const MAX_LIST_SIZE: usize = 64;
pub const DEFAULT_HORIZON: usize = 10;

#[derive(Debug, Clone)]
pub struct TranspileContext {
    pub variables: HashMap<String, (String, Type)>,
    pub boundary_defs: HashMap<String, Boundary>,
    pub assertions: Vec<String>,
    pub declarations: Vec<String>,
    pub counter: usize,
}

impl TranspileContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            boundary_defs: HashMap::new(),
            assertions: Vec::new(),
            declarations: Vec::new(),
            counter: 0,
        }
    }
    
    pub fn fresh_var(&mut self, prefix: &str) -> String {
        let name = format!("{}_{}", prefix, self.counter);
        self.counter += 1;
        name
    }
    
    pub fn declare_var(&mut self, name: &str, typ: &Type) {
        let smt_type = type_to_smt(typ);
        self.declarations.push(format!("(declare-const {} {})", name, smt_type));
        self.variables.insert(name.to_string(), (name.to_string(), typ.clone()));
    }
    
    pub fn add_assertion(&mut self, assertion: String) {
        self.assertions.push(format!("(assert {})", assertion));
    }
}

impl Default for TranspileContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn type_to_smt(typ: &Type) -> String {
    match typ {
        Type::Int => "Int".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "(Array Int Int)".to_string(), // Byte array representation
        Type::List(inner) => format!("(Array Int {})", type_to_smt(inner)),
        Type::Custom(name) => name.clone(), // Will be a declared sort
    }
}

pub fn expr_to_smt(expr: &Expr, ctx: &TranspileContext) -> String {
    match expr {
        Expr::IntLit(n) => {
            if *n < 0 {
                format!("(- {})", -n)
            } else {
                n.to_string()
            }
        }
        Expr::BoolLit(b) => b.to_string(),
        Expr::StringLit(s) => {
            // Convert string to array representation
            format!("\"{}\"", s)
        }
        Expr::Var(name) => {
            // Check if it's a qualified name (contains underscore from boundary instantiation)
            name.clone()
        }
        Expr::FieldAccess(base, field) => {
            let base_smt = expr_to_smt(base, ctx);
            format!("{}_{}", base_smt, field)
        }
        Expr::IndexAccess(array, index) => {
            let array_smt = expr_to_smt(array, ctx);
            let index_smt = expr_to_smt(index, ctx);
            format!("(select {} {})", array_smt, index_smt)
        }
        Expr::Add(l, r) => format!("(+ {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Sub(l, r) => format!("(- {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Mul(l, r) => format!("(* {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Div(l, r) => format!("(div {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Mod(l, r) => format!("(mod {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Eq(l, r) => format!("(= {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Ne(l, r) => format!("(not (= {} {}))", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Lt(l, r) => format!("(< {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Le(l, r) => format!("(<= {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Gt(l, r) => format!("(> {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Ge(l, r) => format!("(>= {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::And(l, r) => format!("(and {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Or(l, r) => format!("(or {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Not(e) => format!("(not {})", expr_to_smt(e, ctx)),
        Expr::Implies(l, r) => format!("(=> {} {})", expr_to_smt(l, ctx), expr_to_smt(r, ctx)),
        Expr::Instantiate { boundary, args } => {
            // This creates a reference to the instantiated boundary's result
            // The actual instantiation is handled during transpilation
            format!("{}_{}", boundary, args.first().map(|(n, _)| n.as_str()).unwrap_or("inst"))
        }
        Expr::ListLit(elements) => {
            // Create an array constant and store elements
            let mut result = "((as const (Array Int Int)) 0)".to_string();
            for (i, elem) in elements.iter().enumerate() {
                result = format!("(store {} {} {})", result, i, expr_to_smt(elem, ctx));
            }
            result
        }
        Expr::ListFirst(list) => {
            format!("(select {} 0)", expr_to_smt(list, ctx))
        }
        Expr::ListLast(list) => {
            // For bounded lists, we need to track length separately
            // For now, assume last is at index (len - 1)
            format!("(select {} (- {}_len 1))", expr_to_smt(list, ctx), expr_to_smt(list, ctx))
        }
        Expr::ListLen(list) => {
            format!("{}_len", expr_to_smt(list, ctx))
        }
    }
}

pub struct Transpiler {
    pub ctx: TranspileContext,
}

impl Transpiler {
    pub fn new() -> Self {
        Self {
            ctx: TranspileContext::new(),
        }
    }
    
    pub fn transpile(&mut self, program: &Program) -> Result<String, String> {
        // First pass: collect all boundary definitions
        for boundary in &program.boundaries {
            self.ctx.boundary_defs.insert(boundary.name.clone(), boundary.clone());
        }
        
        // Second pass: process boundaries
        for boundary in &program.boundaries {
            self.transpile_boundary(boundary, "")?;
        }
        
        // Build final SMT-LIB2 script
        let mut script = String::new();
        script.push_str("; ORTHOS Kernel v0.1 - SMT-LIB2 Output\n");
        script.push_str("(set-logic ALL)\n\n");
        
        // Add declarations
        script.push_str("; Variable Declarations\n");
        for decl in &self.ctx.declarations {
            script.push_str(decl);
            script.push('\n');
        }
        
        script.push_str("\n; Constraints (Laws)\n");
        for assertion in &self.ctx.assertions {
            script.push_str(assertion);
            script.push('\n');
        }
        
        script.push_str("\n; Solve\n");
        script.push_str("(check-sat)\n");
        script.push_str("(get-model)\n");
        
        Ok(script)
    }
    
    fn transpile_boundary(&mut self, boundary: &Boundary, prefix: &str) -> Result<(), String> {
        let boundary_prefix = if prefix.is_empty() {
            boundary.name.clone()
        } else {
            format!("{}_{}", prefix, boundary.name)
        };
        
        // Declare Flux variables
        for flux in &boundary.flux {
            let var_name = format!("{}_{}", boundary_prefix, flux.name);
            self.ctx.declare_var(&var_name, &flux.typ);
            
            // If there's an initializer, add constraint
            if let Some(ref init) = flux.init {
                let init_smt = self.transpile_expr_with_prefix(init, &boundary_prefix);
                self.ctx.add_assertion(format!("(= {} {})", var_name, init_smt));
            }
            
            // For List types, also declare length variable
            if let Type::List(_) = flux.typ {
                let len_var = format!("{}_len", var_name);
                self.ctx.declarations.push(format!("(declare-const {} Int)", len_var));
                // Bound the length
                self.ctx.add_assertion(format!("(>= {} 0)", len_var));
                self.ctx.add_assertion(format!("(<= {} {})", len_var, MAX_LIST_SIZE));
            }
        }
        
        // Transpile Laws
        for law in &boundary.laws {
            let constraint_smt = self.transpile_expr_with_prefix(&law.constraint, &boundary_prefix);
            self.ctx.assertions.push(format!("; Law: {}", law.name));
            self.ctx.add_assertion(constraint_smt);
        }
        
        // Process Manifest if present
        if let Some(ref manifest) = boundary.manifest {
            self.transpile_manifest(manifest, &boundary_prefix)?;
        }
        
        // Process nested boundaries
        for nested in &boundary.nested_boundaries {
            self.transpile_boundary(nested, &boundary_prefix)?;
        }
        
        Ok(())
    }
    
    fn transpile_manifest(&mut self, manifest: &ManifestBlock, prefix: &str) -> Result<(), String> {
        for stmt in &manifest.statements {
            match stmt {
                Statement::FluxDecl(decl) => {
                    let var_name = format!("{}_{}", prefix, decl.name);
                    self.ctx.declare_var(&var_name, &decl.typ);
                    
                    if let Some(ref init) = decl.init {
                        let init_smt = self.transpile_expr_with_prefix(init, prefix);
                        self.ctx.add_assertion(format!("(= {} {})", var_name, init_smt));
                    }
                }
                Statement::Assignment { target, value } => {
                    let var_name = format!("{}_{}", prefix, target);
                    let value_smt = self.transpile_expr_with_prefix(value, prefix);
                    self.ctx.add_assertion(format!("(= {} {})", var_name, value_smt));
                }
            }
        }
        Ok(())
    }
    
    fn transpile_expr_with_prefix(&mut self, expr: &Expr, prefix: &str) -> String {
        match expr {
            Expr::Var(name) => format!("{}_{}", prefix, name),
            Expr::FieldAccess(base, field) => {
                // Handle boundary instantiation field access
                if let Expr::Instantiate { boundary, args } = base.as_ref() {
                    // Create a unique instance of the boundary
                    let instance_prefix = format!("{}_{}_inst", prefix, boundary);
                    
                    // Check if we've already instantiated this boundary
                    if !self.ctx.variables.contains_key(&instance_prefix) {
                        if let Some(boundary_def) = self.ctx.boundary_defs.get(boundary).cloned() {
                            // Declare variables for this instance
                            for flux in &boundary_def.flux {
                                let var_name = format!("{}_{}", instance_prefix, flux.name);
                                self.ctx.declare_var(&var_name, &flux.typ);
                            }
                            
                            // Apply argument constraints
                            for (arg_name, arg_value) in args {
                                let var_name = format!("{}_{}", instance_prefix, arg_name);
                                let value_smt = self.transpile_expr_with_prefix(arg_value, prefix);
                                self.ctx.add_assertion(format!("(= {} {})", var_name, value_smt));
                            }
                            
                            // Apply laws from the boundary
                            for law in &boundary_def.laws {
                                let constraint_smt = self.transpile_expr_with_prefix(&law.constraint, &instance_prefix);
                                self.ctx.assertions.push(format!("; Law: {} (from {})", law.name, boundary));
                                self.ctx.add_assertion(constraint_smt);
                            }
                            
                            // Mark as instantiated
                            self.ctx.variables.insert(instance_prefix.clone(), (instance_prefix.clone(), Type::Custom(boundary.clone())));
                        }
                    }
                    
                    format!("{}_{}", instance_prefix, field)
                } else {
                    let base_smt = self.transpile_expr_with_prefix(base, prefix);
                    format!("{}_{}", base_smt, field)
                }
            }
            Expr::IntLit(n) => {
                if *n < 0 {
                    format!("(- {})", -n)
                } else {
                    n.to_string()
                }
            }
            Expr::BoolLit(b) => b.to_string(),
            Expr::StringLit(s) => format!("\"{}\"", s),
            Expr::IndexAccess(array, index) => {
                let array_smt = self.transpile_expr_with_prefix(array, prefix);
                let index_smt = self.transpile_expr_with_prefix(index, prefix);
                format!("(select {} {})", array_smt, index_smt)
            }
            Expr::Add(l, r) => format!("(+ {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Sub(l, r) => format!("(- {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Mul(l, r) => format!("(* {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Div(l, r) => format!("(div {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Mod(l, r) => format!("(mod {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Eq(l, r) => format!("(= {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Ne(l, r) => format!("(not (= {} {}))", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Lt(l, r) => format!("(< {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Le(l, r) => format!("(<= {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Gt(l, r) => format!("(> {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Ge(l, r) => format!("(>= {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::And(l, r) => format!("(and {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Or(l, r) => format!("(or {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Not(e) => format!("(not {})", 
                self.transpile_expr_with_prefix(e, prefix)),
            Expr::Implies(l, r) => format!("(=> {} {})", 
                self.transpile_expr_with_prefix(l, prefix),
                self.transpile_expr_with_prefix(r, prefix)),
            Expr::Instantiate { boundary, args: _ } => {
                // Return reference to the instance
                format!("{}_{}_inst", prefix, boundary)
            }
            Expr::ListLit(elements) => {
                let inner_type = "Int"; // Default for now
                let mut result = format!("((as const (Array Int {})) 0)", inner_type);
                for (i, elem) in elements.iter().enumerate() {
                    result = format!("(store {} {} {})", result, i, 
                        self.transpile_expr_with_prefix(elem, prefix));
                }
                result
            }
            Expr::ListFirst(list) => {
                format!("(select {} 0)", self.transpile_expr_with_prefix(list, prefix))
            }
            Expr::ListLast(list) => {
                let list_smt = self.transpile_expr_with_prefix(list, prefix);
                format!("(select {} (- {}_len 1))", list_smt, list_smt)
            }
            Expr::ListLen(list) => {
                format!("{}_len", self.transpile_expr_with_prefix(list, prefix))
            }
        }
    }
}

impl Default for Transpiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    
    #[test]
    fn test_simple_transpile() {
        let source = r#"
            Boundary Test {
                Flux x : Int
                Law Positive : x > 0
            }
        "#;
        
        let program = parse(source).unwrap();
        let mut transpiler = Transpiler::new();
        let smt = transpiler.transpile(&program).unwrap();
        
        assert!(smt.contains("(declare-const Test_x Int)"));
        assert!(smt.contains("(assert (> Test_x 0))"));
    }
}
