//! IR builder.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

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
