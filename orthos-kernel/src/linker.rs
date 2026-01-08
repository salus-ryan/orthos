use crate::ast::{Boundary, Expr, FluxDecl, GoalDecl, ImportDecl, LawDecl, Program};
use crate::parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LinkError {
    #[error("Failed to read import '{path}': {reason}")]
    ImportReadError { path: String, reason: String },

    #[error("Failed to parse import '{path}': {reason}")]
    ImportParseError { path: String, reason: String },

    #[error("Circular import detected: {path}")]
    CircularImport { path: String },

    #[error("Import not found: {path}")]
    ImportNotFound { path: String },
}

pub struct Linker {
    base_path: PathBuf,
    visited: HashSet<PathBuf>,
}

impl Linker {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            visited: HashSet::new(),
        }
    }

    pub fn link(&mut self, program: Program) -> Result<Program, LinkError> {
        let mut linked = Program::new();

        // Process each import
        for import in &program.imports {
            let imported_boundaries = self.resolve_import(import)?;
            linked.boundaries.extend(imported_boundaries);
        }

        // Add the program's own boundaries
        linked.boundaries.extend(program.boundaries);

        Ok(linked)
    }

    fn resolve_import(&mut self, import: &ImportDecl) -> Result<Vec<Boundary>, LinkError> {
        // Resolve the import path relative to base_path
        let import_path = self.base_path.join(&import.path);
        let canonical = import_path
            .canonicalize()
            .map_err(|_| LinkError::ImportNotFound {
                path: import.path.clone(),
            })?;

        // Check for circular imports
        if self.visited.contains(&canonical) {
            return Err(LinkError::CircularImport {
                path: import.path.clone(),
            });
        }
        self.visited.insert(canonical.clone());

        // Read the file
        let source = std::fs::read_to_string(&canonical).map_err(|e| LinkError::ImportReadError {
            path: import.path.clone(),
            reason: e.to_string(),
        })?;

        // Parse the imported file
        let imported_program =
            parser::parse(&source).map_err(|e| LinkError::ImportParseError {
                path: import.path.clone(),
                reason: e,
            })?;

        // Recursively resolve imports in the imported file
        let mut sub_linker = Linker {
            base_path: canonical.parent().unwrap_or(&self.base_path).to_path_buf(),
            visited: self.visited.clone(),
        };
        let resolved = sub_linker.link(imported_program)?;
        self.visited.extend(sub_linker.visited);

        // Prefix all boundaries with the alias
        let prefixed: Vec<Boundary> = resolved
            .boundaries
            .into_iter()
            .map(|b| self.prefix_boundary(b, &import.alias))
            .collect();

        Ok(prefixed)
    }

    fn prefix_boundary(&self, mut boundary: Boundary, prefix: &str) -> Boundary {
        // Prefix the boundary name: Velocity -> Physics.Velocity
        boundary.name = format!("{}.{}", prefix, boundary.name);

        // Prefix flux declarations that reference other boundaries
        for flux in &mut boundary.flux {
            self.prefix_flux_decl(flux, prefix);
        }

        // Prefix expressions in laws
        for law in &mut boundary.laws {
            self.prefix_law_decl(law, prefix);
        }

        // Prefix expressions in goals
        for goal in &mut boundary.goals {
            self.prefix_goal_decl(goal, prefix);
        }

        // Prefix nested boundaries
        boundary.nested_boundaries = boundary
            .nested_boundaries
            .into_iter()
            .map(|b| self.prefix_boundary(b, prefix))
            .collect();

        boundary
    }

    fn prefix_flux_decl(&self, flux: &mut FluxDecl, prefix: &str) {
        // Prefix custom types
        if let crate::ast::Type::Custom(ref mut name) = flux.typ {
            *name = format!("{}.{}", prefix, name);
        }

        // Prefix initializer expression
        if let Some(ref mut init) = flux.init {
            self.prefix_expr(init, prefix);
        }
    }

    fn prefix_law_decl(&self, law: &mut LawDecl, prefix: &str) {
        self.prefix_expr(&mut law.constraint, prefix);
    }

    fn prefix_goal_decl(&self, goal: &mut GoalDecl, prefix: &str) {
        self.prefix_expr(&mut goal.constraint, prefix);
    }

    fn prefix_expr(&self, expr: &mut Expr, prefix: &str) {
        match expr {
            // Boundary instantiation: Velocity(...) -> Physics.Velocity(...)
            Expr::Instantiate { boundary, args } => {
                *boundary = format!("{}.{}", prefix, boundary);
                for (_, arg_expr) in args {
                    self.prefix_expr(arg_expr, prefix);
                }
            }

            // Recursively handle all expression types
            Expr::FieldAccess(inner, _) => self.prefix_expr(inner, prefix),
            Expr::IndexAccess(inner, index) => {
                self.prefix_expr(inner, prefix);
                self.prefix_expr(index, prefix);
            }
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mul(l, r)
            | Expr::Div(l, r)
            | Expr::Mod(l, r)
            | Expr::Eq(l, r)
            | Expr::Ne(l, r)
            | Expr::Lt(l, r)
            | Expr::Le(l, r)
            | Expr::Gt(l, r)
            | Expr::Ge(l, r)
            | Expr::And(l, r)
            | Expr::Or(l, r)
            | Expr::Implies(l, r) => {
                self.prefix_expr(l, prefix);
                self.prefix_expr(r, prefix);
            }
            Expr::Not(inner) => self.prefix_expr(inner, prefix),
            Expr::ListLit(elements) => {
                for elem in elements {
                    self.prefix_expr(elem, prefix);
                }
            }
            Expr::ListFirst(inner) | Expr::ListLast(inner) | Expr::ListLen(inner) => {
                self.prefix_expr(inner, prefix);
            }

            // Literals and variables don't need prefixing
            Expr::IntLit(_) | Expr::BoolLit(_) | Expr::StringLit(_) | Expr::Var(_) => {}
        }
    }
}

pub fn link_program(source: &str, base_path: impl AsRef<Path>) -> Result<Program, String> {
    // First parse the source
    let program = parser::parse(source)?;

    // Then link imports
    let mut linker = Linker::new(base_path);
    linker.link(program).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_import() {
        let source = r#"
            import "std/physics.orth" as Physics
            
            Boundary Main {
                Flux x : Int
            }
        "#;

        let program = parser::parse(source).unwrap();
        assert_eq!(program.imports.len(), 1);
        assert_eq!(program.imports[0].path, "std/physics.orth");
        assert_eq!(program.imports[0].alias, "Physics");
    }
}
