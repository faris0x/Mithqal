//! Structured core IR for the Mithqal language.
//!
//! This module defines a typed, validated, versioned representation of a whole
//! program. It is intentionally separate from the expression arena and from the
//! flat virtual machine instructions. The structured form keeps lexical blocks,
//! bindings, definitions, patterns, and control flow visible so later passes can
//! analyze and lower them without reconstructing intent from flat bytecode.
//!
//! The IR is a scaffold. Parsing and lowering into it arrive in a later layer.

use core::fmt;

/// Version of the structured IR layout.
pub const FORMAT_VERSION: u16 = 1;

/// Maximum combined node count accepted by validation.
pub const MAX_NODES: usize = 1_000_000;

macro_rules! index_type {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            /// Build an index from a stored offset.
            pub const fn new(index: u32) -> Self {
                $name(index)
            }

            /// The stored offset.
            pub const fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

index_type!(SymbolId, "Handle to a symbol declaration.");
index_type!(ScopeId, "Handle to a lexical scope.");
index_type!(ExprId, "Handle to a structured expression.");
index_type!(PatternId, "Handle to a pattern.");
index_type!(BlockId, "Handle to a lexical block.");
index_type!(DefinitionId, "Handle to a definition.");

/// Structural attributes attached to a symbol.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Attributes(u16);

impl Attributes {
    /// No attributes.
    pub const fn new() -> Self {
        Attributes(0)
    }

    /// Arguments are hidden from automatic evaluation.
    pub const INERT: u16 = 1 << 0;
    /// The head distributes over list arguments.
    pub const LISTABLE: u16 = 1 << 1;
    /// Nested calls with the same head may be flattened.
    pub const FLAT: u16 = 1 << 2;
    /// Arguments have no fixed order.
    pub const ORDERLESS: u16 = 1 << 3;
    /// The symbol resolves to a fixed constant.
    pub const CONSTANT: u16 = 1 << 4;
    /// The symbol may not be redefined.
    pub const PROTECTED: u16 = 1 << 5;

    /// Add a flag.
    pub const fn with(self, flag: u16) -> Self {
        Attributes(self.0 | flag)
    }

    /// Test a flag.
    pub const fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }
}

/// How a symbol enters the module.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    /// Provided by the kernel.
    Builtin,
    /// Declared at module scope.
    Global,
    /// Introduced by a binding.
    Local,
    /// Introduced by a function parameter.
    Parameter,
}

/// Whether a symbol is visible outside its module.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Visibility {
    /// Visible to importers.
    Public,
    /// Visible only inside the module.
    Private,
}

/// When a right hand side is evaluated.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EvalMode {
    /// Evaluate when the definition is installed.
    Immediate,
    /// Evaluate when the definition is used.
    Delayed,
}

/// How a binding evaluates its value.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BindingMode {
    /// Evaluate once.
    Value,
    /// Evaluate on each read.
    Lazy,
}

/// Whether call arguments are evaluated before the call.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CallMode {
    /// Evaluate arguments before dispatch.
    Strict,
    /// Pass arguments as written.
    Quoted,
}

/// A module level symbol declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolDecl {
    /// Source name.
    pub name: Box<str>,
    /// Entry kind.
    pub kind: SymbolKind,
    /// Structural attributes.
    pub attributes: Attributes,
}

/// A lexical scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scope {
    /// Enclosing scope.
    pub parent: Option<ScopeId>,
    /// Symbols introduced directly in this scope.
    pub symbols: Vec<SymbolId>,
}

/// A structured expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// The unit value.
    Unit,
    /// A boolean.
    Bool(bool),
    /// An arbitrary precision integer written in canonical decimal.
    Integer(Box<str>),
    /// A rational number.
    Rational {
        /// Numerator in canonical decimal.
        numerator: Box<str>,
        /// Denominator in canonical decimal.
        denominator: Box<str>,
    },
    /// An approximate real.
    Real(Box<str>),
    /// A text value.
    Text(Box<str>),
    /// A symbol reference.
    Symbol(SymbolId),
    /// An ordered list.
    List(Vec<ExprId>),
    /// A call with a computed or literal head.
    Call {
        /// Head expression.
        head: ExprId,
        /// Argument expressions.
        arguments: Vec<ExprId>,
        /// Argument evaluation policy.
        mode: CallMode,
    },
    /// A function value.
    Lambda {
        /// Parameter patterns.
        parameters: Vec<PatternId>,
        /// Function body.
        body: BlockId,
    },
    /// A conditional expression.
    If {
        /// Boolean condition.
        condition: ExprId,
        /// Block taken when the condition is true.
        then_block: BlockId,
        /// Block taken when the condition is false.
        else_block: BlockId,
    },
    /// A pattern match expression.
    Match {
        /// Value being inspected.
        scrutinee: ExprId,
        /// Ordered match arms.
        arms: Vec<MatchArm>,
    },
    /// An expression carried without evaluation.
    Quote(ExprId),
}

/// One arm of a match expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArm {
    /// Pattern to match.
    pub pattern: PatternId,
    /// Optional guard evaluated after a successful match.
    pub guard: Option<ExprId>,
    /// Expression produced by the arm.
    pub body: ExprId,
}

/// A pattern in the structured IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pattern {
    /// Match any value.
    Wildcard,
    /// Match any value and bind it to a symbol.
    Bind {
        /// Bound symbol.
        name: SymbolId,
        /// Nested pattern.
        pattern: PatternId,
    },
    /// Match a literal expression.
    Literal(ExprId),
    /// Match a call with a fixed head.
    Constructor {
        /// Required head.
        head: ExprId,
        /// Argument patterns.
        arguments: Vec<PatternId>,
    },
    /// Match a sequence and optionally bind it.
    Sequence(Option<SymbolId>),
    /// Match any one of several patterns.
    Alternative(Vec<PatternId>),
    /// Match a nested pattern and then a condition.
    Guard {
        /// Nested pattern.
        pattern: PatternId,
        /// Condition.
        condition: ExprId,
    },
}

/// A statement inside a block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stmt {
    /// Bind a value.
    Let {
        /// Binding pattern.
        pattern: PatternId,
        /// Value expression.
        value: ExprId,
        /// Evaluation policy.
        mode: BindingMode,
    },
    /// Install a definition.
    Define(DefinitionId),
    /// Replace the value bound to a target.
    Assign {
        /// Assignment target.
        target: ExprId,
        /// New value.
        value: ExprId,
        /// Evaluation policy.
        mode: EvalMode,
    },
    /// Evaluate an expression for its effect.
    Expr(ExprId),
    /// Return from the enclosing callable.
    Return(ExprId),
}

/// A lexical block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    /// Scope introduced by the block.
    pub scope: ScopeId,
    /// Ordered statements.
    pub statements: Vec<Stmt>,
}

/// One definition rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    /// Parameter patterns.
    pub parameters: Vec<PatternId>,
    /// Optional guard.
    pub guard: Option<ExprId>,
    /// Rule body.
    pub body: ExprId,
    /// Evaluation policy.
    pub mode: EvalMode,
}

/// A definition attached to a symbol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    /// Target symbol.
    pub symbol: SymbolId,
    /// Visibility outside the module.
    pub visibility: Visibility,
    /// Ordered rules.
    pub rules: Vec<Rule>,
    /// Optional documentation.
    pub documentation: Option<Box<str>>,
}

/// A validated structured module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    /// Layout version.
    pub format_version: u16,
    /// Symbol table.
    pub symbols: Vec<SymbolDecl>,
    /// Lexical scopes.
    pub scopes: Vec<Scope>,
    /// Pattern arena.
    pub patterns: Vec<Pattern>,
    /// Expression arena.
    pub expressions: Vec<Expr>,
    /// Block arena.
    pub blocks: Vec<Block>,
    /// Definition table.
    pub definitions: Vec<Definition>,
    /// Entry block.
    pub entry: BlockId,
}

/// A validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrError {
    /// The layout version is unknown.
    UnsupportedVersion(u16),
    /// The module exceeds the node limit.
    TooManyNodes,
    /// The entry block index is invalid.
    InvalidBlock(BlockId),
    /// An expression index is invalid.
    InvalidExpr(ExprId),
    /// A pattern index is invalid.
    InvalidPattern(PatternId),
    /// A symbol index is invalid.
    InvalidSymbol(SymbolId),
    /// A scope index is invalid.
    InvalidScope(ScopeId),
    /// A definition index is invalid.
    InvalidDefinition(DefinitionId),
    /// Two symbols share a name.
    DuplicateSymbol(Box<str>),
    /// A symbol name is empty.
    EmptySymbolName,
    /// A literal component is empty.
    EmptyLiteral,
    /// A scope names a cycle through its parent.
    ScopeCycle(ScopeId),
    /// A definition has no rules.
    EmptyDefinition(SymbolId),
}

impl fmt::Display for IrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrError::UnsupportedVersion(version) => {
                write!(formatter, "unsupported IR version {version}")
            }
            IrError::TooManyNodes => formatter.write_str("IR node limit exceeded"),
            IrError::InvalidBlock(id) => write!(formatter, "invalid block {}", id.0),
            IrError::InvalidExpr(id) => write!(formatter, "invalid expression {}", id.0),
            IrError::InvalidPattern(id) => write!(formatter, "invalid pattern {}", id.0),
            IrError::InvalidSymbol(id) => write!(formatter, "invalid symbol {}", id.0),
            IrError::InvalidScope(id) => write!(formatter, "invalid scope {}", id.0),
            IrError::InvalidDefinition(id) => write!(formatter, "invalid definition {}", id.0),
            IrError::DuplicateSymbol(name) => write!(formatter, "duplicate symbol {name}"),
            IrError::EmptySymbolName => formatter.write_str("empty symbol name"),
            IrError::EmptyLiteral => formatter.write_str("empty literal component"),
            IrError::ScopeCycle(id) => write!(formatter, "scope cycle at {}", id.0),
            IrError::EmptyDefinition(id) => write!(formatter, "definition {} has no rules", id.0),
        }
    }
}

impl std::error::Error for IrError {}

impl Module {
    /// Create an empty module with the current format version.
    pub fn new(entry: BlockId) -> Self {
        Module {
            format_version: FORMAT_VERSION,
            symbols: Vec::new(),
            scopes: Vec::new(),
            patterns: Vec::new(),
            expressions: Vec::new(),
            blocks: Vec::new(),
            definitions: Vec::new(),
            entry,
        }
    }

    /// Validate every reference and structural invariant.
    pub fn validate(&self) -> Result<(), IrError> {
        if self.format_version != FORMAT_VERSION {
            return Err(IrError::UnsupportedVersion(self.format_version));
        }
        let nodes = self.symbols.len()
            + self.scopes.len()
            + self.patterns.len()
            + self.expressions.len()
            + self.blocks.len()
            + self.definitions.len();
        if nodes > MAX_NODES {
            return Err(IrError::TooManyNodes);
        }
        let mut names = std::collections::HashSet::new();
        for symbol in &self.symbols {
            if symbol.name.is_empty() {
                return Err(IrError::EmptySymbolName);
            }
            if !names.insert(symbol.name.as_ref()) {
                return Err(IrError::DuplicateSymbol(symbol.name.clone()));
            }
        }
        self.block(self.entry)?;
        for (index, scope) in self.scopes.iter().enumerate() {
            if let Some(parent) = scope.parent {
                self.scope(parent)?;
                let mut current = Some(parent);
                let mut steps = 0;
                while let Some(scope_id) = current {
                    if scope_id.index() == index {
                        return Err(IrError::ScopeCycle(ScopeId::new(index as u32)));
                    }
                    steps += 1;
                    if steps > self.scopes.len() {
                        return Err(IrError::ScopeCycle(ScopeId::new(index as u32)));
                    }
                    current = self.scopes[scope_id.index()].parent;
                }
            }
            for symbol in &scope.symbols {
                self.symbol(*symbol)?;
            }
        }
        for pattern in &self.patterns {
            self.validate_pattern(pattern)?;
        }
        for expression in &self.expressions {
            self.validate_expr(expression)?;
        }
        for block in &self.blocks {
            self.scope(block.scope)?;
            for statement in &block.statements {
                self.validate_stmt(statement)?;
            }
        }
        for definition in &self.definitions {
            self.symbol(definition.symbol)?;
            if definition.rules.is_empty() {
                return Err(IrError::EmptyDefinition(definition.symbol));
            }
            for rule in &definition.rules {
                for parameter in &rule.parameters {
                    self.pattern(*parameter)?;
                }
                if let Some(guard) = rule.guard {
                    self.expr(guard)?;
                }
                self.expr(rule.body)?;
            }
        }
        Ok(())
    }

    fn validate_expr(&self, expression: &Expr) -> Result<(), IrError> {
        match expression {
            Expr::Unit | Expr::Bool(_) | Expr::Integer(_) | Expr::Real(_) | Expr::Text(_) => Ok(()),
            Expr::Rational {
                numerator,
                denominator,
            } => {
                if numerator.is_empty() || denominator.is_empty() {
                    return Err(IrError::EmptyLiteral);
                }
                Ok(())
            }
            Expr::Symbol(symbol) => self.symbol(*symbol),
            Expr::List(items) => {
                for item in items {
                    self.expr(*item)?;
                }
                Ok(())
            }
            Expr::Call {
                head, arguments, ..
            } => {
                self.expr(*head)?;
                for argument in arguments {
                    self.expr(*argument)?;
                }
                Ok(())
            }
            Expr::Lambda { parameters, body } => {
                for parameter in parameters {
                    self.pattern(*parameter)?;
                }
                self.block(*body)
            }
            Expr::If {
                condition,
                then_block,
                else_block,
            } => {
                self.expr(*condition)?;
                self.block(*then_block)?;
                self.block(*else_block)
            }
            Expr::Match { scrutinee, arms } => {
                self.expr(*scrutinee)?;
                for arm in arms {
                    self.pattern(arm.pattern)?;
                    if let Some(guard) = arm.guard {
                        self.expr(guard)?;
                    }
                    self.expr(arm.body)?;
                }
                Ok(())
            }
            Expr::Quote(inner) => self.expr(*inner),
        }
    }

    fn validate_pattern(&self, pattern: &Pattern) -> Result<(), IrError> {
        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Bind { name, pattern } => {
                self.symbol(*name)?;
                self.pattern(*pattern)
            }
            Pattern::Literal(expression) => self.expr(*expression),
            Pattern::Constructor { head, arguments } => {
                self.expr(*head)?;
                for argument in arguments {
                    self.pattern(*argument)?;
                }
                Ok(())
            }
            Pattern::Sequence(name) => {
                if let Some(name) = name {
                    self.symbol(*name)?;
                }
                Ok(())
            }
            Pattern::Alternative(patterns) => {
                for pattern in patterns {
                    self.pattern(*pattern)?;
                }
                Ok(())
            }
            Pattern::Guard { pattern, condition } => {
                self.pattern(*pattern)?;
                self.expr(*condition)
            }
        }
    }

    fn validate_stmt(&self, statement: &Stmt) -> Result<(), IrError> {
        match statement {
            Stmt::Let { pattern, value, .. } => {
                self.pattern(*pattern)?;
                self.expr(*value)
            }
            Stmt::Define(definition) => {
                self.definition(*definition)?;
                Ok(())
            }
            Stmt::Assign { target, value, .. } => {
                self.expr(*target)?;
                self.expr(*value)
            }
            Stmt::Expr(expression) | Stmt::Return(expression) => self.expr(*expression),
        }
    }

    fn symbol(&self, id: SymbolId) -> Result<(), IrError> {
        if id.index() < self.symbols.len() {
            Ok(())
        } else {
            Err(IrError::InvalidSymbol(id))
        }
    }

    fn scope(&self, id: ScopeId) -> Result<(), IrError> {
        if id.index() < self.scopes.len() {
            Ok(())
        } else {
            Err(IrError::InvalidScope(id))
        }
    }

    fn expr(&self, id: ExprId) -> Result<(), IrError> {
        if id.index() < self.expressions.len() {
            Ok(())
        } else {
            Err(IrError::InvalidExpr(id))
        }
    }

    fn pattern(&self, id: PatternId) -> Result<(), IrError> {
        if id.index() < self.patterns.len() {
            Ok(())
        } else {
            Err(IrError::InvalidPattern(id))
        }
    }

    fn block(&self, id: BlockId) -> Result<(), IrError> {
        if id.index() < self.blocks.len() {
            Ok(())
        } else {
            Err(IrError::InvalidBlock(id))
        }
    }

    fn definition(&self, id: DefinitionId) -> Result<(), IrError> {
        if id.index() < self.definitions.len() {
            Ok(())
        } else {
            Err(IrError::InvalidDefinition(id))
        }
    }
}

/// Incremental builder for a structured module.
#[derive(Debug)]
pub struct Builder {
    module: Module,
}

impl Builder {
    /// Start a module with one root scope and an entry block.
    pub fn new() -> Self {
        let mut module = Module::new(BlockId::new(0));
        module.scopes.push(Scope {
            parent: None,
            symbols: Vec::new(),
        });
        module.blocks.push(Block {
            scope: ScopeId::new(0),
            statements: Vec::new(),
        });
        Builder { module }
    }

    /// Append a symbol declaration.
    pub fn symbol(
        &mut self,
        name: impl Into<Box<str>>,
        kind: SymbolKind,
        attributes: Attributes,
    ) -> SymbolId {
        let id = SymbolId::new(self.module.symbols.len() as u32);
        self.module.symbols.push(SymbolDecl {
            name: name.into(),
            kind,
            attributes,
        });
        id
    }

    /// Append a scope.
    pub fn scope(&mut self, parent: Option<ScopeId>, symbols: Vec<SymbolId>) -> ScopeId {
        let id = ScopeId::new(self.module.scopes.len() as u32);
        self.module.scopes.push(Scope { parent, symbols });
        id
    }

    /// Append an expression.
    pub fn expr(&mut self, expression: Expr) -> ExprId {
        let id = ExprId::new(self.module.expressions.len() as u32);
        self.module.expressions.push(expression);
        id
    }

    /// Append a pattern.
    pub fn pattern(&mut self, pattern: Pattern) -> PatternId {
        let id = PatternId::new(self.module.patterns.len() as u32);
        self.module.patterns.push(pattern);
        id
    }

    /// Append a block.
    pub fn block(&mut self, block: Block) -> BlockId {
        let id = BlockId::new(self.module.blocks.len() as u32);
        self.module.blocks.push(block);
        id
    }

    /// Append a definition.
    pub fn definition(&mut self, definition: Definition) -> DefinitionId {
        let id = DefinitionId::new(self.module.definitions.len() as u32);
        self.module.definitions.push(definition);
        id
    }

    /// Replace the entry block.
    pub fn set_entry(&mut self, entry: BlockId) {
        self.module.entry = entry;
    }

    /// Finish construction and validate the module.
    pub fn build(self) -> Result<Module, IrError> {
        self.module.validate()?;
        Ok(self.module)
    }

    /// Borrow the module under construction.
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Mutably borrow the module under construction.
    pub fn module_mut(&mut self) -> &mut Module {
        &mut self.module
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
