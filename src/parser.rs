use lexer;
use token::{Keyword, Kind, Symbol};
use node::{BinOp, Node};

#[derive(Clone, Debug)]
pub struct Parser {
    pub lexer: lexer::Lexer,
}

impl Parser {
    pub fn new(code: String) -> Parser {
        Parser {
            lexer: lexer::Lexer::new(code),
        }
    }
}

impl Parser {
    pub fn next(&mut self) -> Result<Node, ()> {
        self.read_script()
    }
}

impl Parser {
    fn read_script(&mut self) -> Result<Node, ()> {
        self.read_statement_list()
    }
}

impl Parser {
    fn read_statement_list(&mut self) -> Result<Node, ()> {
        let mut items = vec![];

        loop {
            if self.lexer.eof() {
                if items.is_empty() {
                    return Err(());
                }
                break;
            }

            if let Ok(ok) = self.lexer.peek() {
                if ok.is_the_symbol(Symbol::ClosingBrace) {
                    break;
                }
            }

            if let Ok(item) = self.read_statement_list_item() {
                items.push(item)
            }
        }

        Ok(Node::StatementList(items))
    }

    fn read_statement_list_item(&mut self) -> Result<Node, ()> {
        if self.is_declaration() {
            self.read_declaration()
        } else {
            self.read_statement()
        }
    }

    fn read_statement(&mut self) -> Result<Node, ()> {
        let tok = self.lexer.next()?;
        match tok.kind {
            Kind::Keyword(Keyword::If) => self.read_if_statement(),
            _ => {
                self.lexer.unget(&tok);
                self.read_expression_statement()
            }
        }
    }
}

impl Parser {
    fn read_if_statement(&mut self) -> Result<Node, ()> {
        assert_eq!(self.lexer.next()?.kind, Kind::Symbol(Symbol::OpeningParen));
        let cond = self.read_expression()?;
        assert_eq!(self.lexer.next()?.kind, Kind::Symbol(Symbol::ClosingParen));

        let then_ = self.read_statement()?;

        let expect_else_tok = self.lexer.next()?;
        if expect_else_tok.kind == Kind::Keyword(Keyword::Else) {
            let else_ = self.read_statement()?;
            return Ok(Node::If(Box::new(cond), Box::new(then_), Box::new(else_)));
        } else {
            self.lexer.unget(&expect_else_tok);
        }

        Ok(Node::If(
            Box::new(cond),
            Box::new(then_),
            Box::new(Node::StatementList(vec![])),
        ))
    }
}

impl Parser {
    fn read_expression_statement(&mut self) -> Result<Node, ()> {
        self.read_expression()
    }

    fn read_expression(&mut self) -> Result<Node, ()> {
        let lhs = self.read_assignment_expression();
        lhs
    }

    /// https://tc39.github.io/ecma262/#prod-AssignmentExpression
    fn read_assignment_expression(&mut self) -> Result<Node, ()> {
        let lhs = self.read_equality_expression();
        lhs
    }

    /// https://tc39.github.io/ecma262/#prod-EqualityExpression
    fn read_equality_expression(&mut self) -> Result<Node, ()> {
        let mut lhs = self.read_additive_expression()?;
        let tok = self.lexer.next()?;
        match tok.kind {
            Kind::Symbol(Symbol::Eq) => {
                lhs = Node::BinOp(
                    Box::new(lhs),
                    Box::new(self.read_equality_expression()?),
                    BinOp::Eq,
                )
            }
            Kind::Symbol(Symbol::Ne) => {
                lhs = Node::BinOp(
                    Box::new(lhs),
                    Box::new(self.read_equality_expression()?),
                    BinOp::Ne,
                )
            }
            _ => self.lexer.unget(&tok),
        };
        Ok(lhs)
    }

    /// https://tc39.github.io/ecma262/#prod-AssignmentExpression
    fn read_additive_expression(&mut self) -> Result<Node, ()> {
        let mut lhs = self.read_multiplicative_expression()?;
        let tok = self.lexer.next()?;
        match tok.kind {
            Kind::Symbol(Symbol::Add) => {
                lhs = Node::BinOp(
                    Box::new(lhs),
                    Box::new(self.read_additive_expression()?),
                    BinOp::Add,
                )
            }
            Kind::Symbol(Symbol::Sub) => {
                lhs = Node::BinOp(
                    Box::new(lhs),
                    Box::new(self.read_additive_expression()?),
                    BinOp::Sub,
                )
            }
            _ => self.lexer.unget(&tok),
        };
        Ok(lhs)
    }

    /// https://tc39.github.io/ecma262/#prod-AssignmentExpression
    fn read_multiplicative_expression(&mut self) -> Result<Node, ()> {
        let mut lhs = self.read_primary_expression()?;
        let tok = self.lexer.next()?;
        match tok.kind {
            Kind::Symbol(Symbol::Asterisk) => {
                lhs = Node::BinOp(
                    Box::new(lhs),
                    Box::new(self.read_multiplicative_expression()?),
                    BinOp::Mul,
                )
            }
            Kind::Symbol(Symbol::Div) => {
                lhs = Node::BinOp(
                    Box::new(lhs),
                    Box::new(self.read_multiplicative_expression()?),
                    BinOp::Div,
                )
            }
            _ => self.lexer.unget(&tok),
        };
        Ok(lhs)
    }

    /// https://tc39.github.io/ecma262/#prod-PrimaryExpression
    fn read_primary_expression(&mut self) -> Result<Node, ()> {
        match self.lexer.next()?.kind {
            Kind::Keyword(Keyword::This) => unimplemented!(),
            Kind::Identifier(ref i) if i == "true" => Ok(Node::Boolean(true)),
            Kind::Identifier(ref i) if i == "false" => Ok(Node::Boolean(false)),
            Kind::Identifier(ident) => Ok(Node::Identifier(ident)),
            Kind::Number(num) => Ok(Node::Number(num)),
            Kind::LineTerminator => self.read_primary_expression(),
            e => unimplemented!("{:?}", e),
        }
    }
}

impl Parser {
    fn is_declaration(&mut self) -> bool {
        self.is_hoistable_declaration()
    }

    fn read_declaration(&mut self) -> Result<Node, ()> {
        Err(())
    }
}

impl Parser {
    /// https://tc39.github.io/ecma262/#prod-HoistableDeclaration
    fn is_hoistable_declaration(&mut self) -> bool {
        self.is_function_declaration()
    }
}

impl Parser {
    /// https://tc39.github.io/ecma262/#prod-FunctionDeclaration
    fn is_function_declaration(&mut self) -> bool {
        match self.lexer.peek() {
            Ok(tok) => tok.is_the_keyword(Keyword::Function),
            Err(_) => false,
        }
    }
}