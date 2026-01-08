use crate::ast::*;
use crate::lexer::Token;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected end of input")]
    UnexpectedEof,
    
    #[error("Expected {expected}, found {found:?}")]
    Expected { expected: String, found: Option<Token> },
    
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(Token),
    
    #[error("Invalid expression")]
    InvalidExpression,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }
    
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    
    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }
    
    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        match self.advance() {
            Some(ref t) if t == &expected => Ok(()),
            other => Err(ParseError::Expected {
                expected: format!("{:?}", expected),
                found: other,
            }),
        }
    }
    
    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Some(Token::Identifier(name)) => Ok(name),
            other => Err(ParseError::Expected {
                expected: "identifier".to_string(),
                found: other,
            }),
        }
    }
    
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut program = Program::new();
        
        while self.peek().is_some() {
            let boundary = self.parse_boundary()?;
            program.boundaries.push(boundary);
        }
        
        Ok(program)
    }
    
    fn parse_boundary(&mut self) -> Result<Boundary, ParseError> {
        self.expect(Token::Boundary)?;
        let name = self.expect_identifier()?;
        
        // Check for causal mode
        let causal_mode = match self.peek() {
            Some(Token::OmniMode) => {
                self.advance();
                CausalMode::Omni
            }
            Some(Token::ForwardMode) => {
                self.advance();
                CausalMode::Forward
            }
            _ => CausalMode::Forward,
        };
        
        self.expect(Token::LBrace)?;
        
        let mut flux = Vec::new();
        let mut laws = Vec::new();
        let mut manifest = None;
        let mut nested_boundaries = Vec::new();
        
        while self.peek() != Some(&Token::RBrace) {
            match self.peek() {
                Some(Token::Flux) => {
                    flux.push(self.parse_flux_decl()?);
                }
                Some(Token::Law) => {
                    laws.push(self.parse_law_decl()?);
                }
                Some(Token::Manifest) => {
                    manifest = Some(self.parse_manifest()?);
                }
                Some(Token::Boundary) => {
                    nested_boundaries.push(self.parse_boundary()?);
                }
                Some(t) => return Err(ParseError::UnexpectedToken(t.clone())),
                None => return Err(ParseError::UnexpectedEof),
            }
        }
        
        self.expect(Token::RBrace)?;
        
        Ok(Boundary {
            name,
            causal_mode,
            flux,
            laws,
            manifest,
            nested_boundaries,
        })
    }
    
    fn parse_flux_decl(&mut self) -> Result<FluxDecl, ParseError> {
        self.expect(Token::Flux)?;
        let name = self.expect_identifier()?;
        
        // Optional type annotation
        let typ = if self.peek() == Some(&Token::Colon) {
            self.advance();
            self.parse_type()?
        } else if self.peek() == Some(&Token::Assign) {
            // Infer type from initializer (default to Int for now)
            Type::Int
        } else {
            Type::Int
        };
        
        // Optional initializer
        let init = if self.peek() == Some(&Token::Assign) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        
        Ok(FluxDecl { name, typ, init })
    }
    
    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match self.advance() {
            Some(Token::TypeInt) => Ok(Type::Int),
            Some(Token::TypeBool) => Ok(Type::Bool),
            Some(Token::TypeString) => Ok(Type::String),
            Some(Token::TypeList) => {
                self.expect(Token::LessThan)?;
                let inner = self.parse_type()?;
                self.expect(Token::GreaterThan)?;
                Ok(Type::List(Box::new(inner)))
            }
            Some(Token::Identifier(name)) => Ok(Type::Custom(name)),
            other => Err(ParseError::Expected {
                expected: "type".to_string(),
                found: other,
            }),
        }
    }
    
    fn parse_law_decl(&mut self) -> Result<LawDecl, ParseError> {
        self.expect(Token::Law)?;
        let name = self.expect_identifier()?;
        self.expect(Token::Colon)?;
        let constraint = self.parse_expr()?;
        
        Ok(LawDecl { name, constraint })
    }
    
    fn parse_manifest(&mut self) -> Result<ManifestBlock, ParseError> {
        self.expect(Token::Manifest)?;
        let name = self.expect_identifier()?;
        self.expect(Token::LBrace)?;
        
        let mut statements = Vec::new();
        
        while self.peek() != Some(&Token::RBrace) {
            statements.push(self.parse_statement()?);
        }
        
        self.expect(Token::RBrace)?;
        
        Ok(ManifestBlock { name, statements })
    }
    
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            Some(Token::Flux) => {
                let decl = self.parse_flux_decl()?;
                Ok(Statement::FluxDecl(decl))
            }
            Some(Token::Identifier(_)) => {
                let name = self.expect_identifier()?;
                self.expect(Token::Assign)?;
                let value = self.parse_expr()?;
                Ok(Statement::Assignment { target: name, value })
            }
            other => Err(ParseError::Expected {
                expected: "statement".to_string(),
                found: other.cloned(),
            }),
        }
    }
    
    // Expression parsing with precedence climbing
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_implies()
    }
    
    fn parse_implies(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_or()?;
        
        while self.peek() == Some(&Token::Implies) {
            self.advance();
            let right = self.parse_or()?;
            left = Expr::Implies(Box::new(left), Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        
        while self.peek() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        
        while self.peek() == Some(&Token::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive()?;
        
        loop {
            let op = match self.peek() {
                Some(Token::Equals) => Some("=="),
                Some(Token::NotEquals) => Some("!="),
                Some(Token::LessThan) => Some("<"),
                Some(Token::LessEquals) => Some("<="),
                Some(Token::GreaterThan) => Some(">"),
                Some(Token::GreaterEquals) => Some(">="),
                _ => None,
            };
            
            if let Some(op) = op {
                self.advance();
                let right = self.parse_additive()?;
                left = match op {
                    "==" => Expr::Eq(Box::new(left), Box::new(right)),
                    "!=" => Expr::Ne(Box::new(left), Box::new(right)),
                    "<" => Expr::Lt(Box::new(left), Box::new(right)),
                    "<=" => Expr::Le(Box::new(left), Box::new(right)),
                    ">" => Expr::Gt(Box::new(left), Box::new(right)),
                    ">=" => Expr::Ge(Box::new(left), Box::new(right)),
                    _ => unreachable!(),
                };
            } else {
                break;
            }
        }
        
        Ok(left)
    }
    
    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Add(Box::new(left), Box::new(right));
                }
                Some(Token::Minus) => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Sub(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        
        Ok(left)
    }
    
    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Mul(Box::new(left), Box::new(right));
                }
                Some(Token::Slash) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Div(Box::new(left), Box::new(right));
                }
                Some(Token::Modulo) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Mod(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        
        Ok(left)
    }
    
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Some(Token::Not) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Not(Box::new(expr)))
            }
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Sub(Box::new(Expr::IntLit(0)), Box::new(expr)))
            }
            _ => self.parse_postfix(),
        }
    }
    
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        
        loop {
            match self.peek() {
                Some(Token::Dot) => {
                    self.advance();
                    let field = self.expect_identifier()?;
                    
                    // Check for built-in list methods
                    match field.as_str() {
                        "first" => expr = Expr::ListFirst(Box::new(expr)),
                        "last" => expr = Expr::ListLast(Box::new(expr)),
                        "len" => expr = Expr::ListLen(Box::new(expr)),
                        _ => expr = Expr::FieldAccess(Box::new(expr), field),
                    }
                }
                Some(Token::LBracket) => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    expr = Expr::IndexAccess(Box::new(expr), Box::new(index));
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().cloned() {
            Some(Token::Integer(n)) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::BoolLit(true))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::BoolLit(false))
            }
            Some(Token::StringLiteral(s)) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(Token::LBracket) => {
                self.advance();
                let mut elements = Vec::new();
                
                if self.peek() != Some(&Token::RBracket) {
                    elements.push(self.parse_expr()?);
                    while self.peek() == Some(&Token::Comma) {
                        self.advance();
                        elements.push(self.parse_expr()?);
                    }
                }
                
                self.expect(Token::RBracket)?;
                Ok(Expr::ListLit(elements))
            }
            Some(Token::Identifier(name)) => {
                self.advance();
                
                // Check if this is a boundary instantiation
                if self.peek() == Some(&Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    
                    if self.peek() != Some(&Token::RParen) {
                        loop {
                            let arg_name = self.expect_identifier()?;
                            self.expect(Token::Assign)?;
                            let arg_value = self.parse_expr()?;
                            args.push((arg_name, arg_value));
                            
                            if self.peek() == Some(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    
                    self.expect(Token::RParen)?;
                    Ok(Expr::Instantiate { boundary: name, args })
                } else {
                    Ok(Expr::Var(name))
                }
            }
            other => Err(ParseError::Expected {
                expected: "expression".to_string(),
                found: other,
            }),
        }
    }
}

pub fn parse(source: &str) -> Result<Program, String> {
    let tokens_with_spans = crate::lexer::Lexer::tokenize(source)?;
    let tokens: Vec<Token> = tokens_with_spans.into_iter().map(|(t, _)| t).collect();
    
    let mut parser = Parser::new(tokens);
    parser.parse_program().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_boundary() {
        let source = r#"
            Boundary Point {
                Flux x : Int
                Flux y : Int
            }
        "#;
        
        let program = parse(source).unwrap();
        assert_eq!(program.boundaries.len(), 1);
        assert_eq!(program.boundaries[0].name, "Point");
        assert_eq!(program.boundaries[0].flux.len(), 2);
    }
    
    #[test]
    fn test_parse_law() {
        let source = r#"
            Boundary Test {
                Flux x : Int
                Law Positive : x > 0
            }
        "#;
        
        let program = parse(source).unwrap();
        assert_eq!(program.boundaries[0].laws.len(), 1);
        assert_eq!(program.boundaries[0].laws[0].name, "Positive");
    }
    
    #[test]
    fn test_parse_omni_mode() {
        let source = r#"
            Boundary Solver <-> {
                Flux x : Int
            }
        "#;
        
        let program = parse(source).unwrap();
        assert_eq!(program.boundaries[0].causal_mode, CausalMode::Omni);
    }
}
