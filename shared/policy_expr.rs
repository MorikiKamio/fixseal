use serde::{Deserialize, Serialize};

// Wire order is op, then args. UInt64 constants are canonical decimal strings.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "op",
    content = "args",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum UIntExpr {
    Limit,
    Spent,
    Amount,
    Const(String),
    Add(Box<UIntExpr>, Box<UIntExpr>),
    Sub(Box<UIntExpr>, Box<UIntExpr>),
    If(Box<BoolExpr>, Box<UIntExpr>, Box<UIntExpr>),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "op",
    content = "args",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum BoolExpr {
    True,
    False,
    Le(UIntExpr, UIntExpr),
    Lt(UIntExpr, UIntExpr),
    Eq(UIntExpr, UIntExpr),
    And(Box<BoolExpr>, Box<BoolExpr>),
    Or(Box<BoolExpr>, Box<BoolExpr>),
    Not(Box<BoolExpr>),
    If(Box<BoolExpr>, Box<BoolExpr>, Box<BoolExpr>),
}

impl UIntExpr {
    fn eval(&self, limit: u64, spent: u64, amount: u64) -> u64 {
        let eval = |e: &Self| e.eval(limit, spent, amount);
        match self {
            Self::Limit => limit,
            Self::Spent => spent,
            Self::Amount => amount,
            Self::Const(n) => n.parse().expect("validated UInt64 constant"),
            Self::Add(a, b) => eval(a).wrapping_add(eval(b)),
            Self::Sub(a, b) => eval(a).wrapping_sub(eval(b)),
            Self::If(c, a, b) => eval(if c.eval(limit, spent, amount) { a } else { b }),
        }
    }

    fn validate(&self, depth: usize) -> Result<(), &'static str> {
        if depth > 48 {
            return Err("Policy expression too deep");
        }
        match self {
            Self::Const(n) if n.parse::<u64>().map_or(true, |v| v.to_string() != *n) => {
                Err("Invalid UInt64 constant")
            }
            Self::Add(a, b) | Self::Sub(a, b) => {
                a.validate(depth + 1)?;
                b.validate(depth + 1)
            }
            Self::If(c, a, b) => {
                c.validate(depth + 1)?;
                a.validate(depth + 1)?;
                b.validate(depth + 1)
            }
            _ => Ok(()),
        }
    }
}

impl BoolExpr {
    pub fn parse(json: &str) -> Result<Self, &'static str> {
        if json.len() > 32768 {
            return Err("Policy expression too large");
        }
        let expr: Self = serde_json::from_str(json).map_err(|_| "Invalid policy expression")?;
        expr.validate(0)?;
        Ok(expr)
    }

    pub fn canonical(&self) -> String {
        serde_json::to_string(self).expect("serializable policy expression")
    }

    pub fn vulnerable() -> Self {
        Self::Le(
            UIntExpr::Add(Box::new(UIntExpr::Spent), Box::new(UIntExpr::Amount)),
            UIntExpr::Limit,
        )
    }

    pub fn eval(&self, limit: u64, spent: u64, amount: u64) -> bool {
        let uint = |e: &UIntExpr| e.eval(limit, spent, amount);
        let eval = |e: &Self| e.eval(limit, spent, amount);
        match self {
            Self::True => true,
            Self::False => false,
            Self::Le(a, b) => uint(a) <= uint(b),
            Self::Lt(a, b) => uint(a) < uint(b),
            Self::Eq(a, b) => uint(a) == uint(b),
            Self::And(a, b) => eval(a) && eval(b),
            Self::Or(a, b) => eval(a) || eval(b),
            Self::Not(a) => !eval(a),
            Self::If(c, a, b) => eval(if eval(c) { a } else { b }),
        }
    }

    fn validate(&self, depth: usize) -> Result<(), &'static str> {
        if depth > 48 {
            return Err("Policy expression too deep");
        }
        match self {
            Self::Le(a, b) | Self::Lt(a, b) | Self::Eq(a, b) => {
                a.validate(depth + 1)?;
                b.validate(depth + 1)
            }
            Self::And(a, b) | Self::Or(a, b) => {
                a.validate(depth + 1)?;
                b.validate(depth + 1)
            }
            Self::Not(a) => a.validate(depth + 1),
            Self::If(c, a, b) => {
                c.validate(depth + 1)?;
                a.validate(depth + 1)?;
                b.validate(depth + 1)
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoolExpr as B, UIntExpr as U};

    #[test]
    fn wrapping_arithmetic_and_all_operations() {
        assert!(B::vulnerable().eval(1000, 600, u64::MAX - 499));
        let add = U::Add(Box::new(U::Amount), Box::new(U::Const("1".into())));
        let sub = U::Sub(Box::new(U::Const("0".into())), Box::new(U::Amount));
        assert!(B::Eq(add, U::Const("0".into())).eval(0, 0, u64::MAX));
        assert!(B::Eq(sub, U::Const(u64::MAX.to_string())).eval(0, 0, 1));
        let lt = B::Lt(U::Spent, U::Limit);
        let condition = B::And(Box::new(lt), Box::new(B::Not(Box::new(B::False))));
        let choice = U::If(
            Box::new(condition.clone()),
            Box::new(U::Amount),
            Box::new(U::Spent),
        );
        assert!(B::Eq(choice, U::Amount).eval(10, 3, 7));
        assert!(B::If(Box::new(condition), Box::new(B::True), Box::new(B::False)).eval(10, 3, 0));
        assert!(B::Or(Box::new(B::False), Box::new(B::True)).eval(0, 0, 0));
        assert!(!B::Le(U::Amount, U::Limit).eval(1, 0, 2));
    }

    #[test]
    fn canonical_wire_format_and_invalid_expressions() {
        let json = r#" { "args": [{"op":"spent"},{"op":"limit"}], "op":"le" } "#;
        assert_eq!(
            B::parse(json).unwrap().canonical(),
            r#"{"op":"le","args":[{"op":"spent"},{"op":"limit"}]}"#
        );
        for value in ["01", "+1", "-1", "18446744073709551616"] {
            let json = format!(
                r#"{{"op":"le","args":[{{"op":"const","args":"{value}"}},{{"op":"limit"}}]}}"#
            );
            assert!(B::parse(&json).is_err());
        }
        for json in [
            r#"{"op":"true","extra":1}"#,
            r#"{"op":"unknown"}"#,
            r#"{"op":"le","args":[{"op":"true"},{"op":"amount"}]}"#,
        ] {
            assert!(B::parse(json).is_err());
        }
    }
}
