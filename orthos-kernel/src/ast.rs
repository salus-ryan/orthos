use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CausalMode {
    Forward,  // -> (default)
    Omni,     // <->
}

impl Default for CausalMode {
    fn default() -> Self {
        CausalMode::Forward
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Bool,
    String,
    List(Box<Type>),
    Custom(String),  // Reference to a Boundary (struct-like)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    // Literals
    IntLit(i64),
    BoolLit(bool),
    StringLit(String),
    
    // Variable reference
    Var(String),
    
    // Field access: expr.field
    FieldAccess(Box<Expr>, String),
    
    // Index access: expr[index]
    IndexAccess(Box<Expr>, Box<Expr>),
    
    // Binary operations
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    
    // Comparison
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    
    // Logical
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Implies(Box<Expr>, Box<Expr>),
    
    // Boundary instantiation: BoundaryName(field=value, ...)
    Instantiate {
        boundary: String,
        args: Vec<(String, Expr)>,
    },
    
    // List literal: [a, b, c]
    ListLit(Vec<Expr>),
    
    // List operations
    ListFirst(Box<Expr>),
    ListLast(Box<Expr>),
    ListLen(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FluxDecl {
    pub name: String,
    pub typ: Type,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LawDecl {
    pub name: String,
    pub constraint: Expr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestBlock {
    pub name: String,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    FluxDecl(FluxDecl),
    Assignment { target: String, value: Expr },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Boundary {
    pub name: String,
    pub causal_mode: CausalMode,
    pub flux: Vec<FluxDecl>,
    pub laws: Vec<LawDecl>,
    pub manifest: Option<ManifestBlock>,
    pub nested_boundaries: Vec<Boundary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub boundaries: Vec<Boundary>,
}

impl Program {
    pub fn new() -> Self {
        Self { boundaries: Vec::new() }
    }
    
    pub fn find_boundary(&self, name: &str) -> Option<&Boundary> {
        self.boundaries.iter().find(|b| b.name == name)
    }
    
    pub fn find_main(&self) -> Option<&Boundary> {
        self.boundaries.iter().find(|b| b.name == "Main")
    }
    
    pub fn find_manifest(&self) -> Option<(&Boundary, &ManifestBlock)> {
        for boundary in &self.boundaries {
            if let Some(ref manifest) = boundary.manifest {
                return Some((boundary, manifest));
            }
        }
        None
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}
