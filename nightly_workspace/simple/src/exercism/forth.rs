//! re: <https://exercism.org/tracks/rust/exercises/forth/edit>

use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use std::result::Result as StdResult;

pub type Value = i32;
pub type Result = StdResult<(), Error>;

pub struct Forth {
    stack: Vec<Value>,
    subroutines: HashMap<String, Rc<Subroutine>>,
    state: State,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum State {
    NormalExpression,
    DefiningSubroutine,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    DivisionByZero,
    StackUnderflow,
    UnknownWord,
    InvalidWord,
}

enum Subroutine {
    Add,
    Sub,
    Mul,
    Div,
    Drop,
    Swap,
    Dup,
    Over,
    Push(Value),
    Custom(Vec<Rc<Subroutine>>),
}

impl Subroutine {
    fn apply(&self, stack: &mut Vec<Value>) -> Result {
        match self {
            Subroutine::Add => {
                let rhs = stack.pop().ok_or(Error::StackUnderflow)?;
                let lhs = stack.pop().ok_or(Error::StackUnderflow)?;
                stack.push(lhs + rhs);
            }
            Subroutine::Sub => {
                let rhs = stack.pop().ok_or(Error::StackUnderflow)?;
                let lhs = stack.pop().ok_or(Error::StackUnderflow)?;
                stack.push(lhs - rhs);
            }
            Subroutine::Mul => {
                let rhs = stack.pop().ok_or(Error::StackUnderflow)?;
                let lhs = stack.pop().ok_or(Error::StackUnderflow)?;
                stack.push(lhs * rhs);
            }
            Subroutine::Div => {
                let rhs = stack.pop().ok_or(Error::StackUnderflow)?;
                if rhs == 0 {
                    return Err(Error::DivisionByZero);
                }
                let lhs = stack.pop().ok_or(Error::StackUnderflow)?;
                stack.push(lhs / rhs);
            }
            Subroutine::Drop => {
                stack.pop().ok_or(Error::StackUnderflow)?;
            }
            Subroutine::Swap => {
                if stack.len() < 2 {
                    return Err(Error::StackUnderflow);
                }
                let pos1 = stack.len() - 1;
                let pos2 = stack.len() - 2;
                stack.swap(pos1, pos2);
            }
            Subroutine::Dup => {
                stack.push(stack.last().copied().ok_or(Error::StackUnderflow)?);
            }
            Subroutine::Over => {
                if stack.len() < 2 {
                    return Err(Error::StackUnderflow);
                }
                stack.push(
                    stack
                        .get(stack.len() - 2)
                        .copied()
                        .ok_or(Error::StackUnderflow)?,
                );
            }
            Subroutine::Push(val) => {
                stack.push(*val);
            }
            Subroutine::Custom(steps) => {
                for step in steps {
                    step.apply(stack)?;
                }
            }
        }
        Ok(())
    }
}

enum Token<'a> {
    Number(i32),
    Colon,
    Semicolon,
    Symbol(&'a str),
}

// NB: Cannot impl FromStr (and thus enable str::parse() -> Token) because
// FromStr::from_str() trait method cannot bind its parameter to 'a of Token
// i.e., Token return by str::parse() cannot borrow from str
impl<'a> From<&'a str> for Token<'a> {
    fn from(s: &'a str) -> Self {
        match s {
            ":" => Token::Colon,
            ";" => Token::Semicolon,
            s if s.chars().all(char::is_numeric)
                || (s.starts_with('-')
                    && s.len() > 1
                    && s.chars().skip(1).all(char::is_numeric)) =>
            {
                // unwrap cannot fail because our match pattern ensures chars are ok
                Token::Number(s.parse::<Value>().unwrap())
            }
            s => Token::Symbol(s),
        }
    }
}

struct Tokenizer<'a, I> {
    wrapped: I,
    _phantom: PhantomData<&'a I>,
}

impl<'a, I> Tokenizer<'a, I> {
    fn new(wrapped: I) -> Self {
        Self {
            wrapped,
            _phantom: PhantomData,
        }
    }
}

impl<'a, I> Iterator for Tokenizer<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.wrapped.next().map(|s| s.into())
    }
}

impl Forth {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Forth {
        let mut subroutines: HashMap<String, Rc<Subroutine>> = HashMap::new();
        subroutines.insert("+".to_string(), Rc::new(Subroutine::Add));
        subroutines.insert("-".to_string(), Rc::new(Subroutine::Sub));
        subroutines.insert("*".to_string(), Rc::new(Subroutine::Mul));
        subroutines.insert("/".to_string(), Rc::new(Subroutine::Div));
        subroutines.insert("drop".to_string(), Rc::new(Subroutine::Drop));
        subroutines.insert("swap".to_string(), Rc::new(Subroutine::Swap));
        subroutines.insert("dup".to_string(), Rc::new(Subroutine::Dup));
        subroutines.insert("over".to_string(), Rc::new(Subroutine::Over));
        Self {
            stack: Vec::new(),
            subroutines,
            state: State::NormalExpression,
        }
    }

    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    pub fn eval(&mut self, input: &str) -> Result {
        let mut tokenizer = Tokenizer::new(input.split_ascii_whitespace());
        let sub = self.parse_expression(&mut tokenizer)?;
        sub.apply(&mut self.stack)?;
        Ok(())
    }

    /// Parse an expression.
    /// May have the side effect of register new subroutines.
    /// A literal expression is encoded as a subroutine (with push operations)
    /// should be executed on an empty stack.
    ///
    /// # Args
    /// - tokens - input stream to parse
    fn parse_expression<'a, 't>(
        &mut self,
        tokenizer: &mut impl Iterator<Item = Token<'t>>,
    ) -> StdResult<Subroutine, Error> {
        let mut steps = Vec::new();
        let mut is_definition_terminated = false;
        while let Some(token) = tokenizer.next() {
            // cannot put if condition while loop above due to 2021 edition error:
            //  error: let chains are only allowed in Rust 2024 or later
            if !is_definition_terminated {
                break;
            }
            match token {
                Token::Colon if self.state == State::NormalExpression => {
                    // Define subroutine

                    // subroutine name is first thing that follows
                    let sub_name = tokenizer.next().ok_or(Error::InvalidWord)?;
                    let Token::Symbol(sub_name) = sub_name else {
                        return Err(Error::InvalidWord);
                    };
                    let sub_name = sub_name.to_ascii_lowercase();

                    self.state = State::DefiningSubroutine;
                    let sub = self.parse_expression(tokenizer)?;
                    self.state = State::NormalExpression;
                    self.subroutines.insert(sub_name, Rc::new(sub));
                }
                Token::Semicolon if self.state == State::DefiningSubroutine => {
                    is_definition_terminated = true
                }
                Token::Number(val) => {
                    // Parsed number
                    steps.push(Rc::new(Subroutine::Push(val)));
                }
                Token::Symbol(sub_name) => {
                    // Indicate call to subroutine
                    let sub = self
                        .subroutines
                        .get(&sub_name.to_ascii_lowercase())
                        .ok_or(Error::UnknownWord)?;
                    steps.push(Rc::clone(sub));
                }
                _ => return Err(Error::InvalidWord),
            }
        }
        if self.state == State::DefiningSubroutine && !is_definition_terminated {
            // Missing ";" termination in subroutine definition
            return Err(Error::InvalidWord);
        }
        Ok(Subroutine::Custom(steps))
    }
}

#[cfg(test)]
mod tests {
    mod parsing_and_numbers {
        use super::super::*;

        #[test]
        fn numbers_just_get_pushed_onto_the_stack() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 3 4 5").is_ok());
            assert_eq!(f.stack(), [1, 2, 3, 4, 5]);
        }
        #[test]
        fn pushes_negative_numbers_onto_the_stack() {
            let mut f = Forth::new();
            assert!(f.eval("-1 -2 -3 -4 -5").is_ok());
            assert_eq!(f.stack(), [-1, -2, -3, -4, -5]);
        }
    }
    mod addition {
        use super::super::*;
        #[test]
        fn can_add_two_numbers() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 +").is_ok());
            assert_eq!(f.stack(), [3]);
        }
        #[test]
        fn errors_if_there_is_nothing_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("+"), Err(Error::StackUnderflow));
        }
        #[test]
        fn errors_if_there_is_only_one_value_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("1 +"), Err(Error::StackUnderflow));
        }
        #[test]
        fn more_than_two_values_on_the_stack() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 3 +").is_ok());
            assert_eq!(f.stack(), [1, 5]);
        }
    }
    mod subtraction {
        use super::super::*;
        #[test]
        fn can_subtract_two_numbers() {
            let mut f = Forth::new();
            assert!(f.eval("3 4 -").is_ok());
            assert_eq!(f.stack(), [-1]);
        }
        #[test]
        fn errors_if_there_is_nothing_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("-"), Err(Error::StackUnderflow));
        }
        #[test]
        fn errors_if_there_is_only_one_value_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("1 -"), Err(Error::StackUnderflow));
        }
        #[test]
        fn more_than_two_values_on_the_stack() {
            let mut f = Forth::new();
            assert!(f.eval("1 12 3 -").is_ok());
            assert_eq!(f.stack(), [1, 9]);
        }
    }
    mod multiplication {
        use super::super::*;
        #[test]
        fn can_multiply_two_numbers() {
            let mut f = Forth::new();
            assert!(f.eval("2 4 *").is_ok());
            assert_eq!(f.stack(), [8]);
        }
        #[test]
        fn errors_if_there_is_nothing_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("*"), Err(Error::StackUnderflow));
        }
        #[test]
        fn errors_if_there_is_only_one_value_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("1 *"), Err(Error::StackUnderflow));
        }
        #[test]
        fn more_than_two_values_on_the_stack() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 3 *").is_ok());
            assert_eq!(f.stack(), [1, 6]);
        }
    }
    mod division {
        use super::super::*;
        #[test]
        fn can_divide_two_numbers() {
            let mut f = Forth::new();
            assert!(f.eval("12 3 /").is_ok());
            assert_eq!(f.stack(), [4]);
        }
        #[test]
        fn performs_integer_division() {
            let mut f = Forth::new();
            assert!(f.eval("8 3 /").is_ok());
            assert_eq!(f.stack(), [2]);
        }
        #[test]
        fn errors_if_dividing_by_zero() {
            let mut f = Forth::new();
            assert_eq!(f.eval("4 0 /"), Err(Error::DivisionByZero));
        }
        #[test]
        fn errors_if_there_is_nothing_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("/"), Err(Error::StackUnderflow));
        }
        #[test]
        fn errors_if_there_is_only_one_value_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("1 /"), Err(Error::StackUnderflow));
        }
        #[test]
        fn more_than_two_values_on_the_stack() {
            let mut f = Forth::new();
            assert!(f.eval("1 12 3 /").is_ok());
            assert_eq!(f.stack(), [1, 4]);
        }
    }
    mod combined_arithmetic {
        use super::super::*;
        #[test]
        fn addition_and_subtraction() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 + 4 -").is_ok());
            assert_eq!(f.stack(), [-1]);
        }
        #[test]
        fn multiplication_and_division() {
            let mut f = Forth::new();
            assert!(f.eval("2 4 * 3 /").is_ok());
            assert_eq!(f.stack(), [2]);
        }
        #[test]
        fn multiplication_and_addition() {
            let mut f = Forth::new();
            assert!(f.eval("1 3 4 * +").is_ok());
            assert_eq!(f.stack(), [13]);
        }
        #[test]
        fn addition_and_multiplication() {
            let mut f = Forth::new();
            assert!(f.eval("1 3 4 + *").is_ok());
            assert_eq!(f.stack(), [7]);
        }
    }
    mod dup {
        use super::super::*;
        #[test]
        fn copies_a_value_on_the_stack() {
            let mut f = Forth::new();
            assert!(f.eval("1 dup").is_ok());
            assert_eq!(f.stack(), [1, 1]);
        }
        #[test]
        fn copies_the_top_value_on_the_stack() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 dup").is_ok());
            assert_eq!(f.stack(), [1, 2, 2]);
        }
        #[test]
        fn errors_if_there_is_nothing_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("dup"), Err(Error::StackUnderflow));
        }
    }
    mod drop {
        use super::super::*;
        #[test]
        fn removes_the_top_value_on_the_stack_if_it_is_the_only_one() {
            let mut f = Forth::new();
            assert!(f.eval("1 drop").is_ok());
            assert_eq!(f.stack(), []);
        }
        #[test]
        fn removes_the_top_value_on_the_stack_if_it_is_not_the_only_one() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 drop").is_ok());
            assert_eq!(f.stack(), [1]);
        }
        #[test]
        fn errors_if_there_is_nothing_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("drop"), Err(Error::StackUnderflow));
        }
    }
    mod swap {
        use super::super::*;
        #[test]
        fn swaps_the_top_two_values_on_the_stack_if_they_are_the_only_ones() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 swap").is_ok());
            assert_eq!(f.stack(), [2, 1]);
        }
        #[test]
        fn swaps_the_top_two_values_on_the_stack_if_they_are_not_the_only_ones() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 3 swap").is_ok());
            assert_eq!(f.stack(), [1, 3, 2]);
        }
        #[test]
        fn errors_if_there_is_nothing_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("swap"), Err(Error::StackUnderflow));
        }
        #[test]
        fn errors_if_there_is_only_one_value_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("1 swap"), Err(Error::StackUnderflow));
        }
    }
    mod over {
        use super::super::*;
        #[test]
        fn copies_the_second_element_if_there_are_only_two() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 over").is_ok());
            assert_eq!(f.stack(), [1, 2, 1]);
        }
        #[test]
        fn copies_the_second_element_if_there_are_more_than_two() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 3 over").is_ok());
            assert_eq!(f.stack(), [1, 2, 3, 2]);
        }
        #[test]
        fn errors_if_there_is_nothing_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("over"), Err(Error::StackUnderflow));
        }
        #[test]
        fn errors_if_there_is_only_one_value_on_the_stack() {
            let mut f = Forth::new();
            assert_eq!(f.eval("1 over"), Err(Error::StackUnderflow));
        }
    }
    mod user_defined_words {
        use super::super::*;
        #[test]
        fn can_consist_of_built_in_words() {
            let mut f = Forth::new();
            assert!(f.eval(": dup-twice dup dup ;").is_ok());
            assert!(f.eval("1 dup-twice").is_ok());
            assert_eq!(f.stack(), [1, 1, 1]);
        }
        #[test]
        fn execute_in_the_right_order() {
            let mut f = Forth::new();
            assert!(f.eval(": countup 1 2 3 ;").is_ok());
            assert!(f.eval("countup").is_ok());
            assert_eq!(f.stack(), [1, 2, 3]);
        }
        #[test]
        fn can_override_other_user_defined_words() {
            let mut f = Forth::new();
            assert!(f.eval(": foo dup ;").is_ok());
            assert!(f.eval(": foo dup dup ;").is_ok());
            assert!(f.eval("1 foo").is_ok());
            assert_eq!(f.stack(), [1, 1, 1]);
        }
        #[test]
        fn can_override_built_in_words() {
            let mut f = Forth::new();
            assert!(f.eval(": swap dup ;").is_ok());
            assert!(f.eval("1 swap").is_ok());
            assert_eq!(f.stack(), [1, 1]);
        }
        #[test]
        fn can_override_built_in_operators() {
            let mut f = Forth::new();
            assert!(f.eval(": + * ;").is_ok());
            assert!(f.eval("3 4 +").is_ok());
            assert_eq!(f.stack(), [12]);
        }
        #[test]
        fn can_use_different_words_with_the_same_name() {
            let mut f = Forth::new();
            assert!(f.eval(": foo 5 ;").is_ok());
            assert!(f.eval(": bar foo ;").is_ok());
            assert!(f.eval(": foo 6 ;").is_ok());
            assert!(f.eval("bar foo").is_ok());
            assert_eq!(f.stack(), [5, 6]);
        }
        #[test]
        fn can_define_word_that_uses_word_with_the_same_name() {
            let mut f = Forth::new();
            assert!(f.eval(": foo 10 ;").is_ok());
            assert!(f.eval(": foo foo 1 + ;").is_ok());
            assert!(f.eval("foo").is_ok());
            assert_eq!(f.stack(), [11]);
        }
        #[test]
        fn cannot_redefine_non_negative_numbers() {
            let mut f = Forth::new();
            assert_eq!(f.eval(": 1 2 ;"), Err(Error::InvalidWord));
        }
        #[test]
        fn cannot_redefine_negative_numbers() {
            let mut f = Forth::new();
            assert_eq!(f.eval(": -1 2 ;"), Err(Error::InvalidWord));
        }
        #[test]
        fn errors_if_executing_a_non_existent_word() {
            let mut f = Forth::new();
            assert_eq!(f.eval("foo"), Err(Error::UnknownWord));
        }
        #[test]
        fn only_defines_locally() {
            let mut f = Forth::new();
            assert!(f.eval(": + - ;").is_ok());
            assert!(f.eval("1 1 +").is_ok());
            assert_eq!(f.stack(), [0]);
            let mut f = Forth::new();
            assert!(f.eval("1 1 +").is_ok());
            assert_eq!(f.stack(), [2]);
        }
    }
    mod case_insensitivity {
        use super::super::*;
        #[test]
        fn dup_is_case_insensitive() {
            let mut f = Forth::new();
            assert!(f.eval("1 DUP Dup dup").is_ok());
            assert_eq!(f.stack(), [1, 1, 1, 1]);
        }
        #[test]
        fn drop_is_case_insensitive() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 3 4 DROP Drop drop").is_ok());
            assert_eq!(f.stack(), [1]);
        }
        #[test]
        fn swap_is_case_insensitive() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 SWAP 3 Swap 4 swap").is_ok());
            assert_eq!(f.stack(), [2, 3, 4, 1]);
        }
        #[test]
        fn over_is_case_insensitive() {
            let mut f = Forth::new();
            assert!(f.eval("1 2 OVER Over over").is_ok());
            assert_eq!(f.stack(), [1, 2, 1, 2, 1]);
        }
        #[test]
        fn user_defined_words_are_case_insensitive() {
            let mut f = Forth::new();
            assert!(f.eval(": foo dup ;").is_ok());
            assert!(f.eval("1 FOO Foo foo").is_ok());
            assert_eq!(f.stack(), [1, 1, 1, 1]);
        }
        #[test]
        fn definitions_are_case_insensitive() {
            let mut f = Forth::new();
            assert!(f.eval(": SWAP DUP Dup dup ;").is_ok());
            assert!(f.eval("1 swap").is_ok());
            assert_eq!(f.stack(), [1, 1, 1, 1]);
        }
    }
}
