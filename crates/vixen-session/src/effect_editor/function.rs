//! The f(x) Curve expression language (F75), checked against upstream's NCalc2 2.1.0.
//! Function names ignore case; parameters and grammar keywords do not. Numeric types
//! matter: integer operators wrap, and Abs yields a decimal, not a double.

use rust_decimal::{prelude::ToPrimitive, Decimal};

/// A message for the error box, and that box's title.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionError {
    pub message: String,
    pub title: String,
}

fn syntax_error(detail: &str) -> FunctionError {
    FunctionError {
        message: format!("The function entered has the following syntax errors.\n {detail}"),
        title: "Function Error".to_string(),
    }
}

fn runtime_error(detail: &str) -> FunctionError {
    FunctionError {
        message: format!("The function entered has the following syntax errors.\n {detail}"),
        title: "Function Errors".to_string(),
    }
}

/// `btnFunctionCurve_Click`, from the Draw Interval's text and the function's: the points to add, `x = 0, step, 2*step...` to 100, then 100 if that
/// wasn't hit, each `y` clamped to 0..100. A Draw Interval of 0 never finishes upstream, and empty throws: both are the same error box here (GUARD).
pub fn evaluate_curve_function(function: &str, draw_interval: &str) -> Result<Vec<(f64, f64)>, FunctionError> {
    let mut points = Vec::new();
    evaluate_curve_function_into(function, draw_interval, &mut points)?;
    Ok(points)
}

/// Upstream appends points as it evaluates, retaining earlier points on a later error.
pub(crate) fn evaluate_curve_function_into(function: &str, draw_interval: &str, points: &mut Vec<(f64, f64)>) -> Result<(), FunctionError> {
    if function.is_empty() {
        return Err(FunctionError { message: "The function entered is empty.".to_string(), title: "Function Error".to_string() });
    }
    let expression = parse(function).map_err(|e| syntax_error(&e))?;
    let step: i32 = match draw_interval.trim().parse::<i16>() {
        Ok(step) if step > 0 => step as i32,
        Ok(_) => return Err(runtime_error("The Draw Interval must be greater than 0.")),
        Err(_) => return Err(runtime_error("Input string was not in a correct format.")),
    };
    let mut last = 0;
    let mut x = 0;
    while x <= 100 {
        points.push((x as f64, calculate(&expression, x).map_err(|e| runtime_error(&e))?));
        last = x;
        x += step;
    }
    if last != 100 {
        points.push((100.0, calculate(&expression, 100).map_err(|e| runtime_error(&e))?));
    }
    Ok(())
}

/// `CalculateFunction`.
fn calculate(expression: &Expr, x: i32) -> Result<f64, String> {
    let y = expression.eval(x)?.to_double()?;
    Ok(if y < 0.0 {
        0.0
    } else if y > 100.0 {
        100.0
    } else {
        y
    })
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Int(i32),
    Long(i64),
    Double(f64),
    Decimal(Decimal),
    Date(vixen_io::datetime::LocalDateTime),
    Bool(bool),
    Text(String),
}

impl Value {
    /// `Convert.ToDouble`.
    fn to_double(&self) -> Result<f64, String> {
        match self {
            Value::Int(v) => Ok(*v as f64),
            Value::Long(v) => Ok(*v as f64),
            Value::Double(v) => Ok(*v),
            Value::Decimal(v) => Ok(v.to_f64().expect("96-bit decimal fits a double")),
            Value::Date(_) => Err("Invalid cast from 'DateTime' to 'Double'.".into()),
            Value::Bool(v) => Ok(if *v { 1.0 } else { 0.0 }),
            Value::Text(s) => numeric_grouping(s.trim())?.parse().map_err(|_| "Input string was not in a correct format.".to_string()),
        }
    }

    /// `Convert.ToBoolean`.
    fn to_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(v) => Ok(*v),
            Value::Text(s) => match s.trim().to_lowercase().as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err("String was not recognized as a valid Boolean.".to_string()),
            },
            other => Ok(other.to_double()? != 0.0),
        }
    }

    fn to_decimal(&self) -> Result<Decimal, String> {
        let overflow = || "Value was either too large or too small for a Decimal.".to_string();
        match self {
            Self::Int(v) => Ok(Decimal::from(*v)),
            Self::Long(v) => Ok(Decimal::from(*v)),
            Self::Decimal(v) => Ok(*v),
            Self::Bool(v) => Ok(Decimal::from(if *v { 1 } else { 0 })),
            Self::Double(v) => {
                if !v.is_finite() {
                    return Err(overflow());
                }
                if v.abs() < 1e-28 {
                    return Ok(Decimal::from_i128_with_scale((v * 1e28).round_ties_even() as i128, 28).normalize());
                }
                // Convert.ToDecimal(double) uses 15 significant decimal digits.
                let text = format!("{v:.14e}");
                let (base, exponent) = text.split_once('e').unwrap();
                let exponent: i32 = exponent.parse().unwrap();
                if exponent < 0 {
                    let mut value: Decimal = base.parse().map_err(|_| overflow())?;
                    let shift = exponent.unsigned_abs();
                    value = value.round_dp_with_strategy(28 - shift, rust_decimal::RoundingStrategy::MidpointNearestEven);
                    value.set_scale(value.scale() + shift).map_err(|_| overflow())?;
                    Ok(value.normalize())
                } else {
                    Decimal::from_scientific(&text).map(|v| v.normalize()).map_err(|_| overflow())
                }
            }
            Self::Text(v) => parse_decimal_text(v),
            Self::Date(_) => Err("Invalid cast from 'DateTime' to 'Decimal'.".into()),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => "int",
            Self::Long(_) => "long",
            Self::Double(_) => "double",
            Self::Decimal(_) => "decimal",
            Self::Bool(_) => "bool",
            Self::Text(_) => "string",
            Self::Date(_) => "System.DateTime",
        }
    }

    fn text(&self) -> String {
        match self {
            Self::Int(v) => v.to_string(),
            Self::Long(v) => v.to_string(),
            Self::Double(v) => v.to_string(),
            Self::Decimal(v) => v.to_string(),
            Self::Bool(v) => if *v { "True" } else { "False" }.into(),
            Self::Text(v) => v.clone(),
            Self::Date(v) => format!("{:02}/{:02}/{:04} {:02}:{:02}:{:02}", v.month, v.day, v.year, v.hour, v.minute, v.second),
        }
    }

    fn strict_bool(&self, operator: &str) -> Result<bool, String> {
        match self {
            Self::Bool(v) => Ok(*v),
            _ => Err(format!("Operator '{operator}' cannot be applied to operand of type '{}'", self.type_name())),
        }
    }
}

/// The invariant .NET number parser accepts repeated/trailing group separators,
/// but only after a digit and before the decimal separator or exponent.
fn numeric_grouping(text: &str) -> Result<String, String> {
    let mut after_digit = false;
    let mut integer = true;
    let mut result = String::new();
    for c in text.chars() {
        if c == ',' {
            if !after_digit || !integer {
                return Err("Input string was not in a correct format.".into());
            }
            continue;
        }
        if matches!(c, '.' | 'e' | 'E') {
            integer = false;
        }
        after_digit |= c.is_ascii_digit();
        result.push(c);
    }
    Ok(result)
}

/// Decimal.Parse uses midpoint-to-even when text has more than 28 fractional
/// places. The crate's ordinary text parser rounds ties away from zero, so make
/// that one conversion explicit; all arithmetic remains checked 96-bit Decimal.
fn parse_decimal_text(text: &str) -> Result<Decimal, String> {
    let invalid = || "Input string was not in a correct format.".to_string();
    let overflow = || "Value was either too large or too small for a Decimal.".to_string();
    let mut text = text.trim();
    let mut negative = false;
    let mut has_sign = false;
    if text.starts_with(['+', '-']) {
        negative = text.starts_with('-');
        has_sign = true;
        text = &text[1..];
    }
    if text.ends_with(['+', '-']) {
        if has_sign {
            return Err(invalid());
        }
        negative = text.ends_with('-');
        text = text[..text.len() - 1].trim_end();
    }
    let text = numeric_grouping(text)?;
    let mut digits = String::new();
    let mut fraction = None;
    for c in text.chars() {
        match c {
            '0'..='9' => {
                digits.push(c);
                if let Some(scale) = &mut fraction {
                    *scale += 1usize;
                }
            }
            '.' if fraction.is_none() => fraction = Some(0),
            _ => return Err(invalid()),
        }
    }
    if digits.is_empty() {
        return Err(invalid());
    }
    let digits = digits.trim_start_matches('0');
    let scale = fraction.unwrap_or(0);
    if digits.is_empty() {
        return Ok(Decimal::from_i128_with_scale(0, scale.min(28) as u32));
    }
    let mut discard = scale.saturating_sub(28).max(digits.len().saturating_sub(29));
    let max = (1u128 << 96) - 1;
    loop {
        if discard > scale {
            return Err(overflow());
        }
        let kept = digits.len().saturating_sub(discard);
        let mut value = if kept == 0 { 0 } else { digits[..kept].parse::<u128>().map_err(|_| overflow())? };
        if discard > 0 && discard <= digits.len() {
            let remainder = &digits[kept..];
            let first = remainder.as_bytes()[0];
            if first > b'5' || (first == b'5' && (remainder[1..].bytes().any(|c| c != b'0') || value % 2 != 0)) {
                value += 1;
            }
        }
        if value <= max {
            let signed = if negative { -(value as i128) } else { value as i128 };
            return Ok(Decimal::from_i128_with_scale(signed, (scale - discard) as u32));
        }
        discard += 1;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Op {
    Or,
    And,
    BitOr,
    BitXor,
    BitAnd,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Clone, Debug)]
enum Expr {
    Value(Value),
    Param(String),
    Call(String, Vec<Expr>),
    Not(Box<Expr>),
    Negate(Box<Expr>),
    BitNot(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
}

impl Expr {
    fn eval(&self, x: i32) -> Result<Value, String> {
        match self {
            Expr::Value(v) => Ok(v.clone()),
            Expr::Param(name) => {
                if name == "x" {
                    Ok(Value::Int(x))
                } else if name == "Pi" {
                    Ok(Value::Double(std::f64::consts::PI))
                } else {
                    Err(format!("Parameter was not defined (Parameter '{name}')"))
                }
            }
            Expr::Not(e) => Ok(Value::Bool(!e.eval(x)?.strict_bool("!")?)),
            Expr::Negate(e) => arithmetic(Op::Sub, &Value::Int(0), &e.eval(x)?),
            Expr::BitNot(e) => match e.eval(x)? {
                Value::Int(v) => Ok(Value::Int(!v)),
                Value::Long(v) => Ok(Value::Long(!v)),
                value => Err(format!("Operator '~' cannot be applied to operand of type '{}'", value.type_name())),
            },
            Expr::Ternary(c, a, b) => {
                if c.eval(x)?.to_bool()? {
                    a.eval(x)
                } else {
                    b.eval(x)
                }
            }
            Expr::Binary(op, a, b) => binary(*op, a, b, x),
            Expr::Call(name, args) => call(name, args, x),
        }
    }
}

fn operand_error(op: Op, a: &Value, b: &Value) -> String {
    format!("Operator '{}' cannot be applied to operands of type '{}' and '{}'", op_text(op), a.type_name(), b.type_name())
}

fn binary(op: Op, a: &Expr, b: &Expr, x: i32) -> Result<Value, String> {
    let left = a.eval(x)?;
    if matches!(op, Op::And | Op::Or) {
        let value = left.strict_bool(if op == Op::And { "!" } else { "||" })?;
        if (op == Op::And && !value) || (op == Op::Or && value) {
            return Ok(Value::Bool(value));
        }
        let right = b.eval(x)?;
        return match right {
            Value::Bool(value) => Ok(Value::Bool(value)),
            _ => Err(operand_error(op, &left, &right)),
        };
    }
    let right = b.eval(x)?;
    match op {
        Op::BitOr | Op::BitXor | Op::BitAnd | Op::Shl | Op::Shr => bitwise(op, &left, &right),
        Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => relational(op, &left, &right).map(Value::Bool),
        _ => arithmetic(op, &left, &right),
    }
}

fn bitwise(op: Op, a: &Value, b: &Value) -> Result<Value, String> {
    if let (Value::Bool(l), Value::Bool(r)) = (a, b) {
        return match op {
            Op::BitOr => Ok(Value::Bool(*l | *r)),
            Op::BitAnd => Ok(Value::Bool(*l & *r)),
            Op::BitXor => Ok(Value::Bool(*l ^ *r)),
            _ => Err(operand_error(op, a, b)),
        };
    }
    if matches!(op, Op::Shl | Op::Shr) {
        let Value::Int(r) = b else {
            return Err(operand_error(op, a, b));
        };
        return match a {
            Value::Int(l) => Ok(Value::Int(if op == Op::Shl { l.wrapping_shl(*r as u32) } else { l.wrapping_shr(*r as u32) })),
            Value::Long(l) => Ok(Value::Long(if op == Op::Shl { l.wrapping_shl(*r as u32) } else { l.wrapping_shr(*r as u32) })),
            _ => Err(operand_error(op, a, b)),
        };
    }
    let integer = |v: &Value| match v {
        Value::Int(v) => Some(*v as i64),
        Value::Long(v) => Some(*v),
        _ => None,
    };
    let (Some(l), Some(r)) = (integer(a), integer(b)) else {
        return Err(operand_error(op, a, b));
    };
    let result = match op {
        Op::BitOr => l | r,
        Op::BitAnd => l & r,
        Op::BitXor => l ^ r,
        _ => unreachable!(),
    };
    Ok(if matches!((a, b), (Value::Int(_), Value::Int(_))) { Value::Int(result as i32) } else { Value::Long(result) })
}

fn relational(op: Op, a: &Value, b: &Value) -> Result<bool, String> {
    let equal_only = matches!(op, Op::Eq | Op::Ne);
    let order = match (a, b) {
        (Value::Text(l), Value::Text(r)) if equal_only => Some(l.cmp(r)),
        (Value::Bool(l), Value::Bool(r)) if equal_only => Some(l.cmp(r)),
        (Value::Date(l), Value::Date(r)) => Some(l.cmp(r)),
        (Value::Bool(_), _)
        | (_, Value::Bool(_))
        | (Value::Text(_), _)
        | (_, Value::Text(_))
        | (Value::Date(_), _)
        | (_, Value::Date(_)) => return Err(operand_error(op, a, b)),
        (Value::Decimal(_), Value::Double(_)) | (Value::Double(_), Value::Decimal(_)) => return Err(operand_error(op, a, b)),
        (Value::Int(l), Value::Long(r)) => Some((*l as i64).cmp(r)),
        (Value::Long(l), Value::Int(r)) => Some(l.cmp(&(*r as i64))),
        (Value::Long(l), Value::Long(r)) => Some(l.cmp(r)),
        (Value::Decimal(_), _) | (_, Value::Decimal(_)) => Some(a.to_decimal()?.cmp(&b.to_decimal()?)),
        _ => a.to_double()?.partial_cmp(&b.to_double()?),
    };
    use std::cmp::Ordering::*;
    Ok(match op {
        Op::Eq => order == Some(Equal),
        Op::Ne => order != Some(Equal),
        Op::Lt => order == Some(Less),
        Op::Le => matches!(order, Some(Less | Equal)),
        Op::Gt => order == Some(Greater),
        Op::Ge => matches!(order, Some(Greater | Equal)),
        _ => unreachable!(),
    })
}

/// `Numbers.Add` and friends.
fn arithmetic(op: Op, a: &Value, b: &Value) -> Result<Value, String> {
    if op == Op::Add && (matches!(a, Value::Text(_)) || matches!(b, Value::Text(_))) {
        return Ok(Value::Text(a.text() + &b.text()));
    }
    if matches!(a, Value::Bool(_) | Value::Text(_) | Value::Date(_)) || matches!(b, Value::Bool(_) | Value::Text(_) | Value::Date(_)) {
        return Err(operand_error(op, a, b));
    }
    if op != Op::Add && matches!((a, b), (Value::Decimal(_), Value::Double(_)) | (Value::Double(_), Value::Decimal(_))) {
        return Err(operand_error(op, a, b));
    }
    if matches!(a, Value::Decimal(_)) || matches!(b, Value::Decimal(_)) {
        if op == Op::Add && (matches!(a, Value::Double(_)) || matches!(b, Value::Double(_))) {
            return Ok(Value::Double(a.to_double()? + b.to_double()?));
        }
        let (l, r) = (a.to_decimal()?, b.to_decimal()?);
        if matches!(op, Op::Div | Op::Mod) && r.is_zero() {
            return Err("Attempted to divide by zero.".into());
        }
        let result = match op {
            Op::Add => l.checked_add(r),
            Op::Sub => l.checked_sub(r),
            Op::Mul => l.checked_mul(r),
            Op::Div => l.checked_div(r),
            Op::Mod => l.checked_rem(r),
            _ => unreachable!(),
        };
        return result.map(Value::Decimal).ok_or_else(|| "Value was either too large or too small for a Decimal.".into());
    }
    if op == Op::Div {
        // NCalc promotes Int32/Int32 division to double, but its Int64 dynamic
        // operator retains integer division (including mixed Int32/Int64).
        let integer = |v: &Value| match v {
            Value::Int(v) => Some(*v as i64),
            Value::Long(v) => Some(*v),
            _ => None,
        };
        if (matches!(a, Value::Long(_)) || matches!(b, Value::Long(_))) && integer(a).is_some() && integer(b).is_some() {
            let (l, r) = (integer(a).unwrap(), integer(b).unwrap());
            if r == 0 {
                return Err("Attempted to divide by zero.".into());
            }
            return l.checked_div(r).map(Value::Long).ok_or_else(|| "Arithmetic operation resulted in an overflow.".into());
        }
        return Ok(Value::Double(a.to_double()? / b.to_double()?));
    }
    match (a, b) {
        (Value::Int(l), Value::Int(r)) => match op {
            Op::Add => Ok(Value::Int(l.wrapping_add(*r))),
            Op::Sub => Ok(Value::Int(l.wrapping_sub(*r))),
            Op::Mul => Ok(Value::Int(l.wrapping_mul(*r))),
            Op::Mod if *r == 0 => Err("Attempted to divide by zero.".to_string()),
            Op::Mod => Ok(Value::Int(l.wrapping_rem(*r))),
            _ => Err("Unexpected operator.".to_string()),
        },
        (Value::Long(_), Value::Int(_) | Value::Long(_)) | (Value::Int(_), Value::Long(_)) => {
            let long = |v: &Value| match v {
                Value::Int(n) => *n as i64,
                Value::Long(n) => *n,
                _ => 0,
            };
            let (l, r) = (long(a), long(b));
            match op {
                Op::Add => Ok(Value::Long(l.wrapping_add(r))),
                Op::Sub => Ok(Value::Long(l.wrapping_sub(r))),
                Op::Mul => Ok(Value::Long(l.wrapping_mul(r))),
                Op::Mod if r == 0 => Err("Attempted to divide by zero.".to_string()),
                Op::Mod => Ok(Value::Long(l.wrapping_rem(r))),
                _ => Err("Unexpected operator.".to_string()),
            }
        }
        (Value::Bool(_), _) | (_, Value::Bool(_)) | (Value::Text(_), _) | (_, Value::Text(_)) => {
            Err(format!("Operator '{}' can't be applied to these operands.", op_text(op)))
        }
        _ => {
            let (l, r) = (a.to_double()?, b.to_double()?);
            Ok(Value::Double(match op {
                Op::Add => l + r,
                Op::Sub => l - r,
                Op::Mul => l * r,
                Op::Mod => l % r,
                _ => return Err("Unexpected operator.".to_string()),
            }))
        }
    }
}

fn op_text(op: Op) -> &'static str {
    match op {
        Op::Add => "+",
        Op::Sub => "-",
        Op::Mul => "*",
        Op::Div => "/",
        Op::Mod => "%",
        Op::Eq => "==",
        Op::Ne => "!=",
        Op::Lt => "<",
        Op::Le => "<=",
        Op::Gt => ">",
        Op::Ge => ">=",
        Op::BitAnd => "&",
        Op::BitOr => "|",
        Op::BitXor => "^",
        Op::Shl => "<<",
        Op::Shr => ">>",
        Op::And => "&&",
        Op::Or => "||",
    }
}

fn compare(a: &Value, b: &Value) -> Result<std::cmp::Ordering, String> {
    match (a, b) {
        (Value::Text(l), Value::Text(r)) => Ok(l.cmp(r)),
        (Value::Decimal(_), _) | (_, Value::Decimal(_)) => Ok(a.to_decimal()?.cmp(&b.to_decimal()?)),
        (Value::Long(l), Value::Long(r)) => Ok(l.cmp(r)),
        (Value::Int(l), Value::Long(r)) => Ok((*l as i64).cmp(r)),
        (Value::Long(l), Value::Int(r)) => Ok(l.cmp(&(*r as i64))),
        _ => a.to_double()?.partial_cmp(&b.to_double()?).ok_or_else(|| "Cannot compare these values.".to_string()),
    }
}

fn call(name: &str, args: &[Expr], x: i32) -> Result<Value, String> {
    let lower = name.to_lowercase();
    let count = |n: usize| -> Result<(), String> {
        if args.len() == n {
            Ok(())
        } else {
            Err(format!("{name}() takes exactly {n} argument{}", if n == 1 { "" } else { "s" }))
        }
    };
    let num = |i: usize| -> Result<f64, String> { args[i].eval(x)?.to_double() };
    let one = |f: fn(f64) -> f64| -> Result<Value, String> {
        count(1)?;
        Ok(Value::Double(f(num(0)?)))
    };
    match lower.as_str() {
        "abs" => {
            count(1)?;
            Ok(Value::Decimal(args[0].eval(x)?.to_decimal()?.abs()))
        }
        "acos" => one(f64::acos),
        "asin" => one(f64::asin),
        "atan" => one(f64::atan),
        "ceiling" => one(f64::ceil),
        "cos" => one(f64::cos),
        "exp" => one(f64::exp),
        "floor" => one(f64::floor),
        "log10" => one(f64::log10),
        "sin" => one(f64::sin),
        "sqrt" => one(f64::sqrt),
        "tan" => one(f64::tan),
        "truncate" => one(f64::trunc),
        "sign" => {
            count(1)?;
            let v = num(0)?;
            if v.is_nan() {
                return Err("Non-negative number required.".to_string());
            }
            Ok(Value::Int(if v > 0.0 {
                1
            } else if v < 0.0 {
                -1
            } else {
                0
            }))
        }
        "ieeeremainder" => {
            count(2)?;
            let (a, b) = (num(0)?, num(1)?);
            let regular = a % b;
            let alternate = regular - b.abs() * a.signum();
            let result = if alternate.abs() < regular.abs()
                || (alternate.abs() == regular.abs() && (a / b).round_ties_even().abs() > (a / b).abs())
            {
                alternate
            } else {
                regular
            };
            Ok(Value::Double(result))
        }
        "log" => {
            count(2)?;
            let (value, base) = (num(0)?, num(1)?);
            Ok(Value::Double(if base == 1.0 || (value != 1.0 && (base == 0.0 || base == f64::INFINITY)) {
                f64::NAN
            } else {
                value.ln() / base.ln()
            }))
        }
        "pow" => {
            count(2)?;
            Ok(Value::Double(num(0)?.powf(num(1)?)))
        }
        "round" => {
            count(2)?;
            let digits = num(1)?.round_ties_even();
            if !(0.0..=15.0).contains(&digits) {
                return Err("Rounding digits must be between 0 and 15, inclusive. (Parameter 'digits')".into());
            }
            let scale = 10f64.powi(digits as i32);
            let value = num(0)?;
            Ok(Value::Double(if value.abs() < 1e16 { (value * scale).round_ties_even() / scale } else { value }))
        }
        "max" | "min" => {
            count(2)?;
            let (a, b) = (args[0].eval(x)?, args[1].eval(x)?);
            if matches!(a, Value::Bool(_) | Value::Text(_) | Value::Date(_))
                || matches!(b, Value::Bool(_) | Value::Text(_) | Value::Date(_))
                || matches!((&a, &b), (Value::Decimal(_), Value::Double(_)) | (Value::Double(_), Value::Decimal(_)))
            {
                return Err(format!(
                    "The best overloaded method match for 'System.Math.{}(byte, byte)' has some invalid arguments",
                    if lower == "min" { "Min" } else { "Max" }
                ));
            }
            if matches!(a, Value::Double(_)) || matches!(b, Value::Double(_)) {
                let (l, r) = (a.to_double()?, b.to_double()?);
                return Ok(Value::Double(if l.is_nan() || r.is_nan() {
                    f64::NAN
                } else if lower == "min" {
                    l.min(r)
                } else {
                    l.max(r)
                }));
            }
            let less = compare(&a, &b)? == std::cmp::Ordering::Less;
            let value = if less == (lower == "min") { &a } else { &b };
            Ok(if matches!(a, Value::Decimal(_)) || matches!(b, Value::Decimal(_)) {
                Value::Decimal(value.to_decimal()?)
            } else if matches!(a, Value::Long(_)) || matches!(b, Value::Long(_)) {
                match value {
                    Value::Long(v) => Value::Long(*v),
                    Value::Int(v) => Value::Long(*v as i64),
                    _ => unreachable!(),
                }
            } else {
                value.clone()
            })
        }
        "if" => {
            count(3)?;
            if args[0].eval(x)?.to_bool()? {
                args[1].eval(x)
            } else {
                args[2].eval(x)
            }
        }
        "in" => {
            if args.len() < 2 {
                return Err("in() takes at least 2 arguments".to_string());
            }
            let v = args[0].eval(x)?;
            for a in &args[1..] {
                if compare(&v, &a.eval(x)?)? == std::cmp::Ordering::Equal {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }
        _ => Err(format!("Function not found (Parameter '{name}')")),
    }
}

// The grammar, tightest binding last.

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

fn parse(text: &str) -> Result<Expr, String> {
    let mut p = Parser { text, at: 0 };
    let e = p.conditional()?;
    p.skip();
    if p.at < text.len() {
        return Err(format!("extraneous input at position {}", p.at));
    }
    Ok(e)
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.text[self.at..]
    }

    fn skip(&mut self) {
        while self.rest().starts_with(|c: char| c.is_whitespace()) {
            self.at += self.rest().chars().next().map_or(0, char::len_utf8);
        }
    }

    /// Takes a symbol, ignoring space before it. A word needs a break after it.
    fn eat(&mut self, symbol: &str) -> bool {
        self.skip();
        let rest = self.rest();
        let matches = if symbol.chars().all(char::is_alphabetic) {
            rest.len() >= symbol.len()
                && rest.is_char_boundary(symbol.len())
                && &rest[..symbol.len()] == symbol
                && !rest[symbol.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
        } else {
            rest.starts_with(symbol)
        };
        if matches {
            self.at += symbol.len();
        }
        matches
    }

    fn conditional(&mut self) -> Result<Expr, String> {
        let c = self.or()?;
        if self.eat("?") {
            let a = self.conditional()?;
            if !self.eat(":") {
                return Err(format!("missing ':' at position {}", self.at));
            }
            let b = self.conditional()?;
            return Ok(Expr::Ternary(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut e = self.and()?;
        while self.eat("||") || self.eat("or") {
            e = Expr::Binary(Op::Or, Box::new(e), Box::new(self.and()?));
        }
        Ok(e)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut e = self.bit_or()?;
        while self.eat("&&") || self.eat("and") {
            e = Expr::Binary(Op::And, Box::new(e), Box::new(self.bit_or()?));
        }
        Ok(e)
    }

    fn bit_or(&mut self) -> Result<Expr, String> {
        let mut e = self.bit_xor()?;
        loop {
            self.skip();
            if self.rest().starts_with('|') && !self.rest().starts_with("||") {
                self.at += 1;
                e = Expr::Binary(Op::BitOr, Box::new(e), Box::new(self.bit_xor()?));
            } else {
                return Ok(e);
            }
        }
    }

    fn bit_xor(&mut self) -> Result<Expr, String> {
        let mut e = self.bit_and()?;
        while self.eat("^") {
            e = Expr::Binary(Op::BitXor, Box::new(e), Box::new(self.bit_and()?));
        }
        Ok(e)
    }

    fn bit_and(&mut self) -> Result<Expr, String> {
        let mut e = self.equality()?;
        loop {
            self.skip();
            if self.rest().starts_with('&') && !self.rest().starts_with("&&") {
                self.at += 1;
                e = Expr::Binary(Op::BitAnd, Box::new(e), Box::new(self.equality()?));
            } else {
                return Ok(e);
            }
        }
    }

    fn equality(&mut self) -> Result<Expr, String> {
        let mut e = self.relational()?;
        loop {
            let op = if self.eat("==") {
                Op::Eq
            } else if self.eat("!=") || self.eat("<>") {
                Op::Ne
            } else if self.eat("=") {
                Op::Eq
            } else {
                return Ok(e);
            };
            e = Expr::Binary(op, Box::new(e), Box::new(self.relational()?));
        }
    }

    fn relational(&mut self) -> Result<Expr, String> {
        let mut e = self.shift()?;
        loop {
            self.skip();
            // `<>` and the shifts belong to other levels.
            let rest = self.rest();
            let op = if rest.starts_with("<=") {
                self.at += 2;
                Op::Le
            } else if rest.starts_with(">=") {
                self.at += 2;
                Op::Ge
            } else if rest.starts_with('<') && !rest.starts_with("<>") && !rest.starts_with("<<") {
                self.at += 1;
                Op::Lt
            } else if rest.starts_with('>') && !rest.starts_with(">>") {
                self.at += 1;
                Op::Gt
            } else {
                return Ok(e);
            };
            e = Expr::Binary(op, Box::new(e), Box::new(self.shift()?));
        }
    }

    fn shift(&mut self) -> Result<Expr, String> {
        let mut e = self.additive()?;
        loop {
            let op = if self.eat("<<") {
                Op::Shl
            } else if self.eat(">>") {
                Op::Shr
            } else {
                return Ok(e);
            };
            e = Expr::Binary(op, Box::new(e), Box::new(self.additive()?));
        }
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let mut e = self.multiplicative()?;
        loop {
            let op = if self.eat("+") {
                Op::Add
            } else if self.eat("-") {
                Op::Sub
            } else {
                return Ok(e);
            };
            e = Expr::Binary(op, Box::new(e), Box::new(self.multiplicative()?));
        }
    }

    fn multiplicative(&mut self) -> Result<Expr, String> {
        let mut e = self.unary()?;
        loop {
            let op = if self.eat("*") {
                Op::Mul
            } else if self.eat("/") {
                Op::Div
            } else if self.eat("%") {
                Op::Mod
            } else {
                return Ok(e);
            };
            e = Expr::Binary(op, Box::new(e), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        self.skip();
        if self.rest().starts_with('!') && !self.rest().starts_with("!=") {
            self.at += 1;
            return Ok(Expr::Not(Box::new(self.primary()?)));
        }
        if self.eat("not") {
            return Ok(Expr::Not(Box::new(self.primary()?)));
        }
        if self.eat("~") {
            return Ok(Expr::BitNot(Box::new(self.primary()?)));
        }
        if self.eat("-") {
            return Ok(Expr::Negate(Box::new(self.primary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, String> {
        self.skip();
        if self.eat("(") {
            let e = self.conditional()?;
            if !self.eat(")") {
                return Err(format!("missing ')' at position {}", self.at));
            }
            return Ok(e);
        }
        let rest = self.rest();
        let Some(first) = rest.chars().next() else {
            return Err("unexpected end of the function".to_string());
        };
        if first.is_ascii_digit() || (first == '.' && rest[1..].starts_with(|c: char| c.is_ascii_digit())) {
            return self.number();
        }
        if first == '\'' {
            return self.string();
        }
        if first == '#' {
            let Some(end) = rest[1..].find('#') else {
                return Err("missing closing date delimiter".into());
            };
            let text = &rest[1..end + 1];
            let date = vixen_io::datetime::LocalDateTime::parse(text)
                .ok_or_else(|| "String was not recognized as a valid DateTime.".to_string())?;
            let leap = date.year % 4 == 0 && (date.year % 100 != 0 || date.year % 400 == 0);
            let days = match date.month {
                2 => {
                    if leap {
                        29
                    } else {
                        28
                    }
                }
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            };
            if !(1..=9999).contains(&date.year) || date.day > days {
                return Err("String was not recognized as a valid DateTime.".into());
            }
            self.at += end + 2;
            return Ok(Expr::Value(Value::Date(date)));
        }
        if first == '[' {
            let Some(end) = rest.find(']') else {
                return Err(format!("missing ']' at position {}", self.at));
            };
            let name = rest[1..end].to_string();
            self.at += end + 1;
            return Ok(Expr::Param(name));
        }
        if first.is_alphabetic() || first == '_' {
            return self.identifier();
        }
        Err(format!("no viable alternative at input '{first}' at position {}", self.at))
    }

    fn number(&mut self) -> Result<Expr, String> {
        let rest = self.rest();
        let mut end = 0;
        let bytes = rest.as_bytes();
        let mut real = false;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'.' {
            real = true;
            end += 1;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
        }
        if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
            let mut e = end + 1;
            if e < bytes.len() && (bytes[e] == b'+' || bytes[e] == b'-') {
                e += 1;
            }
            if e < bytes.len() && bytes[e].is_ascii_digit() {
                real = true;
                while e < bytes.len() && bytes[e].is_ascii_digit() {
                    e += 1;
                }
                end = e;
            }
        }
        let text = rest[..end].to_string();
        self.at += end;
        if real {
            return text.parse().map(|v| Expr::Value(Value::Double(v))).map_err(|_| format!("bad number {text}"));
        }
        if let Ok(v) = text.parse::<i32>() {
            return Ok(Expr::Value(Value::Int(v)));
        }
        text.parse::<i64>().map(|v| Expr::Value(Value::Long(v))).map_err(|_| format!("bad number {text}"))
    }

    fn string(&mut self) -> Result<Expr, String> {
        self.at += 1;
        let mut value = String::new();
        loop {
            let Some(c) = self.rest().chars().next() else {
                return Err(format!("missing closing quote at position {}", self.at));
            };
            self.at += c.len_utf8();
            match c {
                '\'' => return Ok(Expr::Value(Value::Text(value))),
                '\\' => {
                    let Some(escaped) = self.rest().chars().next() else {
                        return Err("incomplete string escape".into());
                    };
                    self.at += escaped.len_utf8();
                    if escaped == 'u' {
                        let Some(digits) = self.rest().get(..4) else {
                            return Err("incomplete Unicode escape".into());
                        };
                        let code = u32::from_str_radix(digits, 16).map_err(|_| "invalid Unicode escape".to_string())?;
                        self.at += 4;
                        value.push(char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER));
                        continue;
                    }
                    value.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\'' => '\'',
                        '\\' => '\\',
                        _ => return Err(format!("invalid escape \\{escaped}")),
                    });
                }
                _ => value.push(c),
            }
        }
    }

    fn identifier(&mut self) -> Result<Expr, String> {
        let rest = self.rest();
        let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
        let name = rest[..end].to_string();
        self.at += end;
        if name == "true" {
            return Ok(Expr::Value(Value::Bool(true)));
        }
        if name == "false" {
            return Ok(Expr::Value(Value::Bool(false)));
        }
        if !self.eat("(") {
            return Ok(Expr::Param(name));
        }
        let mut args = Vec::new();
        if self.eat(")") {
            return Err(format!("no viable alternative at input '{name}()'"));
        }
        loop {
            args.push(self.conditional()?);
            if self.eat(")") {
                return Ok(Expr::Call(name, args));
            }
            if !self.eat(",") {
                return Err(format!("missing ')' at position {}", self.at));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_ncalc_curve_contracts() {
        let bytes = std::env::var("NCALC_GOLDEN").map_or_else(
            |_| include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/ncalc.json")).to_vec(),
            |path| std::fs::read(path).unwrap(),
        );
        let data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let rows = data["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 4282);
        assert_eq!(rows.iter().map(|row| row["points"].as_array().unwrap().len()).sum::<usize>(), 8650);
        let mut failures = Vec::new();
        for row in rows {
            assert_eq!(row["cancel"], false);
            let expression = row["expression"].as_str().unwrap();
            let step = row["step"].as_str().unwrap();
            let mut points = Vec::new();
            let error = evaluate_curve_function_into(expression, step, &mut points).err();
            let expected: Vec<(f64, f64)> = row["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap_or_else(|| p[1].as_str().unwrap().parse().unwrap())))
                .collect();
            let points_match = points.len() == expected.len()
                && points
                    .iter()
                    .zip(&expected)
                    .all(|((x, y), (ex, ey))| x == ex && (y == ey || (y.is_nan() && ey.is_nan()) || (y - ey).abs() <= 1e-10));
            let title = error.as_ref().map(|e| e.title.as_str());
            if !points_match || title != row["title"].as_str() {
                failures.push(format!(
                    "{expression:?} step={step}: points={points:?} error={error:?}; expected points={expected:?} title={} error={}",
                    row["title"], row["error"]
                ));
            }
        }
        for failure in &failures {
            eprintln!("{failure}");
        }
        assert!(failures.is_empty(), "{} mismatches", failures.len());
        eprintln!("NCalc2 2.1.0: 4,282 cases, 8,650 generated points, zero mismatches");
    }

    #[test]
    fn the_gamma_example() {
        let points = evaluate_curve_function("Pow(x/100, 2.2) * 80", "5").unwrap();
        assert_eq!(points.len(), 21);
        assert_eq!(points[0], (0.0, 0.0));
        assert_eq!(points[20].0, 100.0);
        assert!((points[20].1 - 80.0).abs() < 1e-12);
        assert!((points[10].1 - 80.0 * 0.5f64.powf(2.2)).abs() < 1e-12);
    }

    #[test]
    fn the_step_and_a_final_hundred() {
        let xs: Vec<f64> = evaluate_curve_function("x", "30").unwrap().iter().map(|p| p.0).collect();
        assert_eq!(xs, vec![0.0, 30.0, 60.0, 90.0, 100.0]);
    }

    #[test]
    fn values_clamp_and_names_ignore_case() {
        assert!(evaluate_curve_function("x * 10", "50").unwrap().iter().all(|p| p.1 <= 100.0));
        assert_eq!(evaluate_curve_function("x - 200", "50").unwrap()[1].1, 0.0);
        assert!(evaluate_curve_function("pow(x/100, 2) * 100 + SIN(Pi) * 0", "50").is_ok());
        assert!(evaluate_curve_function("X", "50").is_err());
        assert!(evaluate_curve_function("PI", "50").is_err());
        let sine = evaluate_curve_function("Sin(2 * Pi * (x / 100)) * ((25-7)/2) + ((25-7) /2) + 7", "25").unwrap();
        assert!((sine[1].1 - 25.0).abs() < 1e-9);
    }

    #[test]
    fn errors_have_their_own_text() {
        let e = evaluate_curve_function("", "5").unwrap_err();
        assert_eq!(e.message, "The function entered is empty.");
        assert_eq!(e.title, "Function Error");
        let e = evaluate_curve_function("(x", "5").unwrap_err();
        assert!(e.message.starts_with("The function entered has the following syntax errors.\n "), "{}", e.message);
        assert_eq!(e.title, "Function Error");
        let e = evaluate_curve_function("foo(x)", "5").unwrap_err();
        assert_eq!(e.title, "Function Errors");
    }

    #[test]
    fn a_draw_interval_of_zero_or_nothing_is_an_error_not_a_hang() {
        assert_eq!(evaluate_curve_function("x", "0").unwrap_err().title, "Function Errors");
        assert_eq!(evaluate_curve_function("x", "").unwrap_err().title, "Function Errors");
    }

    #[test]
    fn division_is_real_and_integers_are_ints() {
        assert_eq!(evaluate_curve_function("x / 100 * 100", "100").unwrap()[1].1, 100.0);
        // 100 * 100 * 100 * 100 * 100 overflows an int and wraps to 1410065408, as NCalc's `Numbers.Multiply` does.
        assert_eq!(evaluate_curve_function("x*x*x*x*x", "100").unwrap()[1].1, 100.0);
        assert_eq!(evaluate_curve_function("x % 7", "100").unwrap()[1].1, 2.0);
    }

    #[test]
    fn logic_and_conditionals() {
        let p = evaluate_curve_function("if(x > 50, 80, 20)", "100").unwrap();
        assert_eq!((p[0].1, p[1].1), (20.0, 80.0));
        let p = evaluate_curve_function("x > 50 ? 80 : 20", "100").unwrap();
        assert_eq!((p[0].1, p[1].1), (20.0, 80.0));
        let p = evaluate_curve_function("in(x, 0, 1) and not false ? 10 : 90", "100").unwrap();
        assert_eq!((p[0].1, p[1].1), (10.0, 90.0));
        let p = evaluate_curve_function("Max(x, 10) - Min(x, 10)", "100").unwrap();
        assert_eq!((p[0].1, p[1].1), (10.0, 90.0));
        let p = evaluate_curve_function("-x + 100", "100").unwrap();
        assert_eq!((p[0].1, p[1].1), (100.0, 0.0));
    }
}
