//! forge-expr: a small scalar formula language (docs/specs/EXPR.md).
//!
//! `Expr::parse("Sin((u+t)*2*PI)*0.2", &["u", "t"])` compiles a text once; `eval` runs it
//! with one value per declared variable. Pure and deterministic: no clock, no randomness,
//! no allocation per evaluation.

use std::fmt;

/// A parse error with the byte offset of the offending token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Func {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Sqrt,
    Abs,
    Exp,
    Log,
    Min,
    Max,
    Floor,
    Ceil,
    Round,
    Sign,
    Clamp,
    Mod,
    Pow,
}

impl Func {
    fn lookup(name: &str) -> Option<(Func, usize)> {
        Some(match name {
            "sin" => (Func::Sin, 1),
            "cos" => (Func::Cos, 1),
            "tan" => (Func::Tan, 1),
            "asin" => (Func::Asin, 1),
            "acos" => (Func::Acos, 1),
            "atan" => (Func::Atan, 1),
            "atan2" => (Func::Atan2, 2),
            "sqrt" => (Func::Sqrt, 1),
            "abs" => (Func::Abs, 1),
            "exp" => (Func::Exp, 1),
            "log" => (Func::Log, 1),
            "min" => (Func::Min, 2),
            "max" => (Func::Max, 2),
            "floor" => (Func::Floor, 1),
            "ceil" => (Func::Ceil, 1),
            "round" => (Func::Round, 1),
            "sign" => (Func::Sign, 1),
            "clamp" => (Func::Clamp, 3),
            "mod" => (Func::Mod, 2),
            "pow" => (Func::Pow, 2),
            _ => return None,
        })
    }

    /// `degrees`: the angle functions take and return degrees instead of radians (E5).
    fn call(self, a: &[f64], degrees: bool) -> f64 {
        let to_rad = |x: f64| if degrees { x.to_radians() } else { x };
        let from_rad = |x: f64| if degrees { x.to_degrees() } else { x };
        match self {
            Func::Sin => to_rad(a[0]).sin(),
            Func::Cos => to_rad(a[0]).cos(),
            Func::Tan => to_rad(a[0]).tan(),
            Func::Asin => from_rad(a[0].asin()),
            Func::Acos => from_rad(a[0].acos()),
            Func::Atan => from_rad(a[0].atan()),
            Func::Atan2 => from_rad(a[0].atan2(a[1])),
            Func::Sqrt => a[0].sqrt(),
            Func::Abs => a[0].abs(),
            Func::Exp => a[0].exp(),
            Func::Log => a[0].ln(),
            Func::Min => a[0].min(a[1]),
            Func::Max => a[0].max(a[1]),
            Func::Floor => a[0].floor(),
            Func::Ceil => a[0].ceil(),
            Func::Round => a[0].round(),
            Func::Sign => {
                if a[0] == 0.0 || a[0].is_nan() {
                    0.0
                } else {
                    a[0].signum()
                }
            }
            Func::Clamp => a[0].max(a[1]).min(a[2]),
            Func::Mod => a[0] - a[1] * (a[0] / a[1]).floor(),
            Func::Pow => a[0].powf(a[1]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bin {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Num(f64),
    Var(usize),
    Neg(Box<Node>),
    Bin(Bin, Box<Node>, Box<Node>),
    Call(Func, Vec<Node>),
}

impl Node {
    fn eval(&self, vars: &[f64], degrees: bool) -> f64 {
        match self {
            Node::Num(n) => *n,
            Node::Var(i) => vars.get(*i).copied().unwrap_or(f64::NAN),
            Node::Neg(a) => -a.eval(vars, degrees),
            Node::Bin(op, a, b) => {
                let (x, y) = (a.eval(vars, degrees), b.eval(vars, degrees));
                let flag = |c: bool| if c { 1.0 } else { 0.0 };
                match op {
                    Bin::Add => x + y,
                    Bin::Sub => x - y,
                    Bin::Mul => x * y,
                    Bin::Div => x / y,
                    Bin::Pow => x.powf(y),
                    Bin::Lt => flag(x < y),
                    Bin::Le => flag(x <= y),
                    Bin::Gt => flag(x > y),
                    Bin::Ge => flag(x >= y),
                    Bin::Eq => flag(x == y),
                    Bin::Ne => flag(x != y),
                }
            }
            Node::Call(f, args) => {
                let mut buf = [0.0; 3];
                for (slot, a) in buf.iter_mut().zip(args) {
                    *slot = a.eval(vars, degrees);
                }
                f.call(&buf[..args.len()], degrees)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    /// `<=`, `>=`, `==`, `!=`
    Cmp(Bin),
    End,
}

fn lex(src: &str) -> Result<Vec<(usize, Tok)>, ParseError> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == b'.' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                let mut j = i + 1;
                if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                    j += 1;
                }
                if j < b.len() && b[j].is_ascii_digit() {
                    while j < b.len() && b[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            let n = src[start..i].parse::<f64>().map_err(|_| ParseError {
                offset: start,
                message: format!("bad number `{}`", &src[start..i]),
            })?;
            out.push((start, Tok::Num(n)));
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push((start, Tok::Ident(src[start..i].to_ascii_lowercase())));
        } else if matches!(c, b'<' | b'>' | b'=' | b'!') && b.get(i + 1) == Some(&b'=') {
            let op = match c {
                b'<' => Bin::Le,
                b'>' => Bin::Ge,
                b'=' => Bin::Eq,
                _ => Bin::Ne,
            };
            out.push((i, Tok::Cmp(op)));
            i += 2;
        } else if matches!(
            c,
            b'+' | b'-' | b'*' | b'/' | b'^' | b'(' | b')' | b',' | b'<' | b'>'
        ) {
            out.push((i, Tok::Op(c as char)));
            i += 1;
        } else {
            return Err(ParseError {
                offset: i,
                message: format!(
                    "unexpected character `{}`",
                    src[i..].chars().next().unwrap()
                ),
            });
        }
    }
    out.push((src.len(), Tok::End));
    Ok(out)
}

struct Parser<'a> {
    toks: Vec<(usize, Tok)>,
    pos: usize,
    vars: &'a [&'a str],
}

impl Parser<'_> {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].1
    }

    fn offset(&self) -> usize {
        self.toks[self.pos].0
    }

    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].1.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            offset: self.offset(),
            message: message.into(),
        })
    }

    fn comparison(&mut self) -> Result<Node, ParseError> {
        let mut left = self.sum()?;
        loop {
            let op = match self.peek() {
                Tok::Op('<') => Bin::Lt,
                Tok::Op('>') => Bin::Gt,
                Tok::Cmp(b) => *b,
                _ => return Ok(left),
            };
            self.bump();
            let right = self.sum()?;
            left = Node::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn sum(&mut self) -> Result<Node, ParseError> {
        let mut left = self.product()?;
        loop {
            let op = match self.peek() {
                Tok::Op('+') => Bin::Add,
                Tok::Op('-') => Bin::Sub,
                _ => return Ok(left),
            };
            self.bump();
            let right = self.product()?;
            left = Node::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn product(&mut self) -> Result<Node, ParseError> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Tok::Op('*') => Bin::Mul,
                Tok::Op('/') => Bin::Div,
                _ => return Ok(left),
            };
            self.bump();
            let right = self.unary()?;
            left = Node::Bin(op, Box::new(left), Box::new(right));
        }
    }

    /// Unary minus binds looser than `^`: `-2^2 = -4`.
    fn unary(&mut self) -> Result<Node, ParseError> {
        match self.peek() {
            Tok::Op('-') => {
                self.bump();
                Ok(Node::Neg(Box::new(self.unary()?)))
            }
            Tok::Op('+') => {
                self.bump();
                self.unary()
            }
            _ => self.power(),
        }
    }

    /// `^` is right-associative; its exponent may carry a sign: `2^-1`.
    fn power(&mut self) -> Result<Node, ParseError> {
        let base = self.atom()?;
        if matches!(self.peek(), Tok::Op('^')) {
            self.bump();
            let exp = self.unary()?;
            return Ok(Node::Bin(Bin::Pow, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<Node, ParseError> {
        let at = self.offset();
        match self.bump() {
            Tok::Num(n) => Ok(Node::Num(n)),
            Tok::Op('(') => {
                let e = self.comparison()?;
                if !matches!(self.peek(), Tok::Op(')')) {
                    return self.err("expected `)`");
                }
                self.bump();
                Ok(e)
            }
            Tok::Ident(name) => {
                if matches!(self.peek(), Tok::Op('(')) {
                    let Some((f, arity)) = Func::lookup(&name) else {
                        return Err(ParseError {
                            offset: at,
                            message: format!("unknown function `{name}`"),
                        });
                    };
                    self.bump();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Tok::Op(')')) {
                        loop {
                            args.push(self.comparison()?);
                            if matches!(self.peek(), Tok::Op(',')) {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                    if !matches!(self.peek(), Tok::Op(')')) {
                        return self.err("expected `)`");
                    }
                    self.bump();
                    if args.len() != arity {
                        return Err(ParseError {
                            offset: at,
                            message: format!(
                                "`{name}` takes {arity} argument(s), got {}",
                                args.len()
                            ),
                        });
                    }
                    return Ok(Node::Call(f, args));
                }
                if let Some(i) = self.vars.iter().position(|v| v.eq_ignore_ascii_case(&name)) {
                    return Ok(Node::Var(i));
                }
                match name.as_str() {
                    "pi" => Ok(Node::Num(std::f64::consts::PI)),
                    "e" => Ok(Node::Num(std::f64::consts::E)),
                    _ => Err(ParseError {
                        offset: at,
                        message: format!("unknown name `{name}`"),
                    }),
                }
            }
            _ => Err(ParseError {
                offset: at,
                message: "expected a number, a name or `(`".to_string(),
            }),
        }
    }
}

/// A compiled formula.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    root: Node,
    degrees: bool,
}

impl Expr {
    /// Compiles `src`. `vars` names the variables, in the order [`Expr::eval`] takes their
    /// values; names and functions are case-insensitive, `PI` and `E` are constants (a
    /// variable of the same name wins).
    pub fn parse(src: &str, vars: &[&str]) -> Result<Expr, ParseError> {
        let mut p = Parser {
            toks: lex(src)?,
            pos: 0,
            vars,
        };
        let root = p.comparison()?;
        if !matches!(p.peek(), Tok::End) {
            return p.err("unexpected input after the formula");
        }
        Ok(Expr {
            root,
            degrees: false,
        })
    }

    /// Like [`Expr::parse`], but `sin cos tan` take degrees and `asin acos atan atan2` return
    /// degrees (the Formula field, E5).
    pub fn parse_degrees(src: &str, vars: &[&str]) -> Result<Expr, ParseError> {
        Ok(Expr {
            degrees: true,
            ..Expr::parse(src, vars)?
        })
    }

    /// The value with `vars[i]` bound to the i-th declared name. May be NaN or infinite
    /// (division by zero, `sqrt` of a negative); callers decide what that means.
    pub fn eval(&self, vars: &[f64]) -> f64 {
        self.root.eval(vars, self.degrees)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> f64 {
        Expr::parse(src, &[]).unwrap().eval(&[])
    }

    #[test]
    fn degrees_mode_converts_the_angle_functions() {
        let e = Expr::parse_degrees("sin(30) + cos(60) + asin(0.5) / 30", &[]).unwrap();
        assert!((e.eval(&[]) - 2.0).abs() < 1e-12);
        // radians stay the default
        assert!((run("sin(PI / 6)") - 0.5).abs() < 1e-12);
        assert!((Expr::parse_degrees("atan2(1, 1)", &[]).unwrap().eval(&[]) - 45.0).abs() < 1e-12);
    }

    #[test]
    fn precedence_and_power() {
        assert_eq!(run("1+2*3"), 7.0);
        assert_eq!(run("(1+2)*3"), 9.0);
        assert_eq!(run("-2^2"), -4.0);
        assert_eq!(run("2^3^2"), 512.0);
        assert_eq!(run("2^-1"), 0.5);
        assert_eq!(run("10-4-3"), 3.0);
        assert_eq!(run("8/4/2"), 1.0);
    }

    #[test]
    fn comparisons_give_one_or_zero() {
        assert_eq!(run("1<2"), 1.0);
        assert_eq!(run("2<=1"), 0.0);
        assert_eq!(run("3==3"), 1.0);
        assert_eq!(run("3!=3"), 0.0);
        assert_eq!(run("1+1>1"), 1.0);
    }

    #[test]
    fn functions_are_case_insensitive() {
        let e = Expr::parse("Sin((u+t)*2.0*PI)*0.2", &["u", "t"]).unwrap();
        assert!((e.eval(&[0.25, 0.0]) - 0.2).abs() < 1e-15);
        assert_eq!(run("MAX(1,MIN(5,3))"), 3.0);
        assert_eq!(run("clamp(7,0,2)"), 2.0);
        assert_eq!(run("mod(-1,3)"), 2.0);
        assert_eq!(run("floor(-0.5)+ceil(0.5)"), 0.0);
        assert_eq!(run("abs(-3)+sqrt(16)"), 7.0);
    }

    #[test]
    fn variables_and_numbers() {
        let e = Expr::parse("X*2+y", &["x", "Y"]).unwrap();
        assert_eq!(e.eval(&[3.0, 1.0]), 7.0);
        assert_eq!(run("1.5e2"), 150.0);
        assert_eq!(run(".5"), 0.5);
        assert!(run("1/0").is_infinite());
    }

    #[test]
    fn errors_name_the_place() {
        for bad in [
            "", "1+", "(1", "foo(1)", "sin(1,2)", "q", "1 2", "1 $ 2", "1..2",
        ] {
            assert!(Expr::parse(bad, &[]).is_err(), "{bad:?} must not parse");
        }
        let e = Expr::parse("1+q", &[]).unwrap_err();
        assert_eq!(e.offset, 2);
    }
}
