use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    // Keywords
    #[token("Boundary")]
    Boundary,
    
    #[token("Flux")]
    Flux,
    
    #[token("Law")]
    Law,
    
    #[token("Goal")]
    Goal,
    
    #[token("Manifest")]
    Manifest,
    
    #[token("Diode")]
    Diode,
    
    // Types
    #[token("Int")]
    TypeInt,
    
    #[token("Bool")]
    TypeBool,
    
    #[token("String")]
    TypeString,
    
    #[token("List")]
    TypeList,
    
    // Literals
    #[token("true")]
    True,
    
    #[token("false")]
    False,
    
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Integer(i64),
    
    #[regex(r#""[^"]*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    StringLiteral(String),
    
    // Identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string(), priority = 1)]
    Identifier(String),
    
    // Causal Mode Symbols
    #[token("<->")]
    OmniMode,
    
    #[token("->")]
    ForwardMode,
    
    // Operators
    #[token("==")]
    Equals,
    
    #[token("!=")]
    NotEquals,
    
    #[token("<=")]
    LessEquals,
    
    #[token(">=")]
    GreaterEquals,
    
    #[token("<")]
    LessThan,
    
    #[token(">")]
    GreaterThan,
    
    #[token("&&")]
    And,
    
    #[token("||")]
    Or,
    
    #[token("!")]
    Not,
    
    #[token("+")]
    Plus,
    
    #[token("-")]
    Minus,
    
    #[token("*")]
    Star,
    
    #[token("/")]
    Slash,
    
    #[token("%")]
    Modulo,
    
    #[token("implies")]
    Implies,
    
    // Punctuation
    #[token("{")]
    LBrace,
    
    #[token("}")]
    RBrace,
    
    #[token("(")]
    LParen,
    
    #[token(")")]
    RParen,
    
    #[token("[")]
    LBracket,
    
    #[token("]")]
    RBracket,
    
    #[token(":")]
    Colon,
    
    #[token(";")]
    Semicolon,
    
    #[token(",")]
    Comma,
    
    #[token(".")]
    Dot,
    
    #[token("=")]
    Assign,
    
    #[token("@")]
    At,
}

pub struct Lexer;

impl Lexer {
    pub fn tokenize(input: &str) -> Result<Vec<(Token, std::ops::Range<usize>)>, String> {
        let mut tokens = Vec::new();
        let mut lexer = Token::lexer(input);
        
        while let Some(result) = lexer.next() {
            match result {
                Ok(token) => tokens.push((token, lexer.span())),
                Err(_) => {
                    return Err(format!(
                        "Lexical error at position {}: unexpected character '{}'",
                        lexer.span().start,
                        &input[lexer.span()]
                    ));
                }
            }
        }
        
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_tokens() {
        let input = "Boundary SafeMath { Flux x : Int }";
        let tokens = Lexer::tokenize(input).unwrap();
        assert_eq!(tokens[0].0, Token::Boundary);
        assert_eq!(tokens[1].0, Token::Identifier("SafeMath".to_string()));
        assert_eq!(tokens[2].0, Token::LBrace);
    }
    
    #[test]
    fn test_causal_modes() {
        let input = "<-> ->";
        let tokens = Lexer::tokenize(input).unwrap();
        assert_eq!(tokens[0].0, Token::OmniMode);
        assert_eq!(tokens[1].0, Token::ForwardMode);
    }
}
