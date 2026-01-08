use std::collections::HashMap;
use z3::{ast::{Ast, Bool, Int}, Config, Context, Optimize, SatResult};

use orthos_kernel::{parse, Boundary, Expr, Program, Type, MAX_LIST_SIZE};

#[derive(Debug)]
pub struct InitError {
    pub message: String,
    pub conflicts: Vec<String>,
}

#[derive(Debug)]
pub struct ConstrainError {
    pub conflicts: Vec<String>,
}

#[allow(dead_code)]
pub struct GoalInfo {
    pub name: String,
    pub weight: u32,
    pub satisfied: bool,
}

#[derive(Debug)]
pub struct OptimizeResult {
    pub cost: u64,
}

#[derive(Debug)]
pub struct QueryResult {
    pub values: HashMap<String, serde_json::Value>,
    pub violated_goals: Vec<String>,
}

pub struct DaemonState {
    ctx: &'static Context,
    optimizer: Optimize<'static>,
    int_vars: HashMap<String, Int<'static>>,
    bool_vars: HashMap<String, Bool<'static>>,
    law_names: Vec<String>,
    goal_names: Vec<(String, u32)>,  // (name, weight)
    flux_list: Vec<String>,
    boundary_defs: HashMap<String, Boundary>,
    checkpoint_depth: usize,
}

impl DaemonState {
    pub fn new(source: &str) -> Result<Self, InitError> {
        // Parse the source
        let program = parse(source).map_err(|e| InitError {
            message: format!("Parse error: {}", e),
            conflicts: vec![],
        })?;
        
        // Create Z3 context - we use Box::leak to get 'static lifetime
        // This is safe because DaemonState owns the context for its entire lifetime
        // and the daemon runs until shutdown
        let cfg = Config::new();
        let ctx: &'static Context = Box::leak(Box::new(Context::new(&cfg)));
        let optimizer = Optimize::new(ctx);
        
        let mut state = DaemonState {
            ctx,
            optimizer,
            int_vars: HashMap::new(),
            bool_vars: HashMap::new(),
            law_names: Vec::new(),
            goal_names: Vec::new(),
            flux_list: Vec::new(),
            boundary_defs: HashMap::new(),
            checkpoint_depth: 0,
        };
        
        // Load the program
        state.load_program(&program)?;
        
        // Verify initial satisfiability (hard constraints must be satisfiable)
        match state.optimizer.check(&[]) {
            SatResult::Sat => Ok(state),
            SatResult::Unsat => Err(InitError {
                message: "Ontological Error: Base laws are contradictory".to_string(),
                conflicts: state.law_names.clone(),
            }),
            SatResult::Unknown => Err(InitError {
                message: "Solver returned UNKNOWN".to_string(),
                conflicts: vec![],
            }),
        }
    }
    
    fn load_program(&mut self, program: &Program) -> Result<(), InitError> {
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
    
    fn process_boundary(&mut self, boundary: &Boundary, prefix: &str) -> Result<(), InitError> {
        let boundary_prefix = if prefix.is_empty() {
            boundary.name.clone()
        } else {
            format!("{}_{}", prefix, boundary.name)
        };
        
        // Declare Flux variables
        for flux in &boundary.flux {
            let var_name = format!("{}_{}", boundary_prefix, flux.name);
            self.declare_variable(&var_name, &flux.typ)?;
            
            // Track flux for query API (using dot notation)
            let public_name = format!("{}.{}", boundary_prefix.replace('_', "."), flux.name);
            self.flux_list.push(public_name);
            
            // Handle initializers
            if let Some(ref init) = flux.init {
                let constraint = self.build_equality(&var_name, init, &boundary_prefix)?;
                self.optimizer.assert(&constraint);
            }
        }
        
        // Process Laws (hard constraints)
        for law in &boundary.laws {
            let constraint = self.build_bool_expr(&law.constraint, &boundary_prefix)?;
            self.law_names.push(format!("{}.{}", boundary_prefix, law.name));
            self.optimizer.assert(&constraint);
        }
        
        // Process Goals (soft constraints)
        for goal in &boundary.goals {
            let constraint = self.build_bool_expr(&goal.constraint, &boundary_prefix)?;
            let goal_name = format!("{}.{}", boundary_prefix, goal.name);
            self.goal_names.push((goal_name, goal.weight));
            self.optimizer.assert_soft(&constraint, goal.weight, None);
        }
        
        // Process nested boundaries
        for nested in &boundary.nested_boundaries {
            self.process_boundary(nested, &boundary_prefix)?;
        }
        
        Ok(())
    }
    
    fn declare_variable(&mut self, name: &str, typ: &Type) -> Result<(), InitError> {
        let ctx = self.ctx;
        match typ {
            Type::Int => {
                let var = Int::new_const(ctx, name.to_string());
                self.int_vars.insert(name.to_string(), var);
            }
            Type::Bool => {
                let var = Bool::new_const(ctx, name.to_string());
                self.bool_vars.insert(name.to_string(), var);
            }
            Type::List(inner) => {
                let len_name = format!("{}_len", name);
                let len_var = Int::new_const(ctx, len_name.clone());
                
                let zero = Int::from_i64(ctx, 0);
                let max = Int::from_i64(ctx, MAX_LIST_SIZE as i64);
                self.optimizer.assert(&len_var.ge(&zero));
                self.optimizer.assert(&len_var.le(&max));
                
                self.int_vars.insert(len_name, len_var);
                
                for i in 0..MAX_LIST_SIZE {
                    let elem_name = format!("{}_{}", name, i);
                    self.declare_variable(&elem_name, inner)?;
                }
            }
            Type::String => {
                let len_name = format!("{}_len", name);
                let len_var = Int::new_const(ctx, len_name.clone());
                let zero = Int::from_i64(ctx, 0);
                let max = Int::from_i64(ctx, MAX_LIST_SIZE as i64);
                self.optimizer.assert(&len_var.ge(&zero));
                self.optimizer.assert(&len_var.le(&max));
                self.int_vars.insert(len_name, len_var);
                
                for i in 0..MAX_LIST_SIZE {
                    let byte_name = format!("{}_{}", name, i);
                    let byte_var = Int::new_const(ctx, byte_name.clone());
                    self.optimizer.assert(&byte_var.ge(&Int::from_i64(ctx, 0)));
                    self.optimizer.assert(&byte_var.le(&Int::from_i64(ctx, 255)));
                    self.int_vars.insert(byte_name, byte_var);
                }
            }
            Type::Custom(boundary_name) => {
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
    
    fn build_equality(&mut self, var_name: &str, expr: &Expr, prefix: &str) -> Result<Bool<'static>, InitError> {
        if let Some(var) = self.int_vars.get(var_name).cloned() {
            let value = self.build_int_expr(expr, prefix)?;
            return Ok(var._eq(&value));
        }
        
        if let Some(var) = self.bool_vars.get(var_name).cloned() {
            let value = self.build_bool_expr(expr, prefix)?;
            return Ok(var._eq(&value));
        }
        
        Err(InitError {
            message: format!("Variable {} not found", var_name),
            conflicts: vec![],
        })
    }
    
    fn build_int_expr(&mut self, expr: &Expr, prefix: &str) -> Result<Int<'static>, InitError> {
        let ctx = self.ctx;
        match expr {
            Expr::IntLit(n) => Ok(Int::from_i64(ctx, *n)),
            Expr::Var(name) => {
                let var_name = format!("{}_{}", prefix, name);
                self.int_vars.get(&var_name)
                    .cloned()
                    .ok_or_else(|| InitError {
                        message: format!("Int variable {} not found", var_name),
                        conflicts: vec![],
                    })
            }
            Expr::FieldAccess(base, field) => {
                if let Expr::Var(base_name) = base.as_ref() {
                    let var_name = format!("{}_{}_{}", prefix, base_name, field);
                    self.int_vars.get(&var_name)
                        .cloned()
                        .ok_or_else(|| InitError {
                            message: format!("Int variable {} not found", var_name),
                            conflicts: vec![],
                        })
                } else {
                    Err(InitError {
                        message: "Complex field access not supported".to_string(),
                        conflicts: vec![],
                    })
                }
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
            _ => Err(InitError {
                message: format!("Expression {:?} cannot be converted to Int", expr),
                conflicts: vec![],
            }),
        }
    }
    
    fn build_bool_expr(&mut self, expr: &Expr, prefix: &str) -> Result<Bool<'static>, InitError> {
        let ctx = self.ctx;
        match expr {
            Expr::BoolLit(b) => Ok(Bool::from_bool(ctx, *b)),
            Expr::Var(name) => {
                let var_name = format!("{}_{}", prefix, name);
                self.bool_vars.get(&var_name)
                    .cloned()
                    .ok_or_else(|| InitError {
                        message: format!("Bool variable {} not found", var_name),
                        conflicts: vec![],
                    })
            }
            Expr::Eq(l, r) => {
                if let (Ok(left), Ok(right)) = (
                    self.build_int_expr(l, prefix),
                    self.build_int_expr(r, prefix)
                ) {
                    return Ok(left._eq(&right));
                }
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
                Ok(Bool::and(ctx, &[&left, &right]))
            }
            Expr::Or(l, r) => {
                let left = self.build_bool_expr(l, prefix)?;
                let right = self.build_bool_expr(r, prefix)?;
                Ok(Bool::or(ctx, &[&left, &right]))
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
            _ => Err(InitError {
                message: format!("Expression {:?} cannot be converted to Bool", expr),
                conflicts: vec![],
            }),
        }
    }
    
    pub fn get_flux_list(&self) -> Vec<String> {
        self.flux_list.clone()
    }
    
    fn build_dynamic_int_expr(&self, expr: &Expr) -> Result<Int<'static>, InitError> {
        let ctx = self.ctx;
        match expr {
            Expr::IntLit(n) => Ok(Int::from_i64(ctx, *n)),
            Expr::Var(name) => {
                // Variable name is already fully qualified (e.g., "Main_x")
                self.int_vars.get(name)
                    .cloned()
                    .ok_or_else(|| InitError {
                        message: format!("Int variable {} not found", name),
                        conflicts: vec![],
                    })
            }
            Expr::Add(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left + right)
            }
            Expr::Sub(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left - right)
            }
            Expr::Mul(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left * right)
            }
            Expr::Div(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left / right)
            }
            Expr::Mod(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left % right)
            }
            _ => Err(InitError {
                message: format!("Expression {:?} cannot be converted to Int", expr),
                conflicts: vec![],
            }),
        }
    }
    
    fn build_dynamic_bool_expr(&self, expr: &Expr) -> Result<Bool<'static>, InitError> {
        let ctx = self.ctx;
        match expr {
            Expr::BoolLit(b) => Ok(Bool::from_bool(ctx, *b)),
            Expr::Var(name) => {
                self.bool_vars.get(name)
                    .cloned()
                    .ok_or_else(|| InitError {
                        message: format!("Bool variable {} not found", name),
                        conflicts: vec![],
                    })
            }
            Expr::Eq(l, r) => {
                if let (Ok(left), Ok(right)) = (
                    self.build_dynamic_int_expr(l),
                    self.build_dynamic_int_expr(r)
                ) {
                    return Ok(left._eq(&right));
                }
                let left = self.build_dynamic_bool_expr(l)?;
                let right = self.build_dynamic_bool_expr(r)?;
                Ok(left._eq(&right))
            }
            Expr::Ne(l, r) => {
                let eq = self.build_dynamic_bool_expr(&Expr::Eq(l.clone(), r.clone()))?;
                Ok(eq.not())
            }
            Expr::Lt(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left.lt(&right))
            }
            Expr::Le(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left.le(&right))
            }
            Expr::Gt(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left.gt(&right))
            }
            Expr::Ge(l, r) => {
                let left = self.build_dynamic_int_expr(l)?;
                let right = self.build_dynamic_int_expr(r)?;
                Ok(left.ge(&right))
            }
            Expr::And(l, r) => {
                let left = self.build_dynamic_bool_expr(l)?;
                let right = self.build_dynamic_bool_expr(r)?;
                Ok(Bool::and(ctx, &[&left, &right]))
            }
            Expr::Or(l, r) => {
                let left = self.build_dynamic_bool_expr(l)?;
                let right = self.build_dynamic_bool_expr(r)?;
                Ok(Bool::or(ctx, &[&left, &right]))
            }
            Expr::Not(e) => {
                let inner = self.build_dynamic_bool_expr(e)?;
                Ok(inner.not())
            }
            Expr::Implies(l, r) => {
                let left = self.build_dynamic_bool_expr(l)?;
                let right = self.build_dynamic_bool_expr(r)?;
                Ok(left.implies(&right))
            }
            _ => Err(InitError {
                message: format!("Expression {:?} cannot be converted to Bool", expr),
                conflicts: vec![],
            }),
        }
    }
    
    pub fn checkpoint(&mut self) {
        self.optimizer.push();
        self.checkpoint_depth += 1;
    }
    
    pub fn restore(&mut self) -> Result<(), String> {
        if self.checkpoint_depth == 0 {
            return Err("No checkpoint to restore".to_string());
        }
        self.optimizer.pop();
        self.checkpoint_depth -= 1;
        Ok(())
    }
    
    pub fn constrain(&mut self, laws: &[String]) -> Result<OptimizeResult, ConstrainError> {
        // Push before asserting (micro-checkpoint for auto-rollback)
        self.optimizer.push();
        
        // Parse and assert each law
        // Laws come in format "Law: Boundary.var > 10" or "Boundary.var > 10"
        for law_str in laws {
            let expr_str = law_str.strip_prefix("Law:").unwrap_or(law_str).trim();
            
            // Convert dot notation to underscore for internal variable names
            // e.g., "Main.x > 10" -> "Main_x > 10"
            let internal_expr = expr_str.replace('.', "_");
            
            // Wrap it in a minimal boundary to use existing parser
            let wrapper = format!("Boundary _Temp {{ Law _Dynamic : {} }}", internal_expr);
            
            match parse(&wrapper) {
                Ok(program) => {
                    if let Some(boundary) = program.boundaries.first() {
                        if let Some(law) = boundary.laws.first() {
                            // Build with empty prefix since variable names are already qualified
                            match self.build_dynamic_bool_expr(&law.constraint) {
                                Ok(constraint) => {
                                    self.optimizer.assert(&constraint);
                                    self.law_names.push(format!("Dynamic: {}", expr_str));
                                }
                                Err(e) => {
                                    self.optimizer.pop();
                                    return Err(ConstrainError {
                                        conflicts: vec![format!("Failed to build constraint: {}", e.message)],
                                    });
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    self.optimizer.pop();
                    return Err(ConstrainError {
                        conflicts: vec![format!("Parse error: {}", e)],
                    });
                }
            }
        }
        
        // Check satisfiability with optimization
        match self.optimizer.check(&[]) {
            SatResult::Sat => {
                // Keep the constraints (don't pop)
                // Calculate violated goals cost
                let cost = self.calculate_violated_goals_cost();
                Ok(OptimizeResult { cost })
            }
            SatResult::Unsat => {
                // Auto-rollback
                self.optimizer.pop();
                Err(ConstrainError {
                    conflicts: self.law_names.clone(),
                })
            }
            SatResult::Unknown => {
                self.optimizer.pop();
                Err(ConstrainError {
                    conflicts: vec!["Solver returned UNKNOWN".to_string()],
                })
            }
        }
    }
    
    fn calculate_violated_goals_cost(&self) -> u64 {
        // Z3 Optimize tracks soft constraint violations internally
        // We return the total weight of violated soft constraints
        // For now, we return 0 as the model satisfies as many goals as possible
        // The actual cost would require inspecting the optimization objectives
        0
    }
    
    pub fn query(&self, flux_names: &[String]) -> Result<QueryResult, String> {
        // First check if we have a model
        match self.optimizer.check(&[]) {
            SatResult::Sat => {}
            SatResult::Unsat => return Err("Universe is in UNSAT state".to_string()),
            SatResult::Unknown => return Err("Solver returned UNKNOWN".to_string()),
        }
        
        let model = self.optimizer.get_model()
            .ok_or_else(|| "Failed to get model".to_string())?;
        
        let mut values = HashMap::new();
        
        for name in flux_names {
            // Convert dot notation to underscore (Main.x -> Main_x)
            let internal_name = name.replace('.', "_");
            
            // Try int variable first
            if let Some(var) = self.int_vars.get(&internal_name) {
                if let Some(val) = model.eval(var, true) {
                    if let Some(i) = val.as_i64() {
                        values.insert(name.clone(), serde_json::json!(i));
                        continue;
                    }
                }
            }
            
            // Try bool variable
            if let Some(var) = self.bool_vars.get(&internal_name) {
                if let Some(val) = model.eval(var, true) {
                    if let Some(b) = val.as_bool() {
                        values.insert(name.clone(), serde_json::json!(b));
                        continue;
                    }
                }
            }
            
            // Variable not found or couldn't evaluate
            values.insert(name.clone(), serde_json::Value::Null);
        }
        
        Ok(QueryResult {
            values,
            violated_goals: vec![],  // TODO: Track which goals were violated
        })
    }
    
    pub fn get_goal_names(&self) -> Vec<(String, u32)> {
        self.goal_names.clone()
    }
}
