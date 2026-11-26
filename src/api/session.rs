//! Session methods.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;
use crate::diag::Message;
use crate::expr::Attributes;
use crate::stdlib;

impl Session {
    /// Create a session with default limits.
    pub fn new() -> Self {
        Self::with_config(Config::default())
    }

    /// Create a session with explicit limits.
    ///
    /// # Panics
    ///
    /// Panics if a bundled standard library module fails to parse or evaluate.
    /// The modules are part of the crate and are exercised by the test suite.
    pub fn with_config(config: Config) -> Self {
        Self::with_optional_modules(config, None)
    }

    /// Create a session with only the selected standard library domains.
    ///
    /// Dependencies declared by imports are loaded automatically.
    ///
    /// # Panics
    ///
    /// Panics if a selected module is unknown or fails to parse or evaluate.
    pub fn with_modules(modules: &[&str]) -> Self {
        Self::with_optional_modules(Config::default(), Some(modules))
    }

    /// Create a session with explicit limits and selected standard library domains.
    ///
    /// # Panics
    ///
    /// Panics if a selected module is unknown or fails to parse or evaluate.
    pub fn with_config_and_modules(config: Config, modules: &[&str]) -> Self {
        Self::with_optional_modules(config, Some(modules))
    }

    fn with_optional_modules(config: Config, modules: Option<&[&str]>) -> Self {
        let mut session = Session {
            arena: Arena::with_capacity(config.initial_arena_capacity),
            definitions: SessionDefinitions::new(),
            last_result: None,
            parse_cache: HashMap::default(),
            compiled_cache: HashMap::default(),
            eval_cache: HashMap::default(),
            compiled_results: HashMap::default(),
            output: Vec::new(),
            language_scopes: Vec::new(),
            module_assumptions: Vec::new(),
            source_name: None,
            definition_source: None,
            structures: HashMap::default(),
            package_path: default_package_path(),
            loaded_packages: HashSet::default(),
            loading_packages: Vec::new(),
            config,
        };
        session.load_standard_library(modules);
        session.install_package_path();
        session
    }

    fn install_package_path(&mut self) {
        let attrs = crate::builtins::attributes("$MithqalPath").unwrap_or_default();
        let symbol = self.arena.intern_symbol("$MithqalPath", attrs);
        let path = self.package_path.clone();
        let mut items = Vec::with_capacity(path.len());
        for entry in path {
            items.push(self.arena.intern_string(&entry));
        }
        let list_head = self.arena.builtin_head("list");
        let list = self.arena.intern_call(list_head, &items);
        let lhs = self.arena.symbol_expr(symbol);
        let _ = self.definitions.define(&self.arena, lhs, list, false);
    }

    fn load_standard_library(&mut self, modules: Option<&[&str]>) {
        let order = match modules {
            Some(modules) => stdlib::select_order(stdlib::MODULES, modules)
                .unwrap_or_else(|error| panic!("standard library: {error}")),
            None => stdlib::resolve_order(stdlib::MODULES)
                .unwrap_or_else(|error| panic!("standard library: {error}")),
        };
        for index in order {
            let module = &stdlib::MODULES[index];
            self.eval_named_source(module.name, module.source)
                .unwrap_or_else(|error| {
                    panic!(
                        "standard library module {} failed to evaluate: {error}",
                        module.name
                    )
                });
        }
        self.last_result = None;
        self.output.clear();
        self.collect_garbage(&[]);
    }

    fn privatize_module(&mut self, defined: &[String], exports: &[String], module: &str) {
        let mut renames = Vec::new();
        let mut targets: HashMap<SymbolId, SymbolId> = HashMap::default();
        for name in defined {
            let symbol = self.arena.symbol_or_intern(name);
            if exports.iter().any(|export| export == name) {
                targets.insert(symbol, symbol);
                continue;
            }
            let private = self.private_symbol(module, name);
            renames.push((symbol, private));
            targets.insert(symbol, private);
        }
        let mut taken = Vec::with_capacity(defined.len());
        for name in defined {
            let symbol = self.arena.symbol_or_intern(name);
            let rules = self.definitions.take_rules(symbol);
            let own = self.definitions.take_own(symbol);
            taken.push((symbol, rules, own));
        }
        for (symbol, mut rules, own) in taken {
            for (from, to) in &renames {
                for rule in &mut rules {
                    let (lhs, rhs) = crate::eval::rename_definition(
                        &mut self.arena,
                        rule.lhs,
                        rule.rhs,
                        *from,
                        *to,
                    );
                    rule.lhs = lhs;
                    rule.rhs = rhs;
                }
            }
            let target = targets.get(&symbol).copied().unwrap_or(symbol);
            self.definitions.install_rules(target, rules);
            if let Some(mut own) = own {
                for (from, to) in &renames {
                    own.value =
                        crate::eval::rename_expression(&mut self.arena, own.value, *from, *to);
                }
                self.definitions.install_own(target, own);
            }
            self.protect(target);
        }
    }

    fn private_symbol(&mut self, module: &str, name: &str) -> SymbolId {
        self.arena
            .intern_symbol_in_context(module, name, Attributes::new())
    }

    fn protect(&mut self, symbol: SymbolId) {
        let attributes = self.arena.symbol_attributes(symbol);
        self.arena
            .set_symbol_attributes(symbol, attributes.with(Attributes::PROTECTED));
    }

    /// Parse a single statement.
    pub fn parse_statement(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        let key = (source.to_string(), self.arena.context_version());
        if let Some(expression) = self.parse_cache.get(&key) {
            return Ok(*expression);
        }
        let expression = syntax::parse_statement(&mut self.arena, source)?;
        self.parse_cache.insert(key, expression);
        Ok(expression)
    }

    /// Parse one expression per statement.
    pub fn parse_source(&mut self, source: &str) -> Result<Vec<ExprId>, Diagnostic> {
        syntax::parse_source(&mut self.arena, source)
    }

    /// Parse and evaluate a single statement.
    pub fn eval_statement(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        let expression = self.parse_statement(source)?;
        self.eval(expression)
    }

    /// Load a package once by name or path.
    #[cfg(feature = "std")]
    pub fn needs(&mut self, name: &str) -> Result<ExprId, Diagnostic> {
        self.load_package(name, true)
    }

    /// Evaluate a package file without the once-only guard.
    #[cfg(feature = "std")]
    pub fn get(&mut self, path: &str) -> Result<ExprId, Diagnostic> {
        self.load_package(path, false)
    }

    fn load_package(&mut self, name: &str, once: bool) -> Result<ExprId, Diagnostic> {
        if bundled_module(name) {
            return Ok(self.arena.builtin_head("null"));
        }
        #[cfg(feature = "std")]
        {
            let path = self.resolve_package_path(name).ok_or_else(|| {
                Diagnostic::eval(
                    ErrorCode::EvalPackage,
                    format!("cannot find package {name}"),
                )
            })?;
            let key = std::fs::canonicalize(&path)
                .unwrap_or_else(|_| path.clone())
                .to_string_lossy()
                .into_owned();
            if once && self.loaded_packages.contains(&key) {
                return Ok(self.arena.builtin_head("null"));
            }
            if self.loading_packages.iter().any(|loading| loading == &key) {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalPackage,
                    format!("package import cycle at {}", path.display()),
                ));
            }
            self.loading_packages.push(key.clone());
            let source = std::fs::read_to_string(&path).map_err(|error| {
                Diagnostic::eval(
                    ErrorCode::EvalPackage,
                    format!("cannot read {}: {error}", path.display()),
                )
            });
            let result = match source {
                Ok(source) => self.eval_named_source(&path.to_string_lossy(), &source),
                Err(error) => Err(error),
            };
            self.loading_packages.pop();
            if result.is_ok() {
                self.loaded_packages.insert(key);
            }
            result
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = once;
            Err(Diagnostic::eval(
                ErrorCode::EvalPackage,
                format!("cannot load package {name} without std"),
            ))
        }
    }

    #[cfg(feature = "std")]
    fn resolve_package_path(&self, name: &str) -> Option<std::path::PathBuf> {
        let direct = std::path::PathBuf::from(name);
        if direct.exists() {
            return Some(direct);
        }
        if let Some(source) = &self.source_name
            && let Some(parent) = std::path::Path::new(source).parent()
        {
            let candidate = parent.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
            if !name.contains('/') && !name.ends_with(".mith") {
                let candidate = parent.join(format!("{name}.mith"));
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
        if name.contains('/') || name.ends_with(".mith") {
            return None;
        }
        for directory in &self.package_path {
            let candidate = std::path::Path::new(directory).join(format!("{name}.mith"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// Parse and evaluate a complete surface language module.
    ///
    /// A module with at least one `export` statement is strict: only exported
    /// definitions keep their names, and the other definitions are private to
    /// the module. A module without an `export` statement keeps every
    /// definition visible.
    pub fn eval_language(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        let program = syntax::parse_program(source)?;
        let defined = defined_names(&program);
        let exports = exported_names(&program);
        for name in &exports {
            if !defined.iter().any(|defined| defined == name) {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("module exports undefined function {name}"),
                ));
            }
        }
        for statement in &program.body {
            if let Stmt::Import { module } = statement {
                self.load_package(module.as_str(), true)?;
            }
        }
        let previous_source = self.definition_source;
        self.definition_source = self
            .source_name
            .as_deref()
            .map(|name| self.definitions.intern_source(name));
        self.language_scopes.push(HashMap::default());
        let result = self.eval_language_statements(&program.body, false);
        self.language_scopes.pop();
        self.definition_source = previous_source;
        if result.is_ok() && !exports.is_empty() {
            self.privatize_module(&defined, &exports, &program.name);
        }
        if let Ok(value) = result {
            self.last_result = Some(value);
            self.maybe_collect();
        }
        result
    }

    /// Printed output produced by `prt` statements.
    pub fn output(&self) -> &[String] {
        &self.output
    }

    /// Take the printed output produced by `prt` statements.
    pub fn take_output(&mut self) -> Vec<String> {
        core::mem::take(&mut self.output)
    }

    fn eval_language_statements(
        &mut self,
        statements: &[Stmt],
        local: bool,
    ) -> Result<ExprId, Diagnostic> {
        let mut last = None;
        for statement in statements {
            match self.eval_language_statement(statement, local) {
                Ok(value) => last = Some(value),
                Err(mut diagnostic) => {
                    if let Some((span, expression, head)) = statement.location() {
                        diagnostic.trace.push(Frame {
                            head,
                            expression,
                            span: Some(span),
                            source: self.source_name.clone(),
                        });
                    }
                    return Err(diagnostic);
                }
            }
        }
        Ok(last.unwrap_or_else(|| self.arena.builtin_head("null")))
    }

    fn eval_language_block(&mut self, statements: &[Stmt]) -> Result<ExprId, Diagnostic> {
        self.language_scopes.push(HashMap::default());
        self.definitions.push_scope();
        let result = self.eval_language_statements(statements, true);
        self.definitions.pop_scope();
        self.language_scopes.pop();
        result
    }

    fn compile_definition(
        &mut self,
        name: &str,
        parameters: &[PatternSource],
        guard: &Option<ExprSource>,
        body: &Body,
    ) -> Result<(ExprId, ExprId), Diagnostic> {
        self.compile_definition_with(name, parameters, guard, body, &[])
    }

    fn compile_definition_with(
        &mut self,
        name: &str,
        parameters: &[PatternSource],
        guard: &Option<ExprSource>,
        body: &Body,
        excluded: &[SymbolId],
    ) -> Result<(ExprId, ExprId), Diagnostic> {
        let blank_head = self.arena.builtin_head("blank");
        let blank = self.arena.intern_call(blank_head, &[]);
        let pattern_head = self.arena.builtin_head("pattern");
        let mut patterns = Vec::with_capacity(parameters.len());
        let mut parameter_symbols = Vec::new();
        for parameter in parameters {
            match parameter {
                PatternSource::Name(parameter) => {
                    let symbol = self.arena.symbol_or_intern(parameter);
                    parameter_symbols.push(symbol);
                    let name = self.arena.symbol_expr(symbol);
                    patterns.push(self.arena.intern_call(pattern_head, &[name, blank]));
                }
                PatternSource::Literal(source) => {
                    let expression = self.language_expression(source)?;
                    self.collect_pattern_symbols(expression, &mut parameter_symbols);
                    patterns.push(expression);
                }
            }
        }
        let mut scope = parameter_symbols;
        for symbol in excluded {
            if !scope.contains(symbol) {
                scope.push(*symbol);
            }
        }
        let body = self.resolve_body(body, &scope)?;
        let guard = if let Some(guard) = guard {
            Some(self.resolved_language_expression(guard, &scope)?)
        } else {
            None
        };
        let symbol = self.arena.symbol_or_intern(name);
        let head = self.arena.symbol_expr(symbol);
        let mut lhs = self.arena.intern_call(head, &patterns);
        if let Some(guard) = guard {
            let condition = self.arena.builtin_head("condition");
            lhs = self.arena.intern_call(condition, &[lhs, guard]);
        }
        Ok((lhs, body))
    }

    fn ensure_name_free(&mut self, name: &str) -> Result<SymbolId, Diagnostic> {
        if crate::eval::is_kernel_head(name) {
            return Err(Diagnostic::eval(
                ErrorCode::EvalProtected,
                format!("symbol {name} is protected"),
            ));
        }
        let symbol = self.arena.symbol_or_intern(name);
        if self.definitions.own_value(symbol).is_some()
            || !self.definitions.rules(symbol).is_empty()
            || self
                .arena
                .symbol_attributes(symbol)
                .has(Attributes::PROTECTED)
        {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                format!("symbol {name} is already defined"),
            ));
        }
        Ok(symbol)
    }

    fn declare_structure(
        &mut self,
        name: &str,
        fields: &[String],
        laws: &[(String, ExprSource)],
    ) -> Result<(), Diagnostic> {
        if self.structures.contains_key(name) {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                format!("structure {name} is already declared"),
            ));
        }
        let field_symbols: Vec<SymbolId> = fields
            .iter()
            .map(|field| self.arena.symbol_or_intern(field))
            .collect();
        let mut generated = vec![name.to_string(), format!("{name}Holds")];
        for field in fields {
            generated.push(format!("{name}{}", capitalize(field)));
        }
        for generated_name in &generated {
            self.ensure_name_free(generated_name)?;
        }
        let mut parts = vec![name.to_string()];
        parts.extend(fields.iter().cloned());
        let body = Body::Expression(ExprSource {
            text: format!("[{}]", parts.join(",")),
            line: 1,
            column: 1,
        });
        let parameters: Vec<PatternSource> = fields
            .iter()
            .map(|field| PatternSource::Name(field.clone()))
            .collect();
        let (lhs, rhs) = self.compile_definition_with(name, &parameters, &None, &body, &[])?;
        self.definitions.define(&self.arena, lhs, rhs, true)?;
        for (index, field) in fields.iter().enumerate() {
            let accessor = format!("{name}{}", capitalize(field));
            let body = Body::Expression(ExprSource {
                text: format!("nth(s,{})", index + 1),
                line: 1,
                column: 1,
            });
            let parameters = vec![PatternSource::Name("s".to_string())];
            let (lhs, rhs) =
                self.compile_definition_with(&accessor, &parameters, &None, &body, &[])?;
            self.definitions.define(&self.arena, lhs, rhs, true)?;
        }
        let text = if laws.is_empty() {
            "true".to_string()
        } else {
            let bindings = fields
                .iter()
                .enumerate()
                .map(|(index, field)| format!("{field}=nth(s,{})", index + 1))
                .collect::<Vec<_>>()
                .join(",");
            let conditions = laws
                .iter()
                .map(|(_, law)| law.text.clone())
                .collect::<Vec<_>>()
                .join(",");
            format!("let([{bindings}],and({conditions}))")
        };
        let body = Body::Expression(ExprSource {
            text,
            line: 1,
            column: 1,
        });
        let parameters = vec![PatternSource::Name("s".to_string())];
        let holds = format!("{name}Holds");
        let (lhs, rhs) =
            self.compile_definition_with(&holds, &parameters, &None, &body, &field_symbols)?;
        self.definitions.define(&self.arena, lhs, rhs, true)?;
        self.structures.insert(name.to_string(), fields.to_vec());
        Ok(())
    }

    fn declare_instance(
        &mut self,
        name: &str,
        structure: &str,
        assumed: bool,
        fields: &[(String, ExprSource)],
    ) -> Result<(), Diagnostic> {
        let Some(schema) = self.structures.get(structure).cloned() else {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                format!("unknown structure {structure}"),
            ));
        };
        if fields.len() != schema.len() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                format!("instance {name} does not provide every field of {structure}"),
            ));
        }
        for (field, _) in fields {
            if !schema.iter().any(|schema_field| schema_field == field) {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("instance {name} has unknown field {field}"),
                ));
            }
        }
        let mut values = Vec::with_capacity(schema.len());
        for field in &schema {
            let Some((_, source)) = fields.iter().find(|(name, _)| name == field) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("instance {name} is missing field {field}"),
                ));
            };
            let value = self.resolved_language_expression(source, &[])?;
            values.push(self.eval(value)?);
        }
        let tag = self.arena.symbol_or_intern(structure);
        let tag = self.arena.symbol_expr(tag);
        let mut parts = Vec::with_capacity(values.len() + 1);
        parts.push(tag);
        parts.extend(values);
        let list = self.arena.builtin_head("list");
        let dictionary = self.arena.intern_call(list, &parts);
        if !assumed {
            let holds = format!("{structure}Holds");
            let holds_symbol = self.arena.symbol_or_intern(&holds);
            let holds = self.arena.symbol_expr(holds_symbol);
            let call = self.arena.intern_call(holds, &[dictionary]);
            let result = self.eval(call)?;
            let true_value = self.arena.builtin_head("true");
            let false_value = self.arena.builtin_head("false");
            if result == false_value {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("instance {name} does not satisfy the laws of {structure}"),
                ));
            }
            if result != true_value {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("instance {name} laws are undecidable; declare the instance assumed"),
                ));
            }
        }
        let symbol = self.ensure_name_free(name)?;
        let symbol = self.arena.symbol_expr(symbol);
        self.definitions
            .define(&self.arena, symbol, dictionary, false)?;
        Ok(())
    }

    fn eval_language_statement(
        &mut self,
        statement: &Stmt,
        local: bool,
    ) -> Result<ExprId, Diagnostic> {
        match statement {
            Stmt::Def {
                name,
                parameters,
                guard,
                body,
            } => {
                let (lhs, rhs) = self.compile_definition(name, parameters, guard, body)?;
                {
                    let arena = &self.arena;
                    if local {
                        self.definitions.define_local(arena, lhs, rhs)?;
                    } else {
                        self.definitions.define(arena, lhs, rhs, true)?;
                    }
                }
                if let (Some(source), Some((span, _, _))) =
                    (self.definition_source, statement.location())
                {
                    let arena = &self.arena;
                    self.definitions.annotate_last(
                        arena,
                        lhs,
                        local,
                        crate::eval::DefinitionSite { span, source },
                    );
                }
                Ok(self.arena.builtin_head("null"))
            }
            Stmt::Let { name, body } => {
                let value = self.resolved_language_expression(body, &[])?;
                let value = self.eval(value)?;
                let symbol = self.arena.symbol_or_intern(name);
                if let Some(scope) = self.language_scopes.last_mut() {
                    scope.insert(symbol, value);
                } else {
                    let mut scope = HashMap::default();
                    scope.insert(symbol, value);
                    self.language_scopes.push(scope);
                }
                Ok(value)
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.resolved_language_expression(condition, &[])?;
                let condition = self.eval(condition)?;
                let truth = self.language_boolean(condition)?;
                if truth {
                    self.eval_language_block(then_body)
                } else {
                    self.eval_language_block(else_body)
                }
            }
            Stmt::Match {
                scrutinee,
                cases,
                else_body,
            } => {
                let scrutinee = self.resolved_language_expression(scrutinee, &[])?;
                let scrutinee = self.eval(scrutinee)?;
                for case in cases {
                    if self.eval_case(case, scrutinee)? {
                        let result = self.eval_language_statements(&case.body, true);
                        self.definitions.pop_scope();
                        self.language_scopes.pop();
                        return result;
                    }
                }
                self.eval_language_block(else_body)
            }
            Stmt::Print { body } => {
                let value = self.resolved_language_expression(body, &[])?;
                let value = self.eval(value)?;
                self.output.push(self.format_infix(value)?);
                Ok(value)
            }
            Stmt::Structure { name, fields, laws } => {
                if local {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "a structure must appear at module level",
                    ));
                }
                self.declare_structure(name, fields, laws)?;
                Ok(self.arena.builtin_head("null"))
            }
            Stmt::Instance {
                name,
                structure,
                assumed,
                fields,
            } => {
                if local {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "an instance must appear at module level",
                    ));
                }
                self.declare_instance(name, structure, *assumed, fields)?;
                Ok(self.arena.builtin_head("null"))
            }
            Stmt::Assume { condition } => {
                if local {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "an assumption must appear at module level",
                    ));
                }
                let formula = self.language_expression(condition)?;
                self.module_assumptions.push(formula);
                Ok(self.arena.builtin_head("null"))
            }
            Stmt::Theorem { name, claim, proof } => {
                if local {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "a theorem must appear at module level",
                    ));
                }
                self.verify_theorem(name, claim, proof)?;
                Ok(self.arena.builtin_head("null"))
            }
            Stmt::Export { .. } | Stmt::Import { .. } => {
                if local {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "export and import must appear at module level",
                    ));
                }
                Ok(self.arena.builtin_head("null"))
            }
        }
    }

    fn verify_theorem(
        &mut self,
        name: &str,
        claim: &syntax::ExprSource,
        proof: &[syntax::ExprSource],
    ) -> Result<(), Diagnostic> {
        if proof.is_empty() {
            return Ok(());
        }
        let mut assumptions = Vec::new();
        let mut facts: Vec<(String, String)> = Vec::new();
        let mut case_specs: Vec<(String, Vec<String>)> = Vec::new();
        let mut contradiction = false;
        let mut have_facts: Vec<(String, String)> = Vec::new();
        for step in proof {
            let step_lower = step.text.to_ascii_lowercase();
            if let Some(rest) = step_lower.strip_prefix("assume ") {
                let condition = rest.trim().to_string();
                if let Some(fact) = equality_fact(&condition) {
                    facts.push(fact);
                } else {
                    assumptions.push(syntax::normalize_formula(&condition)?);
                }
                continue;
            }
            if let Some(rest) = step_lower.strip_prefix("have ") {
                let Some(colon) = rest.find(':') else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        format!("theorem {name} has an invalid have step"),
                    ));
                };
                let fact_name = rest[..colon].trim().to_string();
                let claim_text = rest[colon + 1..].trim().to_string();
                self.verify_claim(
                    name,
                    &claim_text,
                    step.line,
                    step.column,
                    &assumptions,
                    &facts,
                )?;
                if let Some(fact) = equality_fact(&claim_text) {
                    facts.push(fact);
                }
                have_facts.push((fact_name, claim_text));
                continue;
            }
            if let Some(rest) = step_lower.strip_prefix("show ") {
                let claim_text = rest.trim().to_string();
                self.verify_claim(
                    name,
                    &claim_text,
                    step.line,
                    step.column,
                    &assumptions,
                    &facts,
                )?;
                if let Some(fact) = equality_fact(&claim_text) {
                    facts.push(fact);
                }
                continue;
            }
            if step_lower.trim() == "contradiction" {
                self.verify_contradiction(name, &assumptions)?;
                contradiction = true;
                continue;
            }
            if let Some(rest) = step_lower.strip_prefix("by cases on ") {
                let Some(in_position) = rest.find(" in ") else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        format!("theorem {name} has an invalid cases step"),
                    ));
                };
                let variable = rest[..in_position].trim().to_string();
                let domain = rest[in_position + 4..].trim();
                if !domain.starts_with('[') || !domain.ends_with(']') {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        format!("theorem {name} cases domain must be a finite list"),
                    ));
                }
                let values = split_list_items(&domain[1..domain.len() - 1]);
                case_specs.push((variable, values));
                continue;
            }
            if let Some(reference) = step_lower.strip_prefix("by ") {
                let reference = reference.trim();
                if have_facts
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case(reference))
                {
                    continue;
                }
            }
            let supported = step_lower.starts_with("by definition")
                || step_lower.starts_with("by arithmetic")
                || step_lower.starts_with("by assumption")
                || step_lower.starts_with("by rewriting")
                || step_lower.starts_with("by the definition");
            if !supported {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("theorem {name} has an unsupported proof step"),
                ));
            }
        }
        if contradiction {
            return Ok(());
        }
        if !case_specs.is_empty() {
            for (variable, values) in &case_specs {
                for value in values {
                    let mut case_facts = facts.clone();
                    case_facts.push((variable.clone(), value.clone()));
                    self.verify_claim(
                        name,
                        &claim.text,
                        claim.line,
                        claim.column,
                        &assumptions,
                        &case_facts,
                    )?;
                }
            }
            return Ok(());
        }
        self.verify_claim(
            name,
            &claim.text,
            claim.line,
            claim.column,
            &assumptions,
            &facts,
        )
    }

    fn verify_contradiction(
        &mut self,
        name: &str,
        assumptions: &[String],
    ) -> Result<(), Diagnostic> {
        if assumptions.is_empty() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                format!("theorem {name} has no assumptions to contradict"),
            ));
        }
        let mut conjunction = assumptions.last().cloned().unwrap_or_default();
        for assumption in assumptions.iter().rev().skip(1) {
            conjunction = format!("and({assumption},{conjunction})");
        }
        let source = syntax::ExprSource {
            text: format!("assume({conjunction},false)"),
            line: 1,
            column: 1,
        };
        let steps = self.config.max_steps;
        let depth = self.config.max_depth;
        self.config.max_steps = self.config.max_steps.max(1_000_000);
        self.config.max_depth = self.config.max_depth.max(2048);
        let result = (|| {
            let expression = self.resolved_language_expression(&source, &[])?;
            match self.eval(expression) {
                Err(error) if error.code == ErrorCode::EvalAssumption => Ok(()),
                Ok(_) => Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("theorem {name} assumptions are consistent"),
                )),
                Err(error) => Err(error),
            }
        })();
        self.config.max_steps = steps;
        self.config.max_depth = depth;
        result
    }

    fn verify_claim(
        &mut self,
        name: &str,
        claim: &str,
        line: usize,
        column: usize,
        assumptions: &[String],
        facts: &[(String, String)],
    ) -> Result<(), Diagnostic> {
        let lower = claim.to_ascii_lowercase();
        if lower.trim_start().starts_with("there exists ") {
            return self.verify_existence(name, claim, line, column, assumptions, facts);
        }
        if lower.trim_start().starts_with("for every ")
            && self.verify_finite_universal(name, claim, line, column, assumptions, facts)?
        {
            return Ok(());
        }
        self.verify_equality(name, claim, line, column, assumptions, facts)
    }

    fn verify_finite_universal(
        &mut self,
        name: &str,
        claim: &str,
        line: usize,
        column: usize,
        assumptions: &[String],
        facts: &[(String, String)],
    ) -> Result<bool, Diagnostic> {
        let lower = claim.to_ascii_lowercase();
        let Some(rest) = lower.strip_prefix("for every ") else {
            return Ok(false);
        };
        let Some(in_position) = rest.find(" in ") else {
            return Ok(false);
        };
        let after = &rest[in_position + 4..];
        let Some(comma) = find_top_level_comma(after) else {
            return Ok(false);
        };
        let domain = after[..comma].trim();
        if !domain.starts_with('[') {
            return Ok(false);
        }
        let variable = rest[..in_position].trim();
        if !variable
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
        {
            return Ok(false);
        }
        let condition = after[comma + 1..].trim();
        let condition = rewrite_facts(condition, facts);
        let condition = wrap_assumptions(&condition, assumptions);
        let expression = format!(
            "forall({},{},{})",
            variable,
            syntax::normalize_formula(domain)?,
            syntax::normalize_formula(&condition)?
        );
        let source = syntax::ExprSource {
            text: expression,
            line,
            column,
        };
        let steps = self.config.max_steps;
        let depth = self.config.max_depth;
        self.config.max_steps = self.config.max_steps.max(1_000_000);
        self.config.max_depth = self.config.max_depth.max(2048);
        let result = (|| {
            let expression = self.resolved_language_expression(&source, &[])?;
            let value = self.eval(expression)?;
            if value == self.arena.builtin_head("true") {
                Ok(true)
            } else {
                Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("theorem {name} universal claim could not be verified"),
                ))
            }
        })();
        self.config.max_steps = steps;
        self.config.max_depth = depth;
        result
    }

    fn verify_existence(
        &mut self,
        name: &str,
        claim: &str,
        line: usize,
        column: usize,
        assumptions: &[String],
        facts: &[(String, String)],
    ) -> Result<(), Diagnostic> {
        let lower = claim.to_ascii_lowercase();
        let Some(rest) = lower.strip_prefix("there exists ") else {
            return Ok(());
        };
        let Some(in_position) = rest.find(" in ") else {
            return Ok(());
        };
        let Some(such_position) = rest.find(" such that ") else {
            return Ok(());
        };
        if such_position <= in_position {
            return Ok(());
        }
        let variable = rest[..in_position].trim();
        let domain = rest[in_position + 4..such_position].trim();
        let condition = rest[such_position + 11..].trim();
        if !variable
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
        {
            return Ok(());
        }
        let condition = rewrite_facts(condition, facts);
        let condition = wrap_assumptions(&condition, assumptions);
        let expression = format!(
            "exists({},{},{})",
            variable,
            syntax::normalize_formula(domain)?,
            syntax::normalize_formula(&condition)?
        );
        let source = syntax::ExprSource {
            text: expression,
            line,
            column,
        };
        let steps = self.config.max_steps;
        let depth = self.config.max_depth;
        self.config.max_steps = self.config.max_steps.max(1_000_000);
        self.config.max_depth = self.config.max_depth.max(2048);
        let result = (|| {
            let expression = self.resolved_language_expression(&source, &[])?;
            let value = self.eval(expression)?;
            if value == self.arena.builtin_head("true") {
                Ok(())
            } else {
                Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("theorem {name} existential claim could not be verified"),
                ))
            }
        })();
        self.config.max_steps = steps;
        self.config.max_depth = depth;
        result
    }

    fn verify_equality(
        &mut self,
        name: &str,
        claim: &str,
        line: usize,
        column: usize,
        assumptions: &[String],
        facts: &[(String, String)],
    ) -> Result<(), Diagnostic> {
        let lower = claim.to_ascii_lowercase();
        if lower.contains("there exists") {
            return Ok(());
        }
        let claim_text = strip_universal_quantifier(claim);
        let claim_text = rewrite_facts(claim_text, facts);
        let normalized = syntax::normalize_formula(&claim_text).map_err(|error| {
            Diagnostic::eval(
                ErrorCode::EvalDefinition,
                format!("theorem {name} claim is not a formula: {error}"),
            )
        })?;
        let steps = self.config.max_steps;
        let depth = self.config.max_depth;
        self.config.max_steps = self.config.max_steps.max(1_000_000);
        self.config.max_depth = self.config.max_depth.max(2048);
        let result = (|| {
            if let Some(equals) = claim_equals(&normalized) {
                let left = syntax::ExprSource {
                    text: wrap_assumptions(&normalized[..equals], assumptions),
                    line,
                    column,
                };
                let right = syntax::ExprSource {
                    text: wrap_assumptions(&normalized[equals + 1..], assumptions),
                    line,
                    column,
                };
                let left = self.resolved_language_expression(&left, &[])?;
                let left = self.eval(left)?;
                let right = self.resolved_language_expression(&right, &[])?;
                let right = self.eval(right)?;
                if left != right {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        format!("theorem {name} claim could not be verified"),
                    ));
                }
                return Ok(());
            }
            let source = syntax::ExprSource {
                text: wrap_assumptions(&normalized, assumptions),
                line,
                column,
            };
            let condition = self.resolved_language_expression(&source, &[])?;
            let value = self.eval(condition)?;
            if value == self.arena.builtin_head("true") {
                Ok(())
            } else {
                Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("theorem {name} claim could not be verified"),
                ))
            }
        })();
        self.config.max_steps = steps;
        self.config.max_depth = depth;
        result
    }

    fn eval_case(&mut self, case: &Case, scrutinee: ExprId) -> Result<bool, Diagnostic> {
        self.language_scopes.push(HashMap::default());
        let matched = match &case.pattern {
            PatternSource::Name(name) => {
                let symbol = self.arena.symbol_or_intern(name);
                if let Some(scope) = self.language_scopes.last_mut() {
                    scope.insert(symbol, scrutinee);
                }
                true
            }
            PatternSource::Literal(source) => {
                let literal = self.language_expression(source)?;
                let literal = self.eval(literal)?;
                literal == scrutinee
            }
        };
        if matched {
            self.definitions.push_scope();
        } else {
            self.language_scopes.pop();
        }
        Ok(matched)
    }

    fn language_expression(&mut self, source: &ExprSource) -> Result<ExprId, Diagnostic> {
        match syntax::parse_statement(&mut self.arena, &source.text) {
            Ok(expression) => Ok(expression),
            Err(mut diagnostic) => {
                if let Some(span) = diagnostic.span.as_deref_mut()
                    && span.line == 1
                {
                    span.line = source.line;
                    span.column = span.column.saturating_add(source.column - 1);
                }
                Err(diagnostic)
            }
        }
    }

    fn resolved_language_expression(
        &mut self,
        source: &ExprSource,
        excluded: &[SymbolId],
    ) -> Result<ExprId, Diagnostic> {
        let expression = self.language_expression(source)?;
        Ok(self.resolve_expression(expression, excluded))
    }

    fn resolve_expression(&mut self, expression: ExprId, excluded: &[SymbolId]) -> ExprId {
        match self.arena.node(expression) {
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                if excluded.contains(&symbol) {
                    return expression;
                }
                for scope in self.language_scopes.iter().rev() {
                    if let Some(value) = scope.get(&symbol) {
                        return *value;
                    }
                }
                expression
            }
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                let head = self.resolve_expression(head, excluded);
                let mut resolved = Vec::with_capacity(args.len());
                for arg in args {
                    resolved.push(self.resolve_expression(arg, excluded));
                }
                self.arena.intern_call(head, &resolved)
            }
            _ => expression,
        }
    }

    fn collect_pattern_symbols(&self, expression: ExprId, symbols: &mut Vec<SymbolId>) {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return;
        };
        if args.len() == 2
            && self
                .arena
                .symbol_id(*head)
                .map(|symbol| self.arena.symbol_name(symbol))
                == Some("pattern")
            && let Some(symbol) = self.arena.symbol_id(args[0])
            && !symbols.contains(&symbol)
        {
            symbols.push(symbol);
        }
        for argument in args {
            self.collect_pattern_symbols(*argument, symbols);
        }
    }

    fn resolve_body(&mut self, body: &Body, parameters: &[SymbolId]) -> Result<ExprId, Diagnostic> {
        match body {
            Body::Expression(source) => self.resolved_language_expression(source, parameters),
            Body::Block(statements) => self.compile_body(statements, parameters),
        }
    }

    fn compile_body(
        &mut self,
        statements: &[BodyStmt],
        parameters: &[SymbolId],
    ) -> Result<ExprId, Diagnostic> {
        let Some((first, rest)) = statements.split_first() else {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "a definition body is empty",
            ));
        };
        if let BodyStmt::Def { .. } = first {
            let mut entries = Vec::new();
            let mut tail = Vec::new();
            let mut leading = true;
            for statement in statements {
                match statement {
                    BodyStmt::Def {
                        name,
                        parameters,
                        guard,
                        body,
                    } if leading => {
                        let (lhs, rhs) = self.compile_definition(name, parameters, guard, body)?;
                        let set_delayed = self.arena.builtin_head("setDelayed");
                        entries.push(self.arena.intern_call(set_delayed, &[lhs, rhs]));
                    }
                    BodyStmt::Def { .. } => {
                        return Err(Diagnostic::eval(
                            ErrorCode::EvalDefinition,
                            "local definitions must appear before other body statements",
                        ));
                    }
                    other => {
                        leading = false;
                        tail.push(other.clone());
                    }
                }
            }
            if tail.is_empty() {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "a definition body ends with one expression",
                ));
            }
            let tail = self.compile_body(&tail, parameters)?;
            let list = self.arena.builtin_head("list");
            let entries = self.arena.intern_call(list, &entries);
            let local_defs = self.arena.builtin_head("localDefs");
            return Ok(self.arena.intern_call(local_defs, &[entries, tail]));
        }
        match first {
            BodyStmt::Expr(source) => {
                if !rest.is_empty() {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "a definition body ends with one expression",
                    ));
                }
                self.resolved_language_expression(source, parameters)
            }
            BodyStmt::Let { name, body } => {
                let value = self.resolved_language_expression(body, parameters)?;
                let tail = self.compile_body(rest, parameters)?;
                let name = self.arena.symbol_or_intern(name);
                let name = self.arena.symbol_expr(name);
                let set = self.arena.builtin_head("set");
                let binding = self.arena.intern_call(set, &[name, value]);
                let list = self.arena.builtin_head("list");
                let bindings = self.arena.intern_call(list, &[binding]);
                let let_head = self.arena.builtin_head("let");
                Ok(self.arena.intern_call(let_head, &[bindings, tail]))
            }
            BodyStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                if !rest.is_empty() {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "an if statement must be the last statement in a definition body",
                    ));
                }
                let condition = self.resolved_language_expression(condition, parameters)?;
                let then_value = self.compile_body(then_body, parameters)?;
                let else_value = if else_body.is_empty() {
                    self.arena.builtin_head("null")
                } else {
                    self.compile_body(else_body, parameters)?
                };
                let if_head = self.arena.builtin_head("if");
                Ok(self
                    .arena
                    .intern_call(if_head, &[condition, then_value, else_value]))
            }
            BodyStmt::Def { .. } => unreachable!("local definitions are grouped above"),
        }
    }

    fn language_boolean(&self, value: ExprId) -> Result<bool, Diagnostic> {
        if let Node::Symbol(symbol) = self.arena.node(value) {
            match self.arena.symbol_name(*symbol) {
                "true" => return Ok(true),
                "false" => return Ok(false),
                _ => {}
            }
        }
        Err(Diagnostic::eval(
            crate::diag::ErrorCode::EvalDefinition,
            "a conditional requires true or false",
        ))
    }

    /// Parse and evaluate every statement, returning the final value.
    pub fn eval_named_source(&mut self, name: &str, source: &str) -> Result<ExprId, Diagnostic> {
        let previous = self.source_name.replace(name.to_string());
        let result = self.eval_source(source);
        self.source_name = previous;
        result.map_err(|diagnostic| attach_source(diagnostic, name))
    }

    /// Read and evaluate a source file.
    #[cfg(feature = "std")]
    pub fn eval_file(&mut self, path: &str) -> Result<ExprId, Diagnostic> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            Diagnostic::eval(
                ErrorCode::EvalDefinition,
                format!("cannot read {path}: {error}"),
            )
            .with_source(path)
        })?;
        self.eval_named_source(path, &source)
    }

    /// Parse and evaluate every statement, returning the final value.
    pub fn eval_source(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        if syntax::looks_like_prose_program(source) {
            return self.eval_language(source);
        }
        let statements = self.parse_source(source)?;
        if statements.is_empty() {
            return Err(Diagnostic::parse(
                ErrorCode::ParseUnexpectedToken,
                "expected a statement",
                Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            ));
        }
        let mut last = None;
        for statement in statements {
            last = Some(self.eval(statement)?);
        }
        Ok(last.expect("statements is not empty"))
    }

    fn with_module_assumptions(&mut self, expression: ExprId) -> ExprId {
        let assumptions = self.module_assumptions.clone();
        let mut result = expression;
        let head = self.arena.builtin_head("assume");
        for assumption in assumptions {
            result = self.arena.intern_call(head, &[assumption, result]);
        }
        result
    }

    /// Evaluate an expression to a normal form.
    pub fn eval(&mut self, expression: ExprId) -> Result<ExprId, Diagnostic> {
        let expression = self.with_module_assumptions(expression);
        if !self.is_live(expression) {
            return Err(stale_handle());
        }
        let version = self.definitions.version();
        let context_version = self.arena.context_version();
        if let Some(value) = self.eval_cache.get(&(expression, version, context_version)) {
            self.last_result = Some(*value);
            return Ok(*value);
        }
        let value = evaluate_with_definitions(
            &mut self.arena,
            &mut self.definitions,
            expression,
            self.config.max_steps,
            self.config.max_depth,
        )?;
        self.eval_cache
            .insert((expression, version, context_version), value);
        self.last_result = Some(value);
        self.maybe_collect();
        Ok(value)
    }

    /// Evaluate an expression through the compiled instruction path.
    pub fn eval_compiled(&mut self, expression: ExprId) -> Result<ExprId, Diagnostic> {
        if !self.is_live(expression) {
            return Err(stale_handle());
        }
        let version = self.definitions.version();
        let context_version = self.arena.context_version();
        if let Some(value) = self
            .compiled_results
            .get(&(expression, version, context_version))
        {
            self.last_result = Some(*value);
            return Ok(*value);
        }
        let code = if let Some(code) = self.compiled_cache.get(&expression) {
            code.clone()
        } else {
            let code = runtime::compile(&mut self.arena, expression)?;
            self.compiled_cache.insert(expression, code.clone());
            code
        };
        let value = runtime::execute(
            &mut self.arena,
            &mut self.definitions,
            &code,
            self.config.max_steps,
            self.config.max_depth,
        )?;
        self.compiled_results
            .insert((expression, version, context_version), value);
        self.last_result = Some(value);
        self.maybe_collect();
        Ok(value)
    }

    fn maybe_collect(&mut self) {
        if !self.language_scopes.is_empty() {
            return;
        }
        if self.arena.len() > self.config.gc_threshold_nodes.max(1) {
            self.collect_garbage(&[]);
        }
    }

    /// Parse and evaluate a single statement through the compiled path.
    pub fn eval_compiled_statement(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        let expression = self.parse_statement(source)?;
        self.eval_compiled(expression)
    }

    /// True when the handle still refers to a live expression.
    pub fn is_live(&self, expression: ExprId) -> bool {
        self.arena.try_node(expression).is_some()
    }

    /// Render an expression in canonical form.
    pub fn format(&self, expression: ExprId) -> Result<String, Diagnostic> {
        if !self.is_live(expression) {
            return Err(stale_handle());
        }
        Ok(print::canonical(&self.arena, expression))
    }

    /// Render an expression in human surface form.
    pub fn format_infix(&self, expression: ExprId) -> Result<String, Diagnostic> {
        if !self.is_live(expression) {
            return Err(stale_handle());
        }
        Ok(print::infix(&self.arena, expression))
    }

    /// Inspect an expression.
    pub fn view(&self, expression: ExprId) -> Result<ExprView<'_>, Diagnostic> {
        let node = self.arena.try_node(expression).ok_or_else(stale_handle)?;
        Ok(match node {
            Node::Int(value) => ExprView::Integer(value),
            Node::Rat(value) => ExprView::Rational(value),
            Node::Real(value) => ExprView::Real(value),
            Node::String(value) => ExprView::String(value),
            Node::Bytes(value) => ExprView::Bytes(value),
            Node::Symbol(symbol) => ExprView::Symbol(self.arena.symbol_name(*symbol)),
            Node::Call { head, args } => ExprView::Call {
                head: *head,
                args: args.as_ref(),
            },
            Node::Association(entries) => ExprView::Association(entries),
        })
    }

    /// Reclaim expressions not reachable from definitions, the last result,
    /// active language scope values, or the supplied roots.
    pub fn collect_garbage(&mut self, roots: &[ExprId]) {
        let mut all = roots.to_vec();
        if let Some(last) = self.last_result {
            all.push(last);
        }
        for scope in &self.language_scopes {
            all.extend(scope.values().copied());
        }
        self.definitions.roots(&mut all);
        self.arena.collect(&all);
        self.parse_cache.clear();
        self.compiled_cache.clear();
        self.eval_cache.clear();
        self.compiled_results.clear();
    }

    /// Number of live expression nodes in the session arena.
    pub fn live_expressions(&self) -> usize {
        self.arena.len()
    }

    /// Move emitted messages out of the session.
    pub fn take_messages(&mut self) -> Vec<Message> {
        self.arena.take_messages()
    }

    /// The value of the most recent evaluation.
    pub fn last_result(&self) -> Option<ExprId> {
        self.last_result
    }
}

fn default_package_path() -> Vec<String> {
    #[cfg(feature = "std")]
    let mut path = vec![".".to_string()];
    #[cfg(not(feature = "std"))]
    let path = vec![".".to_string()];
    #[cfg(feature = "std")]
    if let Ok(extra) = std::env::var("MITHQAL_PATH") {
        for entry in extra.split(':') {
            if !entry.is_empty() && !path.iter().any(|known| known == entry) {
                path.push(entry.to_string());
            }
        }
    }
    path
}

fn bundled_module(name: &str) -> bool {
    stdlib::MODULES.iter().any(|module| module.name == name)
}

fn equality_fact(claim: &str) -> Option<(String, String)> {
    let normalized = syntax::normalize_formula(claim).ok()?;
    let equals = claim_equals(&normalized)?;
    let left = normalized[..equals].trim();
    let right = normalized[equals + 1..].trim();
    let symbol = |text: &str| {
        !text.is_empty()
            && text.chars().all(|character| {
                character.is_alphanumeric() || character == '_' || character == '$'
            })
            && !text
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
    };
    if symbol(left) {
        Some((left.to_string(), right.to_string()))
    } else if symbol(right) {
        Some((right.to_string(), left.to_string()))
    } else {
        None
    }
}

fn rewrite_facts(text: &str, facts: &[(String, String)]) -> String {
    let mut result = text.to_string();
    for (symbol, replacement) in facts {
        let chars: Vec<char> = result.chars().collect();
        let mut next = String::with_capacity(result.len());
        let mut index = 0usize;
        while index < chars.len() {
            let character = chars[index];
            if character.is_alphanumeric() || character == '_' || character == '$' {
                let start = index;
                while chars
                    .get(index)
                    .is_some_and(|value| value.is_alphanumeric() || *value == '_' || *value == '$')
                {
                    index += 1;
                }
                let word: String = chars[start..index].iter().collect();
                if &word == symbol {
                    next.push('(');
                    next.push_str(replacement);
                    next.push(')');
                } else {
                    next.push_str(&word);
                }
                continue;
            }
            next.push(character);
            index += 1;
        }
        result = next;
    }
    result
}

fn wrap_assumptions(expression: &str, assumptions: &[String]) -> String {
    let mut result = expression.to_string();
    for assumption in assumptions.iter().rev() {
        result = format!("assume({assumption},{result})");
    }
    result
}

fn strip_universal_quantifier(text: &str) -> &str {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("for every ") {
        return trimmed;
    }
    let mut depth = 0i32;
    for (index, character) in trimmed.char_indices() {
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => return trimmed[index + 1..].trim(),
            _ => {}
        }
    }
    trimmed
}

fn split_list_items(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_string = false;
    for character in text.chars() {
        if character == '"' {
            in_string = !in_string;
        }
        if !in_string {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    items.push(current.trim().to_string());
                    current.clear();
                    continue;
                }
                _ => {}
            }
        }
        current.push(character);
    }
    if !current.trim().is_empty() {
        items.push(current.trim().to_string());
    }
    items
}

fn find_top_level_comma(text: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    for (index, character) in text.char_indices() {
        if character == '"' {
            in_string = !in_string;
        }
        if in_string {
            continue;
        }
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn claim_equals(text: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    for (index, character) in text.char_indices() {
        if character == '"' {
            in_string = !in_string;
        }
        if in_string {
            continue;
        }
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if character == '=' && depth == 0 {
            let previous = text[..index].chars().next_back();
            let next = text[index + 1..].chars().next();
            if previous == Some('=')
                || previous == Some('/')
                || previous == Some('<')
                || previous == Some('>')
                || previous == Some('!')
                || next == Some('=')
            {
                continue;
            }
            return Some(index);
        }
    }
    None
}

fn defined_names(program: &syntax::Program) -> Vec<String> {
    let mut names = Vec::new();
    for statement in &program.body {
        match statement {
            Stmt::Def { name, .. } if !names.contains(name) => {
                names.push(name.clone());
            }
            Stmt::Let { name, .. } if !names.contains(name) => {
                names.push(name.clone());
            }
            Stmt::Structure { name, fields, .. } => {
                for generated in core::iter::once(name.clone())
                    .chain(core::iter::once(format!("{name}Holds")))
                    .chain(
                        fields
                            .iter()
                            .map(|field| format!("{name}{}", capitalize(field))),
                    )
                {
                    if !names.contains(&generated) {
                        names.push(generated);
                    }
                }
            }
            Stmt::Instance { name, .. } if !names.contains(name) => {
                names.push(name.clone());
            }
            _ => {}
        }
    }
    names
}

fn capitalize(name: &str) -> String {
    let mut characters = name.chars();
    match characters.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), characters.as_str()),
        None => String::new(),
    }
}

fn exported_names(program: &syntax::Program) -> Vec<String> {
    let mut names = Vec::new();
    for statement in &program.body {
        if let Stmt::Export { names: exported } = statement {
            for name in exported {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
        }
    }
    names
}

fn attach_source(mut diagnostic: Diagnostic, source: &str) -> Diagnostic {
    if diagnostic.source.is_none() {
        diagnostic.source = Some(source.into());
    }
    for frame in &mut diagnostic.trace {
        if frame.source.is_none() {
            frame.source = Some(source.to_string());
        }
    }
    diagnostic
}
