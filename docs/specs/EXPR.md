# Expression library `forge-expr` (brief 052)

Wording: "formula", "variable", "function". Sources: standard operator precedence (as in
any scientific calculator); the function list is the planner's brief 052; the case-insensitive
spelling is observed in O9 [deformers-2.md](../parity/deformers-2.md) (`Sin((u+t)*2.0*PI)*0.2`,
case `formula/defaults`). Own design otherwise.

This is the **text** formula language used by the Formula deformer (MODIFIERS.md §17) and,
later, the Formula spline, effector and field. It is not the node-graph expression system of
`forge-nodes` (NODES.md; typed ports, a Rhai script node): that one evaluates graphs per
frame and has no one-line formula syntax with `^`, so the two do not overlap. Formula
features share only this crate.

## E1 Contract
- Crate `crates/expr`, package `forge-expr`, no dependencies, no `unsafe`, pure `f64`
  (deterministic: no clock, no random numbers).
- `Expr::parse(src, vars) -> Result<Expr, ParseError>`: `vars` are the variable names the
  caller binds, in order. `Expr::eval(&self, values: &[f64]) -> f64` takes one value per
  name. A missing value reads as NaN.
- `ParseError { offset, message }`: the byte offset of the offending token.

## E2 Syntax
- Numbers: `12`, `1.5`, `.5`, `1e-3`.
- Operators, tightest first: `^` (right-associative; the exponent may carry a sign, `2^-1`);
  unary `-` / `+` (looser than `^`: `-2^2 = -4`); `*` `/`; `+` `-`; comparisons `<` `<=` `>`
  `>=` `==` `!=`, whose value is `1` or `0` (left-associative). Parentheses group.
- Names are case-insensitive. Variables are the caller's; `PI` and `E` are constants (a
  variable of the same name wins).
- Functions (name, arguments): `sin cos tan asin acos atan sqrt abs exp floor ceil round
  sign` (1), `log` (natural; 1), `min max mod atan2 pow` (2), `clamp(x, lo, hi)` (3).
  `mod(a, b) = a − b·floor(a/b)` (the result has the sign of `b`). Angles are radians.
- A wrong argument count, an unknown name or function, or trailing input is a parse error.

## E3 Evaluation
Division by zero, `sqrt(-1)` and the like give infinity or NaN, as in IEEE 754; the caller
decides (the Formula deformer treats a non-finite result as "no change", MODIFIERS.md §17).

## E4 Tests
Unit tests in `crates/expr/src/lib.rs`: precedence, `^`, comparisons, the O9 formula text,
case-insensitivity, error offsets. Formula parity is MODIFIERS.md §17–19.

## E5 Degrees mode (brief 059)
`Expr::parse_degrees(src, vars)` compiles the same language with the angle functions in degrees:
`sin cos tan` take degrees, `asin acos atan atan2` return degrees. `PI` stays 3.14159... The
Formula field uses it: its default text `0.5 + sin( ( subfields + ( id / count ) + d ) * f * 360 ) * 0.5`
only matches the observed values with degrees (FIELDS.md F7, case `formula-defaults`).
