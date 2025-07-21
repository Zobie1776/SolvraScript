use crate::ast::*;
use std::collections::HashMap;
use std::fmt;

/// NovaScript runtime value types
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        closure: Environment,
    },
    NativeFunction {
        name: String,
        arity: usize,
        func: fn(&[Value]) -> Result<Value, RuntimeError>,
    },
    Null,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, val) in arr.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                write!(f, "{{")?;
                for (i, (key, val)) in obj.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", key, val)?;
                }
                write!(f, "}}")
            }
            Value::Function { name, .. } => write!(f, "<function {}>", name),
            Value::NativeFunction { name, .. } => write!(f, "<native function {}>", name),
            Value::Null => write!(f, "null"),
        }
    }
}

impl Value {
    /// Check if value is truthy (NovaScript truthiness rules)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Int(0) => false,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
            _ => true,
        }
    }

    /// Get the type name of the value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function { .. } => "function",
            Value::NativeFunction { .. } => "native_function",
            Value::Null => "null",
        }
    }

    /// Convert to number if possible
    pub fn to_number(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            Value::Bool(true) => Some(1.0),
            Value::Bool(false) => Some(0.0),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuntimeError {
    VariableNotFound(String),
    TypeError(String),
    ArgumentError(String),
    IndexError(String),
    DivisionByZero,
    StackOverflow,
    NotImplemented(String),
    Return(Value),
    Break,
    Continue,
    Custom(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::VariableNotFound(name) => write!(f, "Variable '{}' not found", name),
            RuntimeError::TypeError(msg) => write!(f, "Type error: {}", msg),
            RuntimeError::ArgumentError(msg) => write!(f, "Argument error: {}", msg),
            RuntimeError::IndexError(msg) => write!(f, "Index error: {}", msg),
            RuntimeError::DivisionByZero => write!(f, "Division by zero"),
            RuntimeError::StackOverflow => write!(f, "Stack overflow"),
            RuntimeError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            RuntimeError::Return(val) => write!(f, "Return: {}", val),
            RuntimeError::Break => write!(f, "Break statement outside loop"),
            RuntimeError::Continue => write!(f, "Continue statement outside loop"),
            RuntimeError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

type Environment = HashMap<String, Value>;

pub struct Interpreter {
    globals: Environment,
    locals: Vec<Environment>,
    call_stack: Vec<String>,
    max_call_depth: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            globals: HashMap::new(),
            locals: Vec::new(),
            call_stack: Vec::new(),
            max_call_depth: 1000,
        };
        interpreter.init_builtins();
        interpreter
    }

    fn init_builtins(&mut self) {
        // Built-in functions
        self.globals.insert("print".to_string(), Value::NativeFunction {
            name: "print".to_string(),
            arity: 1,
            func: |args| {
                if args.len() != 1 {
                    return Err(RuntimeError::ArgumentError("print expects 1 argument".to_string()));
                }
                println!("{}", args[0]);
                Ok(Value::Null)
            },
        });

        self.globals.insert("len".to_string(), Value::NativeFunction {
            name: "len".to_string(),
            arity: 1,
            func: |args| {
                if args.len() != 1 {
                    return Err(RuntimeError::ArgumentError("len expects 1 argument".to_string()));
                }
                let length = match &args[0] {
                    Value::String(s) => s.len(),
                    Value::Array(arr) => arr.len(),
                    Value::Object(obj) => obj.len(),
                    _ => return Err(RuntimeError::TypeError("len() not supported for this type".to_string())),
                };
                Ok(Value::Int(length as i64))
            },
        });

        self.globals.insert("type".to_string(), Value::NativeFunction {
            name: "type".to_string(),
            arity: 1,
            func: |args| {
                if args.len() != 1 {
                    return Err(RuntimeError::ArgumentError("type expects 1 argument".to_string()));
                }
                Ok(Value::String(args[0].type_name().to_string()))
            },
        });
    }

    pub fn eval_program(&mut self, program: &Program) -> Result<Option<Value>, RuntimeError> {
        let mut last = None;
        for stmt in &program.statements {
            match self.eval_stmt(stmt) {
                Ok(val) => last = val,
                Err(RuntimeError::Return(val)) => return Ok(Some(val)),
                Err(e) => return Err(e),
            }
        }
        Ok(last)
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> Result<Option<Value>, RuntimeError> {
        match stmt {
            Stmt::VariableDecl { decl } => {
                let val = if let Some(expr) = &decl.initializer {
                    self.eval_expr(expr)?
                } else {
                    Value::Null
                };
                self.set_variable(decl.name.clone(), val);
                Ok(None)
            }
            
            Stmt::Expression { expr, .. } => {
                let v = self.eval_expr(expr)?;
                Ok(Some(v))
            }
            
            Stmt::Return { value, .. } => {
                let v = if let Some(expr) = value {
                    self.eval_expr(expr)?
                } else {
                    Value::Null
                };
                Err(RuntimeError::Return(v))
            }
            
            Stmt::Break { .. } => Err(RuntimeError::Break),
            Stmt::Continue { .. } => Err(RuntimeError::Continue),
            
            Stmt::Block { statements, .. } => {
                self.push_scope();
                let mut result = Ok(None);
                for stmt in statements {
                    match self.eval_stmt(stmt) {
                        Ok(val) => result = Ok(val),
                        Err(RuntimeError::Break) | Err(RuntimeError::Continue) => {
                            result = Err(RuntimeError::Break);
                            break;
                        }
                        Err(RuntimeError::Return(val)) => {
                            result = Err(RuntimeError::Return(val));
                            break;
                        }
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    }
                }
                self.pop_scope();
                result
            }

            Stmt::If { condition, then_branch, else_branch, .. } => {
                let cond_val = self.eval_expr(condition)?;
                if cond_val.is_truthy() {
                    self.eval_stmt(then_branch)
                } else if let Some(else_stmt) = else_branch {
                    self.eval_stmt(else_stmt)
                } else {
                    Ok(None)
                }
            }

            Stmt::While { condition, body, .. } => {
                let mut result = Ok(None);
                loop {
                    let cond_val = self.eval_expr(condition)?;
                    if !cond_val.is_truthy() {
                        break;
                    }
                    
                    match self.eval_stmt(body) {
                        Ok(val) => result = Ok(val),
                        Err(RuntimeError::Break) => break,
                        Err(RuntimeError::Continue) => continue,
                        Err(RuntimeError::Return(val)) => return Err(RuntimeError::Return(val)),
                        Err(e) => return Err(e),
                    }
                }
                result
            }

            Stmt::For { variable, iterable, body, .. } => {
                let iterable_val = self.eval_expr(iterable)?;
                match iterable_val {
                    Value::Array(elements) => {
                        let mut result = Ok(None);
                        for item in elements {
                            self.push_scope();
                            self.set_variable(variable.clone(), item);
                            match self.eval_stmt(body) {
                                Ok(val) => result = Ok(val),
                                Err(RuntimeError::Break) => { self.pop_scope(); break; }
                                Err(RuntimeError::Continue) => { self.pop_scope(); continue; }
                                Err(RuntimeError::Return(v)) => { self.pop_scope(); return Err(RuntimeError::Return(v)); }
                                Err(e) => { self.pop_scope(); return Err(e); }
                            }
                            self.pop_scope();
                        }
                        result
                    }
                    _ => Err(RuntimeError::TypeError(
                        format!("Value of type '{}' is not iterable", iterable_val.type_name())
                    )),
                }
            }

            Stmt::FunctionDecl { decl } => {
                let func = Value::Function {
                    name: decl.name.clone(),
                    params: decl.params.iter().map(|p| p.name.clone()).collect(),
                    body: decl.body.clone(),
                    closure: self.capture_environment(),
                };
                self.set_variable(decl.name.clone(), func);
                Ok(None)
            }

            other => Err(RuntimeError::NotImplemented(format!("Statement: {:?}", other))),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal { value, .. } => self.eval_literal(value),
            Expr::Identifier { name, .. } => {
                self.get_variable(name)
                    .ok_or_else(|| RuntimeError::VariableNotFound(name.clone()))
            }
            Expr::Binary { left, operator, right, .. } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.eval_binary_op(operator, l, r)
            }
            Expr::Unary { operator, operand, .. } => {
                let v = self.eval_expr(operand)?;
                self.eval_unary_op(operator, v)
            }

            Expr::Assignment { target, value, .. } => {
                if let Expr::Identifier { name, .. } = &**target {
                    let val = self.eval_expr(value)?;
                    self.set_variable(name.clone(), val.clone());
                    Ok(val)
                } else {
                    Err(RuntimeError::TypeError("Invalid assignment target".to_string()))
                }
            }
            Expr::Call { callee, args, .. } => {
                let func = self.eval_expr(callee)?;
                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(self.eval_expr(arg)?);
                }
                self.call_function(func, arg_values)
            }
            Expr::Index { object, index, .. } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                self.eval_index_access(obj, idx)
            }
            Expr::Member { object, property, .. } => {
                let obj = self.eval_expr(object)?;
                match obj {
                    Value::Object(map) => {
                        Ok(map.get(property)
                            .cloned()
                            .unwrap_or(Value::Null))
                    }
                    _ => Err(RuntimeError::TypeError(
                        format!("Cannot access property '{}' on {}", property, obj.type_name())
                    )),
                }
            }

            other => Err(RuntimeError::NotImplemented(format!("Expression: {:?}", other))),
        }
    }

    fn eval_literal(&mut self, lit: &Literal) -> Result<Value, RuntimeError> {
        match lit {
            Literal::Integer(n) => Ok(Value::Int(*n)),
            Literal::Float(f) => Ok(Value::Float(*f)),
            Literal::Boolean(b) => Ok(Value::Bool(*b)),
            Literal::String(s) => Ok(Value::String(s.clone())),
            Literal::Null => Ok(Value::Null),
            
            Literal::Array(arr) => {
                let mut values = Vec::new();
                for expr in arr {
                    values.push(self.eval_expr(expr)?);
                }
                Ok(Value::Array(values))
            }
            
            Literal::Object(props) => {
                let mut map = HashMap::new();
                for (key, expr) in props {
                    let value = self.eval_expr(expr)?;
                    map.insert(key.clone(), value);
                }
                Ok(Value::Object(map))
            }
        }
    }

    fn eval_binary_op(&self, op: &BinaryOp, left: Value, right: Value) -> Result<Value, RuntimeError> {
        use BinaryOp::*;
        use Value::*;
        match op {
            Add => match (left, right) {
                (Int(a), Int(b)) => Ok(Int(a + b)),
                (Float(a), Float(b)) => Ok(Float(a + b)),
                (Int(a), Float(b)) => Ok(Float(a as f64 + b)),
                (Float(a), Int(b)) => Ok(Float(a + b as f64)),
                (String(a), String(b)) => Ok(String(a + &b)),
                (String(a), b) => Ok(String(a + &b.to_string())),
                (a, String(b)) => Ok(String(a.to_string() + &b)),
                (a, b) => Err(RuntimeError::TypeError(format!("Add not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            Subtract => match (left, right) {
                (Int(a), Int(b)) => Ok(Int(a - b)),
                (Float(a), Float(b)) => Ok(Float(a - b)),
                (Int(a), Float(b)) => Ok(Float(a as f64 - b)),
                (Float(a), Int(b)) => Ok(Float(a - b as f64)),
                (a, b) => Err(RuntimeError::TypeError(format!("Subtract not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            Multiply => match (left, right) {
                (Int(a), Int(b)) => Ok(Int(a * b)),
                (Float(a), Float(b)) => Ok(Float(a * b)),
                (Int(a), Float(b)) => Ok(Float(a as f64 * b)),
                (Float(a), Int(b)) => Ok(Float(a * b as f64)),
                (a, b) => Err(RuntimeError::TypeError(format!("Multiply not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            Divide => match (left, right) {
                (Int(_), Int(0)) | (Float(_), Float(0.0)) | (Int(_), Float(0.0)) | (Float(_), Int(0)) => Err(RuntimeError::DivisionByZero),
                (Int(a), Int(b)) => Ok(Int(a / b)),
                (Float(a), Float(b)) => Ok(Float(a / b)),
                (Int(a), Float(b)) => Ok(Float(a as f64 / b)),
                (Float(a), Int(b)) => Ok(Float(a / b as f64)),
                (a, b) => Err(RuntimeError::TypeError(format!("Divide not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            Modulo => match (left, right) {
                (Int(_), Int(0)) => Err(RuntimeError::DivisionByZero),
                (Int(a), Int(b)) => Ok(Int(a % b)),
                (a, b) => Err(RuntimeError::TypeError(format!("Modulo not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            Equal => Ok(Value::Bool(left == right)),
            NotEqual => Ok(Value::Bool(left != right)),
            Less => match (left, right) {
                (Int(a), Int(b)) => Ok(Bool(a < b)),
                (Float(a), Float(b)) => Ok(Bool(a < b)),
                (a, b) => Err(RuntimeError::TypeError(format!("Less not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            Greater => match (left, right) {
                (Int(a), Int(b)) => Ok(Bool(a > b)),
                (Float(a), Float(b)) => Ok(Bool(a > b)),
                (a, b) => Err(RuntimeError::TypeError(format!("Greater not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            LessEqual => match (left, right) {
                (Int(a), Int(b)) => Ok(Bool(a <= b)),
                (Float(a), Float(b)) => Ok(Bool(a <= b)),
                (a, b) => Err(RuntimeError::TypeError(format!("LessEqual not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            GreaterEqual => match (left, right) {
                (Int(a), Int(b)) => Ok(Bool(a >= b)),
                (Float(a), Float(b)) => Ok(Bool(a >= b)),
                (a, b) => Err(RuntimeError::TypeError(format!("GreaterEqual not supported for {} and {}", a.type_name(), b.type_name()))),
            },
            And => Ok(Bool(left.is_truthy() && right.is_truthy())),
            Or => Ok(Bool(left.is_truthy() || right.is_truthy())),
            _ => Err(RuntimeError::NotImplemented(format!("Operator {:?} not implemented", op))),
        }
    }

    fn eval_unary_op(&self, op: &UnaryOp, operand: Value) -> Result<Value, RuntimeError> {
        use UnaryOp::*;
        use Value::*;
        
        match (op, operand) {
            (_Neg, Int(n)) => Ok(Int(-n)),
            (_Neg, Float(f)) => Ok(Float(-f)),
            (Not, val) => Ok(Bool(!val.is_truthy())),
            (_, val) => Err(RuntimeError::TypeError(
                format!("Unary operation {:?} not supported for {}", op, val.type_name())
            )),
        }
    }

    fn eval_index_access(&self, object: Value, index: Value) -> Result<Value, RuntimeError> {
        match (object, index) {
            (Value::Array(arr), Value::Int(idx)) => {
                let i = if idx < 0 {
                    arr.len() as i64 + idx
                } else {
                    idx
                };
                
                if i < 0 || i >= arr.len() as i64 {
                    Err(RuntimeError::IndexError(format!("Index {} out of bounds", idx)))
                } else {
                    Ok(arr[i as usize].clone())
                }
            }
            (Value::Object(obj), Value::String(key)) => {
                Ok(obj.get(&key).cloned().unwrap_or(Value::Null))
            }
            (Value::String(s), Value::Int(idx)) => {
                let chars: Vec<char> = s.chars().collect();
                let i = if idx < 0 {
                    chars.len() as i64 + idx
                } else {
                    idx
                };
                
                if i < 0 || i >= chars.len() as i64 {
                    Err(RuntimeError::IndexError(format!("Index {} out of bounds", idx)))
                } else {
                    Ok(Value::String(chars[i as usize].to_string()))
                }
            }
            (obj, idx) => Err(RuntimeError::TypeError(
                format!("Cannot index {} with {}", obj.type_name(), idx.type_name())
            )),
        }
    }

    fn call_function(&mut self, func: Value, args: Vec<Value>) -> Result<Value, RuntimeError> {
        if self.call_stack.len() >= self.max_call_depth {
            return Err(RuntimeError::StackOverflow);
        }

        match func {
            Value::NativeFunction { name, arity, func } => {
                if args.len() != arity {
                    return Err(RuntimeError::ArgumentError(
                        format!("Function '{}' expects {} arguments, got {}", name, arity, args.len())
                    ));
                }
                func(&args)
            }
            
            Value::Function { name, params, body, closure: _ } => {
                if args.len() != params.len() {
                    return Err(RuntimeError::ArgumentError(
                        format!("Function '{}' expects {} arguments, got {}", name, params.len(), args.len())
                    ));
                }
                
                self.call_stack.push(name.clone());
                
                // Create new scope with closure and parameters
                self.push_scope();
                
                // Bind parameters to arguments
                for (param, arg) in params.iter().zip(args.iter()) {
                    self.set_variable(param.clone(), arg.clone());
                }
                
                // Execute function body
                let mut result = Value::Null;
                for stmt in &body {
                    match self.eval_stmt(stmt) {
                        Ok(_) => {}
                        Err(RuntimeError::Return(val)) => {
                            result = val;
                            break;
                        }
                        Err(e) => {
                            self.pop_scope();
                            self.call_stack.pop();
                            return Err(e);
                        }
                    }
                }
                
                self.pop_scope();
                self.call_stack.pop();
                Ok(result)
            }
            
            _ => Err(RuntimeError::TypeError(
                format!("Value of type '{}' is not callable", func.type_name())
            )),
        }
    }

    // Environment management
    fn push_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.locals.pop();
    }

    fn set_variable(&mut self, name: String, value: Value) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name, value);
        } else {
            self.globals.insert(name, value);
        }
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        // Search local scopes from innermost to outermost
        for scope in self.locals.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        // Search global scope
        self.globals.get(name).cloned()
    }

    fn capture_environment(&self) -> Environment {
        // For now, just capture globals. In a full implementation,
        // you'd capture the current scope chain.
        self.globals.clone()
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_display() {
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::String("hello".to_string()).to_string(), "hello");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Null.to_string(), "null");
    }

    #[test]
    fn test_value_truthiness() {
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(!Value::Null.is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
    }

    #[test]
    fn test_interpreter_creation() {
        let interpreter = Interpreter::new();
        assert!(!interpreter.globals.is_empty()); // Should have builtins
    }
}