use super::expr::{Env, Environment, Expr, Op, Var, C};
use super::parser_combinator::*;
use std::rc::Rc;

fn unsigned_number<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    one_or_more(any_char.pred(|c| c.0.is_numeric())).map(|chars| {
        let expr = Expr::Num(chars.iter().fold(C::new(0, 1), |s, c| {
            s * C::new(10, 1) + C::new(c.0.to_digit(10).expect("") as i64, 1)
        }));
        let env = chars.last().expect("").1;
        if let Some(p) = env.borrow().search_expr(&expr) {
            return (p, env);
        }
        let p = env.borrow_mut().extend_expr(expr);
        return (p, env);
    })
}
#[test]
fn number_parser() {
    let e = Environment::new();
    assert_eq!(
        Ok(("", &e, (Rc::new(Expr::Num(C::new(64, 1))), &e))),
        unsigned_number().parse("64", &e)
    );
    assert_eq!(
        Ok(("", &e, (Rc::new(Expr::Num(C::new(12333, 1))), &e))),
        unsigned_number().parse("12333", &e)
    );
    assert_eq!(
        Ok(("", &e, (Rc::new(Expr::Num(C::new(64, 1))), &e))),
        unsigned_number().parse("64", &e)
    );
    println!("{:?}", e);
    // assert_eq!(Ok(("", &e, Expr::Num(0))), unsigned_number().parse("0", &e));
    assert_eq!(Err(""), unsigned_number().parse("", &e));
    assert_eq!(Err("-123"), unsigned_number().parse("-123", &e));
}

fn variable<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    identifier.map(|(s, env)| {
        let option_var = env.borrow_mut().search_var(&s);
        if let Some(var) = option_var {
            let var_expr = env.borrow().exprs[&Expr::Var(var)].clone();
            (var_expr, env)
        } else {
            let v = env.borrow_mut().extend_var(s);
            let var_expr = env.borrow_mut().extend_expr(Expr::Var(v));
            (var_expr, env)
        }
    })
}

#[test]
fn variable_parser() {
    let e = Environment::new();
    // let (_inp, new_e, res1) = variable().parse("x1", &e).unwrap();
    // println!("{:?}", new_e);
    assert_eq!(
        Ok(("", &e, (Rc::new(Expr::Var(Var::new(0))), &e))),
        variable().parse("x1", &e)
    );
    assert_eq!(
        Ok(("", &e, (Rc::new(Expr::Var(Var::new(0))), &e))),
        variable().parse("x1", &e)
    );
    println!("{:?}", e);
}
// TODO: (expr)
fn primary<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    either(unsigned_number(), variable())
}

fn func<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    // either(whitespace_wrap(match_literal("cos")))
    // right(
    //     match_literal("("),
    //     left(whitespace_wrap(expr()), match_literal(")")),
    // )
    primary()
}

fn unary<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    zero_or_more(whitespace_wrap(
        any_char.pred(|(c, _e)| *c == '+' || *c == '-'),
    ))
    .and_then(|vec_c_r| {
        either(func(), primary()).map(move |(p, env)| {
            let expr;
            if vec_c_r.iter().filter(|(c, _e)| *c == '-').count() % 2 != 0 {
                expr = Expr::Neg(p);
                if let Some(p) = env.borrow().search_expr(&expr) {
                    return (p, env);
                }
                let p = env.borrow_mut().extend_expr(expr);
                return (p, env);
            } else {
                return (p, env);
            }
        })
    })
}

fn factor<'a>() -> impl Parser<'a, (Rc<Expr>, &'a Env)> {
    unary().and_then(|(one, _env)| {
        zero_or_more(right(whitespace_wrap(match_literal("^")), unary())).map(move |mut unaries| {
            let env = unaries.last().unwrap().1;
            if unaries.len() == 0 {
                (one.clone(), env)
            } else {
                let mut pow: Rc<Expr> = unaries.pop().unwrap().0;
                let expr = Expr::BinOp {
                    op: Op::Pow,
                    exp1: one.clone(),
                    exp2: pow,
                };
                let mut res = Rc::new(Expr::Num(C::new(std::i64::MAX, 1)));
                let optoin_expr = env.borrow().search_expr(&expr);
                if optoin_expr.is_some() {
                    res = optoin_expr.unwrap();
                } else {
                    res = env.borrow_mut().extend_expr(expr);
                }
                // 毎更新ごとに登録
                while let Some((una, _env)) = unaries.pop() {
                    let mut cur_expr = (*res).clone();
                    match &mut cur_expr {
                        Expr::BinOp { exp2, .. } => {
                            pow = Rc::new(Expr::BinOp {
                                op: Op::Pow,
                                exp1: una,
                                exp2: exp2.clone(),
                            });
                            *exp2 = pow;
                        }
                        _ => unreachable!(),
                    }
                    let optoin_expr = env.borrow().search_expr(&cur_expr);
                    if optoin_expr.is_some() {
                        res = optoin_expr.unwrap();
                    } else {
                        res = env.borrow_mut().extend_expr(cur_expr);
                    }
                }
                (res, env)
            }
        })
    })
}
#[test]
fn factor_parser() {
    let e = Environment::new();
    let expected_factor1 = Rc::new(Expr::BinOp {
        op: Op::Pow,
        exp1: Rc::new(Expr::Var(Var::new(0))),
        exp2: Rc::new(Expr::BinOp {
            op: Op::Pow,
            exp1: Rc::new(Expr::Num(C::new(3, 1))),
            exp2: Rc::new(Expr::Num(C::new(2, 1))),
        }),
    });
    assert_eq!(
        Ok(("", &e, (expected_factor1, &e))),
        factor().parse("x1 ^ 3 ^ 2", &e)
    );

    // let expected_factor2 = Expr::Pow {
    //     exp1: Box::new(Expr::Var(Var::new(0))),
    //     exp2: Box::new(P::Pow {
    //         exp1: Box::new(P::Num(3)),
    //         exp2: Box::new(P::Pow {
    //             exp1: Box::new(P::Num(3)),
    //             exp2: Box::new(P::Num(2)),
    //         }),
    //     }),
    // };
    // assert_eq!(Ok(("", expected_factor2)), factor().parse("x1 ^ 3 ^ 3 ^ 2"));
}

// fn term<'a>() -> impl Parser<'a, P> {
//     factor().and_then(|val| {
//         zero_or_more(right(whitespace_wrap(match_literal("*")), factor())).map(
//             move |mut factors| {
//                 if factors.len() == 0 {
//                     // closureのmove, borrowingまったくわかってない...
//                     val.clone()
//                 } else {
//                     let mut res = val.clone();
//                     factors.reverse();
//                     while let Some(f) = factors.pop() {
//                         res = P::Mul {
//                             exp1: Box::new(res),
//                             exp2: Box::new(f),
//                         };
//                     }
//                     res
//                 }
//             },
//         )
//     })
// }

// #[test]
// fn term_parser() {
//     let expected_term = P::Mul {
//         exp1: Box::new(P::Mul {
//             exp1: Box::new(P::Pow {
//                 exp1: Box::new(P::Var("x1".to_string())),
//                 exp2: Box::new(P::Num(3)),
//             }),
//             exp2: Box::new(P::Pow {
//                 exp1: Box::new(P::Var("y1".to_string())),
//                 exp2: Box::new(P::Num(2)),
//             }),
//         }),
//         exp2: Box::new(P::Pow {
//             exp1: Box::new(P::Var("x1".to_string())),
//             exp2: Box::new(P::Num(4)),
//         }),
//     };

//     assert_eq!(
//         Ok(("", expected_term)),
//         term().parse("x1 ^ 3 * y1 ^ 2 * x1 ^ 4")
//     );
// }
