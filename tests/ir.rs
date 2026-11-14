use mithqal::ir::{
    Attributes, BindingMode, Block, Builder, CallMode, Definition, EvalMode, Expr, FORMAT_VERSION,
    IrError, Module, Pattern, Rule, Scope, ScopeId, Stmt, SymbolKind, Visibility,
};

fn sample_module() -> Module {
    let mut builder = Builder::new();
    let x = builder.symbol("x", SymbolKind::Local, Attributes::new());
    let f = builder.symbol("f", SymbolKind::Global, Attributes::new());
    let plus = builder.symbol(
        "plus",
        SymbolKind::Builtin,
        Attributes::new()
            .with(Attributes::FLAT)
            .with(Attributes::ORDERLESS),
    );
    let x_expr = builder.expr(Expr::Symbol(x));
    let one = builder.expr(Expr::Integer("1".into()));
    let plus_expr = builder.expr(Expr::Symbol(plus));
    let call = builder.expr(Expr::Call {
        head: plus_expr,
        arguments: vec![x_expr, one],
        mode: CallMode::Strict,
    });
    let wildcard = builder.pattern(Pattern::Wildcard);
    let bound = builder.pattern(Pattern::Bind {
        name: x,
        pattern: wildcard,
    });
    let body = builder.expr(Expr::Symbol(x));
    let definition = builder.definition(Definition {
        symbol: f,
        visibility: Visibility::Public,
        rules: vec![Rule {
            parameters: vec![bound],
            guard: None,
            body,
            mode: EvalMode::Delayed,
        }],
        documentation: Some("identity".into()),
    });
    let scope = builder.scope(Some(ScopeId::new(0)), vec![x]);
    let block = builder.block(Block {
        scope,
        statements: vec![Stmt::Define(definition), Stmt::Expr(call)],
    });
    builder.set_entry(block);
    builder.build().unwrap()
}

#[test]
fn structured_module_validates() {
    let module = sample_module();
    assert_eq!(module.format_version, FORMAT_VERSION);
    assert!(module.validate().is_ok());
    assert_eq!(module.symbols.len(), 3);
    assert_eq!(module.definitions.len(), 1);
    assert_eq!(module.blocks.len(), 2);
}

#[test]
fn duplicate_symbol_names_are_rejected() {
    let mut builder = Builder::new();
    builder.symbol("x", SymbolKind::Global, Attributes::new());
    builder.symbol("x", SymbolKind::Global, Attributes::new());
    assert!(matches!(
        builder.build(),
        Err(IrError::DuplicateSymbol(name)) if name.as_ref() == "x"
    ));
}

#[test]
fn invalid_entry_block_is_rejected() {
    let module = Module::new(mithqal::ir::BlockId::new(9));
    assert!(matches!(module.validate(), Err(IrError::InvalidBlock(_))));
}

#[test]
fn unsupported_version_is_rejected() {
    let mut module = sample_module();
    module.format_version = FORMAT_VERSION + 1;
    assert!(matches!(
        module.validate(),
        Err(IrError::UnsupportedVersion(_))
    ));
}

#[test]
fn empty_definition_is_rejected() {
    let mut builder = Builder::new();
    let f = builder.symbol("f", SymbolKind::Global, Attributes::new());
    let definition = builder.definition(Definition {
        symbol: f,
        visibility: Visibility::Private,
        rules: Vec::new(),
        documentation: None,
    });
    builder.module_mut().blocks[0]
        .statements
        .push(Stmt::Define(definition));
    assert!(matches!(builder.build(), Err(IrError::EmptyDefinition(_))));
}

#[test]
fn scope_cycles_are_rejected() {
    let mut builder = Builder::new();
    builder.module_mut().scopes.push(Scope {
        parent: Some(ScopeId::new(0)),
        symbols: Vec::new(),
    });
    builder.module_mut().scopes[0].parent = Some(ScopeId::new(1));
    assert!(matches!(builder.build(), Err(IrError::ScopeCycle(_))));
}

#[test]
fn attributes_are_distinct_flags() {
    let attributes = Attributes::new()
        .with(Attributes::INERT)
        .with(Attributes::LISTABLE)
        .with(Attributes::PROTECTED);
    assert!(attributes.has(Attributes::INERT));
    assert!(attributes.has(Attributes::LISTABLE));
    assert!(attributes.has(Attributes::PROTECTED));
    assert!(!attributes.has(Attributes::FLAT));
    assert!(!attributes.has(Attributes::ORDERLESS));
}

#[test]
fn lazy_bindings_and_match_arms_validate() {
    let mut builder = Builder::new();
    let x = builder.symbol("x", SymbolKind::Local, Attributes::new());
    let value = builder.expr(Expr::Integer("42".into()));
    let symbol = builder.expr(Expr::Symbol(x));
    let wildcard = builder.pattern(Pattern::Wildcard);
    let arm = mithqal::ir::MatchArm {
        pattern: wildcard,
        guard: None,
        body: symbol,
    };
    let match_expr = builder.expr(Expr::Match {
        scrutinee: symbol,
        arms: vec![arm],
    });
    let scope = builder.scope(Some(ScopeId::new(0)), vec![x]);
    let block = builder.block(Block {
        scope,
        statements: vec![
            Stmt::Let {
                pattern: wildcard,
                value,
                mode: BindingMode::Lazy,
            },
            Stmt::Expr(match_expr),
        ],
    });
    builder.set_entry(block);
    assert!(builder.build().is_ok());
}
