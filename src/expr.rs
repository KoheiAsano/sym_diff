pub use num_rational::Rational64;
pub use num_traits::identities::{One, Zero};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type C = Rational64;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Var {
    id: usize,
}

impl Var {
    pub fn new(id: usize) -> Self {
        Var { id }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bop {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Uop {
    Sin,
    Cos,
    Tan,
    Log,
    Exp,
    Neg,
}

// reduceの都合でBinOpを最後に
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    UnOp {
        op: Uop,
        exp: Rc<Expr>,
    },
    Sin(Rc<Expr>),
    Cos(Rc<Expr>),
    Tan(Rc<Expr>),
    Log(Rc<Expr>),
    Exp(Rc<Expr>),
    Neg(Rc<Expr>),
    Var(Var),
    Num(C),
    BinOp {
        op: Bop,
        exp1: Rc<Expr>,
        exp2: Rc<Expr>,
    },
}

impl Expr {
    fn new_unop(one: Rc<Expr>, op: Uop) -> Rc<Expr> {
        Rc::new(Expr::UnOp { op, exp: one })
    }

    fn new_binop(one: Rc<Expr>, other: Rc<Expr>, op: Bop) -> Rc<Expr> {
        Rc::new(Expr::BinOp {
            op,
            exp1: one,
            exp2: other,
        })
    }

    fn new_num(n: i64) -> Rc<Expr> {
        Rc::new(Expr::Num(C::new(n, 1)))
    }

    fn new_num_from_op(left: Rc<Expr>, right: Rc<Expr>, op: Bop) -> Rc<Expr> {
        match *left {
            Expr::Num(n) => match *right {
                Expr::Num(m) => match op {
                    Bop::Add => Rc::new(Expr::Num(n + m)),
                    Bop::Sub => Rc::new(Expr::Num(n - m)),
                    Bop::Mul => Rc::new(Expr::Num(n * m)),
                    Bop::Div => Rc::new(Expr::Num(n / m)),
                    // Powは無理(無理数)
                    Bop::Pow => unimplemented!(),
                },
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn is_const(&self) -> bool {
        match self {
            Expr::Num(_n) => true,
            _ => false,
        }
    }

    fn is_zero(&self) -> bool {
        match self {
            Expr::Num(n) => n.is_zero(),
            _ => false,
        }
    }

    fn is_one(&self) -> bool {
        match self {
            Expr::Num(n) => *n == C::one(),
            _ => false,
        }
    }

    fn is_minus_one(&self) -> bool {
        match self {
            Expr::Num(n) => *n == -C::one(),
            _ => false,
        }
    }

    pub fn diff(&self, v: &str, e: &Env) -> Rc<Expr> {
        match e.borrow().search_var(&String::from(v)) {
            Some(v) => self.diff_internal(v, e),
            None => {
                // unreachable!();
                Rc::new(Expr::Num(C::new(0, 1)))
            }
        }
    }
    fn diff_internal(&self, v: Var, e: &Env) -> Rc<Expr> {
        match self {
            // TODO: Uop
            Expr::UnOp { op, exp: inexp } => match op {
                Sin => Expr::new_binop(
                    Expr::new_unop(inexp.clone(), Uop::Cos),
                    inexp.diff_internal(v, e),
                    Bop::Mul,
                ),
                Cos => {
                    let inner = Expr::new_binop(
                        Expr::new_unop(inexp.clone(), Uop::Sin),
                        inexp.diff_internal(v, e),
                        Bop::Mul,
                    );
                    Expr::new_unop(inner, Uop::Neg)
                }
                Tan => {
                    let factor = Expr::new_binop(
                        Expr::new_unop(inexp.clone(), Uop::Cos),
                        Expr::new_num(2),
                        Bop::Pow,
                    );
                    Expr::new_binop(factor, inexp.diff_internal(v, e), Bop::Mul)
                }
                Log => {
                    let factor = Expr::new_binop(Expr::new_num(1), inexp.clone(), Bop::Div);
                    Expr::new_binop(factor, inexp.diff_internal(v, e), Bop::Mul)
                }
                Exp => Expr::new_binop(Rc::new(self.clone()), inexp.diff_internal(v, e), Bop::Mul),
                Neg => Expr::new_unop(inexp.diff_internal(v, e), Uop::Neg),
            },
            Expr::BinOp { op, exp1, exp2 } => {
                let factor_left;
                let factor_right;
                match op {
                    Bop::Add => {
                        factor_left = Expr::new_num(1);
                        factor_right = Expr::new_num(1);
                    }
                    Bop::Sub => {
                        factor_left = Expr::new_num(1);
                        factor_right = Expr::new_num(-1);
                    }
                    Bop::Mul => {
                        factor_left = exp2.clone();
                        factor_right = exp1.clone();
                    }
                    Bop::Div => {
                        factor_left = Expr::new_binop(Expr::new_num(1), exp2.clone(), Bop::Div);
                        let deno = Expr::new_binop(exp2.clone(), Expr::new_num(2), Bop::Pow);
                        factor_right =
                            Rc::new(Expr::Neg(Expr::new_binop(exp1.clone(), deno, Bop::Div)));
                    }
                    Bop::Pow => {
                        let factor1 = Expr::new_binop(exp2.clone(), exp1.clone(), Bop::Div);
                        let factor2 = Rc::new(Expr::Log(exp1.clone()));
                        factor_left = Expr::new_binop(factor1, Rc::new(self.clone()), Bop::Mul);
                        factor_right = Expr::new_binop(factor2, Rc::new(self.clone()), Bop::Mul);
                    }
                }
                let left = Expr::new_binop(factor_left, exp1.diff_internal(v, e), Bop::Mul);
                let right = Expr::new_binop(factor_right, exp2.diff_internal(v, e), Bop::Mul);
                Expr::new_binop(left, right, Bop::Add)
            }
            Expr::Sin(inexp) => Expr::new_binop(
                Rc::new(Expr::Cos(inexp.clone())),
                inexp.diff_internal(v, e),
                Bop::Mul,
            ),
            Expr::Cos(inexp) => {
                let inner = Expr::new_binop(
                    Rc::new(Expr::Sin(inexp.clone())),
                    inexp.diff_internal(v, e),
                    Bop::Mul,
                );
                Rc::new(Expr::Neg(inner))
            }
            Expr::Tan(inexp) => {
                let factor = Expr::new_binop(
                    Rc::new(Expr::Cos(inexp.clone())),
                    Expr::new_num(2),
                    Bop::Pow,
                );
                Expr::new_binop(factor, inexp.diff_internal(v, e), Bop::Mul)
            }
            Expr::Log(inexp) => {
                let factor = Expr::new_binop(Expr::new_num(1), inexp.clone(), Bop::Div);
                Expr::new_binop(factor, inexp.diff_internal(v, e), Bop::Mul)
            }
            Expr::Exp(inexp) => {
                Expr::new_binop(Rc::new(self.clone()), inexp.diff_internal(v, e), Bop::Mul)
            }
            Expr::Neg(inexp) => Rc::new(Expr::Neg(inexp.diff_internal(v, e))),
            Expr::Var(vt) => {
                if *vt == v {
                    Expr::new_num(1)
                } else {
                    Expr::new_num(0)
                }
            }
            Expr::Num(_n) => Expr::new_num(0),
        }
    }

    pub fn reduce(&self, e: &Env) -> Rc<Expr> {
        self.reduce_internal(e)
    }
    fn reduce_internal(&self, e: &Env) -> Rc<Expr> {
        match self {
            Expr::UnOp { op, exp: inexp } => match op {
                Sin => {
                    let inexp = inexp.reduce(e);
                    // rationalなので, 完全な定数化は無理
                    if inexp.is_zero() {
                        Expr::new_num(0)
                    } else {
                        Rc::new(Expr::Sin(inexp))
                    }
                }
                Cos => {
                    let inexp = inexp.reduce(e);
                    if inexp.is_zero() {
                        Expr::new_num(1)
                    } else {
                        Rc::new(Expr::Cos(inexp))
                    }
                }
                Tan => {
                    let inexp = inexp.reduce(e);
                    if inexp.is_zero() {
                        Expr::new_num(0)
                    } else {
                        Rc::new(Expr::Tan(inexp))
                    }
                }
                Log => {
                    // TODO: log x ^ e = e * log x ??
                    // TODO: log e = 1 ??
                    let inexp = inexp.reduce(e);
                    if inexp.is_one() {
                        Expr::new_num(0)
                    } else {
                        Rc::new(Expr::Log(inexp))
                    }
                }
                Exp => {
                    // TODO: e log x = x
                    let inexp = inexp.reduce(e);
                    if inexp.is_zero() {
                        Expr::new_num(1)
                    } else {
                        Rc::new(Expr::Exp(inexp))
                    }
                }
                Neg => {
                    let inexp = inexp.reduce(e);
                    match *inexp {
                        Expr::Num(n) => Rc::new(Expr::Num(-n)),
                        _ => Rc::new(Expr::Neg(inexp)),
                    }
                    // Rc::new(Expr::Neg(inexp))
                }
            },
            Expr::BinOp { op, exp1, exp2 } => {
                let (left, right) = (exp1.reduce(e), exp2.reduce(e));
                match op {
                    Bop::Add => {
                        // TODO: plus + plus以外をsubにする？
                        if left.is_zero() {
                            right
                        } else if right.is_zero() {
                            left
                        } else if left.is_const() && right.is_const() {
                            Expr::new_num_from_op(left, right, Bop::Add)
                        } else if Rc::ptr_eq(&left, &right) {
                            Expr::new_binop(Expr::new_num(2), left, Bop::Mul)
                        } else {
                            Expr::new_binop(left, right, Bop::Add)
                        }
                    }
                    Bop::Sub => {
                        if left.is_zero() {
                            right
                        } else if right.is_zero() {
                            left
                        } else if left.is_const() && right.is_const() {
                            Expr::new_num_from_op(left, right, Bop::Sub)
                        } else if Rc::ptr_eq(&left, &right) {
                            Expr::new_num(0)
                        } else {
                            Expr::new_binop(left, right, Bop::Sub)
                        }
                    }
                    Bop::Mul => {
                        if left.is_zero() || right.is_zero() {
                            Expr::new_num(0)
                        } else if left.is_one() {
                            right
                        } else if left.is_minus_one() {
                            Rc::new(Expr::Neg(right))
                        } else if right.is_one() {
                            left
                        } else if right.is_minus_one() {
                            Rc::new(Expr::Neg(left))
                        } else if left.is_const() && right.is_const() {
                            Expr::new_num_from_op(left, right, Bop::Mul)
                        } else if Rc::ptr_eq(&left, &right) {
                            Expr::new_binop(Expr::new_num(2), left, Bop::Pow)
                        } else {
                            Expr::new_binop(left, right, Bop::Mul)
                        }
                    }
                    Bop::Div => {
                        if left.is_zero() || right.is_zero() {
                            panic!("zero div")
                        } else if right.is_one() {
                            left
                        } else if right.is_minus_one() {
                            Rc::new(Expr::Neg(left))
                        } else if left.is_const() && right.is_const() {
                            Expr::new_num_from_op(left, right, Bop::Div)
                        } else if Rc::ptr_eq(&left, &right) {
                            Expr::new_num(1)
                        } else {
                            Expr::new_binop(left, right, Bop::Div)
                        }
                    }
                    Bop::Pow => {
                        // TODO: e log x = x
                        if left.is_zero() {
                            Expr::new_num(0)
                        } else if right.is_zero() {
                            Expr::new_num(1)
                        } else if left.is_one() {
                            Expr::new_num(1)
                        } else if right.is_one() {
                            left
                        } else {
                            Expr::new_binop(left, right, Bop::Pow)
                        }
                    }
                }
            }
            Expr::Sin(inexp) => {
                let inexp = inexp.reduce(e);
                // rationalなので, 完全な定数化は無理
                if inexp.is_zero() {
                    Expr::new_num(0)
                } else {
                    Rc::new(Expr::Sin(inexp))
                }
            }
            Expr::Cos(inexp) => {
                let inexp = inexp.reduce(e);
                if inexp.is_zero() {
                    Expr::new_num(1)
                } else {
                    Rc::new(Expr::Cos(inexp))
                }
            }
            Expr::Tan(inexp) => {
                let inexp = inexp.reduce(e);
                if inexp.is_zero() {
                    Expr::new_num(0)
                } else {
                    Rc::new(Expr::Tan(inexp))
                }
            }
            Expr::Log(inexp) => {
                // TODO: log x ^ e = e * log x ??
                // TODO: log e = 1 ??
                let inexp = inexp.reduce(e);
                if inexp.is_one() {
                    Expr::new_num(0)
                } else {
                    Rc::new(Expr::Log(inexp))
                }
            }
            Expr::Exp(inexp) => {
                // TODO: e log x = x
                let inexp = inexp.reduce(e);
                if inexp.is_zero() {
                    Expr::new_num(1)
                } else {
                    Rc::new(Expr::Exp(inexp))
                }
            }
            Expr::Neg(inexp) => {
                let inexp = inexp.reduce(e);
                match *inexp {
                    Expr::Num(n) => Rc::new(Expr::Num(-n)),
                    _ => Rc::new(Expr::Neg(inexp)),
                }
                // Rc::new(Expr::Neg(inexp))
            }
            Expr::Var(vt) => Rc::new(Expr::Var(*vt)),
            Expr::Num(n) => Rc::new(Expr::Num(*n)),
        }
    }

    pub fn print(&self, e: &Env) {
        self.print_internal(e);
    }

    fn print_func(&self, name: &str, e: &Env) {
        print!("{}(", name);
        self.print_internal(e);
        print!(")");
    }

    fn print_internal(&self, e: &Env) {
        match self {
            Expr::UnOp { op, exp: inexp } => match op {
                Sin => {
                    inexp.print_func("sin", e);
                }
                Cos => {
                    inexp.print_func("cos", e);
                }
                Tan => {
                    inexp.print_func("tan", e);
                }
                Log => {
                    inexp.print_func("log", e);
                }
                Exp => {
                    inexp.print_func("exp", e);
                }
                Neg => {
                    print!("-");
                    inexp.print_internal(e);
                }
            },
            Expr::BinOp { op, exp1, exp2 } => {
                let ops;
                match op {
                    Bop::Add => {
                        ops = String::from("+");
                    }
                    Bop::Sub => {
                        ops = String::from("-");
                    }
                    Bop::Mul => {
                        ops = String::from("*");
                    }
                    Bop::Div => {
                        ops = String::from("/");
                    }
                    Bop::Pow => {
                        ops = String::from("^");
                    }
                }
                exp1.print_internal(e);
                print!("{}", ops);
                exp2.print_internal(e);
            }
            Expr::Sin(inexp) => {
                inexp.print_func("sin", e);
            }
            Expr::Cos(inexp) => {
                inexp.print_func("cos", e);
            }
            Expr::Tan(inexp) => {
                inexp.print_func("tan", e);
            }
            Expr::Log(inexp) => {
                inexp.print_func("log", e);
            }
            Expr::Exp(inexp) => {
                inexp.print_func("exp", e);
            }
            Expr::Neg(inexp) => {
                print!("-");
                inexp.print_internal(e);
            }
            Expr::Var(vt) => {
                print!("{}", e.borrow().vars[vt]);
            }
            Expr::Num(n) => {
                print!("{}", n);
            }
        }
    }
}

// 文字列からの検索, 変数からの検索を両方早くしたいんだけど, Mapにすると重そう
// かといってそうじゃなければ変数の数だけはかかる？
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub vars: HashMap<Var, String>,
    pub rev_vars: HashMap<String, Var>,
    pub exprs: HashMap<Expr, Rc<Expr>>,
}

pub type Env = RefCell<Environment>;

impl Environment {
    pub fn new() -> Env {
        RefCell::new(Environment {
            vars: HashMap::new(),
            rev_vars: HashMap::new(),
            exprs: HashMap::new(),
        })
    }

    pub fn extend_var(&mut self, var_str: String) -> Var {
        let v = Var::new(self.vars.len());
        self.vars.insert(v, var_str.clone());
        self.rev_vars.insert(var_str, v);
        v
    }

    pub fn search_var(&self, var_str: &String) -> Option<Var> {
        match self.rev_vars.get(var_str) {
            Some(v) => Some(*v),
            None => None,
        }
    }

    pub fn extend_expr(&mut self, e: Expr) -> Rc<Expr> {
        match self.search_expr(&e) {
            Some(ptr_e) => ptr_e,
            None => {
                let ptr_e = Rc::new(e.clone());
                self.exprs.insert(e, ptr_e.clone());
                ptr_e
            }
        }
    }

    fn search_expr(&self, e: &Expr) -> Option<Rc<Expr>> {
        match self.exprs.get(&e) {
            Some(ptr) => Some(ptr.clone()),
            None => None,
        }
    }

    pub fn remove_expr(&mut self, e: &Expr) {
        match self.exprs.remove(e) {
            Some(_) => (),
            None => unreachable!(),
        }
    }
}
