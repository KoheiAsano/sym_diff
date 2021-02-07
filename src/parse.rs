use super::expr::{Bop, Env, Environment, Expr, Uop, Var, C};
pub use super::parser_combinator::*;
use std::rc::Rc;

fn unsigned_number<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    one_or_more(any_char.pred(|c| c.0.is_numeric())).map(|chars| {
        let env = chars.last().expect("").1;
        (
            Expr::new_num(
                chars
                    .iter()
                    .fold(0, |s, c| s * 10 + c.0.to_digit(10).expect("") as i64),
                env,
            ),
            env,
        )
    })
}
#[test]
fn number_parser() {
    let e = &Environment::new();
    assert_eq!(
        Ok(("", e, (Expr::new_num(64, e), e))),
        unsigned_number().parse("64", e)
    );
    assert_eq!(
        Ok(("", e, (Expr::new_num(12333, e), e))),
        unsigned_number().parse("12333", e)
    );
    assert_eq!(Err(""), unsigned_number().parse("", e));
    assert_eq!(Err("-123"), unsigned_number().parse("-123", e));
}

fn variable<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    identifier.map(|(s, env)| {
        let v = env.borrow_mut().extend_var(s);
        (Expr::new_var(v.id, env), env)
    })
}
pub fn variables<'a>() -> impl Parser<'a, Vec<Rc<Expr>>> {
    one_or_more(whitespace_wrap(variable())).map(|v| v.into_iter().map(|(v, _e)| v).collect())
}

#[test]
fn variable_parser() {
    let e = &Environment::new();
    assert_eq!(
        Ok(("", e, (Expr::new_var(0, e), e))),
        variable().parse("x1", e)
    );
    assert_eq!(
        Ok(("", e, (Expr::new_var(0, e), e))),
        variable().parse("x1", e)
    );
    println!("{:?}", e);
}

fn primary<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    either(unsigned_number(), either(variable(), parenthesized_expr()))
}

fn func<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    pair(
        one_of(vec!["sin", "cos", "tan", "log", "exp"]),
        parenthesized_expr(),
    )
    .map(|(name, (exp, env))| {
        let op;
        match name {
            "sin" => {
                op = Uop::Sin;
            }
            "cos" => {
                op = Uop::Cos;
            }
            "tan" => {
                op = Uop::Tan;
            }
            "log" => {
                op = Uop::Log;
            }
            "exp" => {
                op = Uop::Exp;
            }
            _ => unimplemented!(),
        }
        (Expr::new_unop(op, exp, env), env)
    })
}

fn unary<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    zero_or_more(whitespace_wrap(
        any_char.pred(|(c, _e)| *c == '+' || *c == '-'),
    ))
    .and_then(|vec_c_r| {
        either(func(), primary()).map(move |(mut res, env)| {
            if vec_c_r.iter().filter(|(c, _e)| *c == '-').count() % 2 != 0 {
                res = Expr::new_unop(Uop::Neg, res, env);
                return (res, env);
            } else {
                return (res, env);
            }
        })
    })
}

#[test]
fn unary_parser() {
    let e = &Environment::new();
    let exptcted_expr = Expr::new_unop(Uop::Log, Expr::new_var(0, e), e);
    assert_eq!(
        Ok(("", e, (exptcted_expr, e))),
        unary().parse("  - + - + log(x)", e)
    );
}

fn factor<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    unary().and_then(|(one, env)| {
        zero_or_more(right(whitespace_wrap(match_literal("^")), unary())).map(move |mut unaries| {
            if unaries.len() == 0 {
                (one.clone(), env)
            } else {
                let env = unaries.last().unwrap().1;
                let mut res = unaries.pop().unwrap().0;
                while let Some((una, _env)) = unaries.pop() {
                    res = Expr::new_binop(Bop::Pow, una, res, env);
                }
                res = Expr::new_binop(Bop::Pow, one.clone(), res, env);
                (res, env)
            }
        })
    })
}
#[test]
fn factor_parser() {
    let e = &Environment::new();
    let expected_factor1 = Expr::new_binop(
        Bop::Pow,
        Expr::new_var(0, e),
        Expr::new_binop(Bop::Pow, Expr::new_num(3, e), Expr::new_num(2, e), e),
        e,
    );
    assert_eq!(
        Ok(("", e, (expected_factor1, e))),
        factor().parse("x1 ^ 3 ^ 2", e)
    );
}

fn term<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    factor().and_then(|(one, env)| {
        zero_or_more(whitespace_wrap(pair(
            whitespace_wrap(any_char.pred(|(c, _e)| *c == '*' || *c == '/')),
            factor(),
        )))
        .map(move |mut factors| {
            if factors.len() == 0 {
                (one.clone(), env)
            } else {
                let env = factors.last().unwrap().1 .1;
                let mut res = one.clone();
                factors.reverse();
                while let Some(((c, _e1), (f, _e2))) = factors.pop() {
                    match c {
                        '*' => {
                            res = Expr::new_binop(Bop::Mul, res, f, env);
                        }
                        '/' => {
                            res = Expr::new_binop(Bop::Div, res, f, env);
                        }
                        _ => unreachable!(),
                    }
                }
                (res, env)
            }
        })
    })
}

#[test]
fn term_parser() {
    let e = &Environment::new();
    let expected_term = Expr::new_binop(
        Bop::Mul,
        Expr::new_binop(
            Bop::Mul,
            Expr::new_binop(Bop::Pow, Expr::new_var(0, e), Expr::new_num(3, e), e),
            Expr::new_binop(Bop::Pow, Expr::new_var(1, e), Expr::new_num(2, e), e),
            e,
        ),
        Expr::new_binop(Bop::Pow, Expr::new_var(0, e), Expr::new_num(4, e), e),
        e,
    );
    assert_eq!(
        Ok(("", e, (expected_term, e))),
        term().parse("x1 ^ 3 * y1 ^ 2 * x1 ^ 4", e)
    );
}

pub fn expr<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    whitespace_wrap(term()).and_then(|(one, env)| {
        zero_or_more(whitespace_wrap(pair(
            whitespace_wrap(any_char.pred(|(c, _e)| *c == '+' || *c == '-')),
            term(),
        )))
        .map(move |mut terms| {
            if terms.len() == 0 {
                (one.clone(), env)
            } else {
                let env = terms.last().unwrap().1 .1;
                let mut res = one.clone();
                // わざわざReverseしなくていいよね...
                terms.reverse();
                while let Some(((c, _e1), (t, _e2))) = terms.pop() {
                    match c {
                        '+' => {
                            res = Expr::new_binop(Bop::Add, res, t, env);
                        }
                        '-' => {
                            res = Expr::new_binop(Bop::Sub, res, t, env);
                        }
                        _ => unreachable!(),
                    }
                }
                (res, env)
            }
        })
    })
}

fn parenthesized_expr<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    right(
        match_literal("("),
        left(whitespace_wrap(expr()), match_literal(")")),
    )
}

#[test]
fn expr_parser() {
    let e = &Environment::new();

    let num = Expr::new_binop(
        Bop::Add,
        Expr::new_num(2, e),
        Expr::new_unop(Uop::Log, Expr::new_var(0, e), e),
        e,
    );
    let deno = Expr::new_unop(Uop::Tan, Expr::new_var(0, e), e);
    let expr1 = Expr::new_binop(
        Bop::Pow,
        Expr::new_binop(Bop::Div, num, deno, e),
        Expr::new_var(1, e),
        e,
    );
    let exptcted_expr = Expr::new_binop(
        Bop::Add,
        expr1,
        Expr::new_unop(Uop::Sin, Expr::new_var(1, e), e),
        e,
    );
    assert_eq!(
        Ok(("", e, (exptcted_expr, e))),
        expr().parse("( ( 2 + log(x) ) / tan(x) ) ^ y  + sin(y) ", e)
    );
    let res = expr().parse("1 / tan( x )", e);

    match res {
        Ok((_, _, (expr, env))) => {
            let d = expr.diff("x", env).reduce(env);
            d.print(env);
            env.borrow_mut().clean();
            println!("{}", d.eval("x", &vec![std::f64::consts::FRAC_PI_2], env));
        }
        Err(_) => panic!(""),
    }
}
