//! Syntax data and a small optional parser for lambda calculus.
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Var(String),
    Lam(String, Box<Expr>),
    App(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn var(name: &str) -> Self {
        Self::Var(name.to_owned())
    }
    pub fn lam(parameter: &str, body: Self) -> Self {
        Self::Lam(parameter.to_owned(), Box::new(body))
    }
    pub fn app(function: Self, argument: Self) -> Self {
        Self::App(Box::new(function), Box::new(argument))
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Var(name) => write!(f, "{name}"),
            Self::Lam(parameter, body) => write!(f, "(λ{parameter}. {body})"),
            Self::App(function, argument) => write!(f, "({function} {argument})"),
        }
    }
}

/// Input convenience, separate from evaluation. Application associates left;
/// lambda bodies extend right. Use λ or a backslash, one parameter per lambda.
pub fn parse(source: &str) -> Result<Expr, String> {
    let mut parser = Parser {
        tokens: tokenize(source)?,
        position: 0,
    };
    let expression = parser.expression()?;
    if parser.position != parser.tokens.len() {
        return Err(format!(
            "unexpected token {:?}",
            parser.tokens[parser.position]
        ));
    }
    Ok(expression)
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Lambda,
    Dot,
    Open,
    Close,
    Name(String),
}

fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        let token = match character {
            character if character.is_whitespace() => continue,
            'λ' | '\\' => Token::Lambda,
            '.' => Token::Dot,
            '(' => Token::Open,
            ')' => Token::Close,
            character if character.is_alphabetic() || character == '_' => {
                let mut name = String::from(character);
                while let Some(&next) = characters.peek() {
                    // λ is alphabetic in Unicode, but is a delimiter here.
                    if next != 'λ' && (next.is_alphanumeric() || next == '_' || next == '\'') {
                        name.push(next);
                        characters.next();
                    } else {
                        break;
                    }
                }
                Token::Name(name)
            }
            _ => return Err(format!("unexpected character {character:?}")),
        };
        tokens.push(token);
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn expression(&mut self) -> Result<Expr, String> {
        if self.tokens.get(self.position) == Some(&Token::Lambda) {
            self.position += 1;
            let parameter = match self.tokens.get(self.position) {
                Some(Token::Name(name)) => name.clone(),
                _ => return Err("expected a parameter name after λ".to_owned()),
            };
            self.position += 1;
            if self.tokens.get(self.position) != Some(&Token::Dot) {
                return Err("expected '.' after the lambda parameter".to_owned());
            }
            self.position += 1;
            return Ok(Expr::lam(&parameter, self.expression()?));
        }
        let mut expression = self.atom()?;
        while matches!(
            self.tokens.get(self.position),
            Some(Token::Name(_) | Token::Open)
        ) {
            expression = Expr::app(expression, self.atom()?);
        }
        Ok(expression)
    }

    fn atom(&mut self) -> Result<Expr, String> {
        match self.tokens.get(self.position) {
            Some(Token::Name(name)) => {
                let expression = Expr::var(name);
                self.position += 1;
                Ok(expression)
            }
            Some(Token::Open) => {
                self.position += 1;
                let expression = self.expression()?;
                if self.tokens.get(self.position) != Some(&Token::Close) {
                    return Err("expected ')' to close the expression".to_owned());
                }
                self.position += 1;
                Ok(expression)
            }
            _ => Err("expected a variable or parenthesized expression".to_owned()),
        }
    }
}
