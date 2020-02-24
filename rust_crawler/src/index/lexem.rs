use crate::index::fprocessing::Filter;
use std::slice::Iter;
#[derive(Debug)]
pub enum Lexem {
    Or(Box<Lexem>, Box<Lexem>),
    And(Box<Lexem>, Box<Lexem>),
    Not(Box<Lexem>),
    WithFilter(Filter),
}

pub fn parse_to_lexem(text:&str) -> Option<Lexem> {
    let tokens = get_tokens(text);
    let mut iterator = tokens.iter();
    process_tokens(&mut iterator)
}

fn process_tokens(iter :&'_ mut Iter<'_, Token>) -> Option<Lexem> {
    
    let mut left_lx : Option<Lexem> = None;
    loop {

        let opt = iter.next();
        match opt {
            None => {
                break;
            }
            Some(token) => {
                match token {
                    Token::LParen => {
                        left_lx =  process_tokens(iter);
                    },
                    Token::RParen => {
                        return left_lx;
                    },
                    Token::And => {
                        left_lx = Some(Lexem::And(Box::new(left_lx.unwrap()), Box::new(process_tokens(iter).unwrap())));
                    },
                    Token::Or => {
                        left_lx = Some(Lexem::Or(Box::new(left_lx.unwrap()), Box::new(process_tokens(iter).unwrap())));
                    },
                    Token::Text(word) => {
                        match iter.next().unwrap() {
                            Token::Text(w) => {
                                match &word[..] {
                                    "startswith" => {
                                        left_lx = Some(Lexem::WithFilter(Filter::StartsWith(w.clone())));
                                    },
                                    "endswith" => {
                                        left_lx = Some(Lexem::WithFilter(Filter::EndsWith(w.clone())));
                                    },
                                    "equals" => {
                                        left_lx = Some(Lexem::WithFilter(Filter::Word(w.clone())));
                                    }
                                    _ => {
                                        panic!("unknown command");
                                    }
                                }
                            }
                            _ => {
                                panic!("word expected at");
                            }
                        }
                        
                    }
                    Token::Not => {
                        left_lx = Some(Lexem::Not(Box::new(process_tokens(iter).unwrap())));
                    }
                }
            }
        };
    }
    left_lx
}


pub fn get_tokens(text: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut token_text: String = String::new();
    for c in text.chars() {
        if c.is_alphabetic() {
            token_text.push(c);
            continue;
        }

        if c.is_whitespace() {
            if !token_text.is_empty() {
                tokens.push(Token::Text(token_text.clone()));
                token_text.clear();
            }
            continue;
        }
        if c as u16 ^ '|' as u16 == 0 {
            if !token_text.is_empty() {
                tokens.push(Token::Text(token_text.clone()));
                token_text.clear();
            }
            tokens.push(Token::Or);
            continue;
        }
        if c as u16 ^ '&' as u16 == 0 {
            if !token_text.is_empty() {
                tokens.push(Token::Text(token_text.clone()));
                token_text.clear();
            }
            tokens.push(Token::And);
            continue;
        }
        if c as u16 ^ '~' as u16 == 0 {
            if !token_text.is_empty() {
                tokens.push(Token::Text(token_text.clone()));
                token_text.clear();
            }
            tokens.push(Token::Not);
            continue;
        }
        if c as u16 ^ '(' as u16 == 0 {
            if !token_text.is_empty() {
                tokens.push(Token::Text(token_text.clone()));
                token_text.clear();
            }
            tokens.push(Token::LParen);
            continue;
        }
        if c as u16 ^ ')' as u16 == 0 {
            if !token_text.is_empty() {
                tokens.push(Token::Text(token_text.clone()));
                token_text.clear();
            }
            tokens.push(Token::RParen);
            continue;
        }
    }


    tokens
}


#[derive(Debug)]
pub enum Token {
    LParen,
    RParen,
    Text(String),
    Or,
    And,
    Not,
}
