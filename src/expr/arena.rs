//! The expression arena and symbol table.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::collections::HashMap;
use core::cmp::Ordering;

use crate::diag::{Diagnostic, ErrorCode, Message};
use crate::hash::FnvBuildHasher;
use crate::num::{Int, Rat, Real};

use super::node::{AssociationEntry, Attributes, ExprId, Node, SymbolId};
use super::order::canonical_cmp;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Key {
    Int {
        sign: i8,
        limbs: Box<[u8]>,
    },
    Rat {
        sign: i8,
        num: Box<[u8]>,
        den: Box<[u8]>,
    },
    Real {
        precision: u32,
        sign: i8,
        exponent: i64,
        limbs: Box<[u8]>,
        error: u64,
    },
    String(Box<str>),
    Bytes(Box<[u8]>),
    Symbol(SymbolId),
    Call {
        head: ExprId,
        args: Box<[ExprId]>,
    },
    Association(Box<[AssociationEntry]>),
}

const GLOBAL_CONTEXT: &str = "Global";

fn symbol_key(context: &str, name: &str) -> String {
    let mut key = String::with_capacity(context.len() + name.len() + 1);
    key.push_str(context);
    key.push(';');
    key.push_str(name);
    key
}

struct SymbolInfo {
    name: String,
    context: String,
    attrs: Attributes,
    node: Option<ExprId>,
}

/// An arena of hash consed expression nodes and a symbol table.
pub struct Arena {
    nodes: Vec<Option<Node>>,
    generations: Vec<u32>,
    intern: HashMap<Key, ExprId, FnvBuildHasher>,
    free: Vec<usize>,
    live: usize,
    symbols: Vec<SymbolInfo>,
    symbol_names: HashMap<String, SymbolId, FnvBuildHasher>,
    builtin_heads: HashMap<&'static str, ExprId, FnvBuildHasher>,
    heads: Option<crate::kernel::Heads>,
    current_context: String,
    context_stack: Vec<String>,
    context_path: Vec<String>,
    contexts: Vec<String>,
    context_version: u64,
    messages: Vec<Message>,
    message_suppression: usize,
    disabled_messages: Vec<String>,
}

impl Arena {
    /// Create an empty arena.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create an empty arena with initial node capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Arena {
            nodes: Vec::with_capacity(capacity),
            generations: Vec::with_capacity(capacity),
            intern: HashMap::default(),
            free: Vec::new(),
            live: 0,
            symbols: Vec::new(),
            symbol_names: HashMap::default(),
            builtin_heads: HashMap::default(),
            heads: None,
            current_context: GLOBAL_CONTEXT.to_string(),
            context_stack: Vec::new(),
            context_path: vec![GLOBAL_CONTEXT.to_string()],
            contexts: vec![GLOBAL_CONTEXT.to_string()],
            context_version: 0,
            messages: Vec::new(),
            message_suppression: 0,
            disabled_messages: Vec::new(),
        }
    }

    /// Number of live interned expression nodes.
    pub fn len(&self) -> usize {
        self.live
    }

    /// True when the arena has no live expression nodes.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Read a live node by handle.
    pub(crate) fn node(&self, id: ExprId) -> &Node {
        self.try_node(id).expect("expression handle is not live")
    }

    /// Read a live node by handle, or return `None` for a stale handle.
    pub fn try_node(&self, id: ExprId) -> Option<&Node> {
        let index = id.index();
        if self.generations.get(index).copied()? != id.generation() {
            return None;
        }
        self.nodes.get(index)?.as_ref()
    }

    /// Intern an integer.
    pub fn intern_int(&mut self, value: Int) -> ExprId {
        self.intern_node(Node::Int(value))
    }

    /// Intern a rational. A denominator of one becomes an integer node.
    pub fn intern_rat(&mut self, value: Rat) -> ExprId {
        if value.den().is_one() {
            self.intern_int(value.num())
        } else {
            self.intern_node(Node::Rat(value))
        }
    }

    /// Intern an approximate real.
    pub fn intern_real(&mut self, value: Real) -> ExprId {
        self.intern_node(Node::Real(value))
    }

    /// Intern a string value.
    pub fn intern_string(&mut self, value: &str) -> ExprId {
        self.intern_node(Node::String(value.into()))
    }

    /// Intern an immutable byte array.
    pub fn intern_bytes(&mut self, value: &[u8]) -> ExprId {
        self.intern_node(Node::Bytes(value.into()))
    }

    /// Intern an association.
    ///
    /// Keys are sorted into canonical order. Duplicate keys are rejected.
    pub fn intern_association(
        &mut self,
        mut entries: Vec<AssociationEntry>,
    ) -> Result<ExprId, Diagnostic> {
        entries.sort_by(|left, right| {
            self.canonical_cmp(left.key, right.key)
                .then_with(|| self.canonical_cmp(left.value, right.value))
        });
        for pair in entries.windows(2) {
            if pair[0].key == pair[1].key {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "an association cannot contain duplicate keys",
                ));
            }
        }
        Ok(self.intern_node(Node::Association(entries.into_boxed_slice())))
    }

    /// Intern a symbol name in the global context and set its attributes.
    pub fn intern_symbol(&mut self, name: &str, attrs: Attributes) -> SymbolId {
        self.intern_symbol_in_context(GLOBAL_CONTEXT, name, attrs)
    }

    /// Intern a symbol in a specific context and set its attributes.
    pub fn intern_symbol_in_context(
        &mut self,
        context: &str,
        name: &str,
        attrs: Attributes,
    ) -> SymbolId {
        let key = symbol_key(context, name);
        if let Some(&id) = self.symbol_names.get(&key) {
            self.symbols[id as usize].attrs = attrs;
            return id;
        }
        let id = self.symbols.len() as SymbolId;
        self.symbols.push(SymbolInfo {
            name: name.to_string(),
            context: context.to_string(),
            attrs,
            node: None,
        });
        self.symbol_names.insert(key, id);
        if !self.contexts.iter().any(|known| known == context) {
            self.contexts.push(context.to_string());
        }
        id
    }

    /// Intern a symbol in the current context without changing attributes.
    pub fn symbol_or_intern(&mut self, name: &str) -> SymbolId {
        let context = self.current_context.clone();
        if let Some(&id) = self.symbol_names.get(&symbol_key(&context, name)) {
            return id;
        }
        self.intern_symbol_in_context(&context, name, Attributes::new())
    }

    /// Resolve a short name through the current context and context path.
    ///
    /// The current context is searched first, followed by `$ContextPath`.
    /// A name that is not found is created in the current context.
    pub fn resolve_symbol(&mut self, name: &str) -> SymbolId {
        let mut contexts = Vec::with_capacity(self.context_path.len() + 1);
        contexts.push(self.current_context.clone());
        for context in &self.context_path {
            if !contexts.iter().any(|known| known == context) {
                contexts.push(context.clone());
            }
        }
        for context in &contexts {
            if let Some(&id) = self.symbol_names.get(&symbol_key(context, name)) {
                return id;
            }
        }
        let context = self.current_context.clone();
        self.intern_symbol_in_context(&context, name, Attributes::new())
    }

    /// Intern a symbol in an explicit context.
    pub fn intern_qualified_symbol(
        &mut self,
        context: &str,
        name: &str,
        attrs: Attributes,
    ) -> SymbolId {
        self.intern_symbol_in_context(context, name, attrs)
    }

    /// The current context.
    pub fn current_context(&self) -> &str {
        &self.current_context
    }

    /// The ordered context path searched for short names.
    pub fn context_path(&self) -> &[String] {
        &self.context_path
    }

    /// Every context that has been used in this arena.
    pub fn contexts(&self) -> Vec<String> {
        let mut contexts = self.contexts.clone();
        contexts.sort();
        contexts.dedup();
        contexts
    }

    /// Emit a non-fatal message.
    pub(crate) fn push_message(&mut self, message: Message) {
        if self.message_suppression == 0
            && !self
                .disabled_messages
                .iter()
                .any(|name| name == &message.name)
        {
            self.messages.push(message);
        }
    }

    /// Number of emitted messages.
    pub(crate) fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Move emitted messages out of the arena.
    pub(crate) fn take_messages(&mut self) -> Vec<Message> {
        core::mem::take(&mut self.messages)
    }

    /// Suppress messages until the matching unsuppress call.
    pub(crate) fn suppress_messages(&mut self) {
        self.message_suppression += 1;
    }

    /// End one message suppression.
    pub(crate) fn unsuppress_messages(&mut self) {
        self.message_suppression = self.message_suppression.saturating_sub(1);
    }

    /// Disable a named message.
    pub(crate) fn disable_message(&mut self, name: &str) {
        if !self.disabled_messages.iter().any(|known| known == name) {
            self.disabled_messages.push(name.to_string());
        }
    }

    /// Enable a named message.
    pub(crate) fn enable_message(&mut self, name: &str) {
        self.disabled_messages.retain(|known| known != name);
    }

    /// A monotonically increasing context generation.
    pub fn context_version(&self) -> u64 {
        self.context_version
    }

    /// Enter a context and return its name.
    pub fn begin_context(&mut self, context: &str) -> String {
        self.context_stack.push(self.current_context.clone());
        self.current_context = context.to_string();
        if !self.contexts.iter().any(|known| known == context) {
            self.contexts.push(context.to_string());
        }
        self.context_version = self.context_version.wrapping_add(1);
        self.current_context.clone()
    }

    /// Leave the current context and return its name.
    pub fn end_context(&mut self) -> Option<String> {
        let previous = self.context_stack.pop()?;
        let ended = core::mem::replace(&mut self.current_context, previous);
        self.context_version = self.context_version.wrapping_add(1);
        Some(ended)
    }

    /// Create a deterministic fresh symbol derived from an existing name.
    ///
    /// Names use the base name followed by a counter, so they remain valid
    /// source identifiers and printed expressions round trip.
    pub(crate) fn fresh_symbol(&mut self, base: SymbolId) -> SymbolId {
        let base_name = self.symbols[base as usize].name.clone();
        let context = self.symbols[base as usize].context.clone();
        let mut counter = 0u64;
        loop {
            counter = counter.wrapping_add(1);
            let name = format!("{base_name}{counter}");
            if !self.symbol_names.contains_key(&symbol_key(&context, &name)) {
                return self.intern_symbol_in_context(&context, &name, Attributes::new());
            }
        }
    }

    /// The symbol expression for a symbol table entry.
    pub fn symbol_expr(&mut self, id: SymbolId) -> ExprId {
        if let Some(node) = self.symbols[id as usize].node {
            return node;
        }
        let node = self.intern_node(Node::Symbol(id));
        self.symbols[id as usize].node = Some(node);
        node
    }

    /// The symbol short name.
    pub fn symbol_name(&self, id: SymbolId) -> &str {
        &self.symbols[id as usize].name
    }

    /// The symbol context.
    pub fn symbol_context(&self, id: SymbolId) -> &str {
        &self.symbols[id as usize].context
    }

    /// The symbol name including its context when it is not global.
    pub fn symbol_full_name(&self, id: SymbolId) -> String {
        let info = &self.symbols[id as usize];
        if info.context == GLOBAL_CONTEXT {
            info.name.clone()
        } else {
            format!("{};{}", info.context, info.name)
        }
    }

    /// The symbol name for human display in the current context.
    pub fn symbol_display_name(&self, id: SymbolId) -> String {
        let info = &self.symbols[id as usize];
        if info.context == GLOBAL_CONTEXT || info.context == self.current_context {
            info.name.clone()
        } else {
            format!("{};{}", info.context, info.name)
        }
    }

    /// The symbol attributes.
    pub fn symbol_attributes(&self, id: SymbolId) -> Attributes {
        self.symbols[id as usize].attrs
    }

    /// Replace the symbol attributes.
    pub fn set_symbol_attributes(&mut self, id: SymbolId, attrs: Attributes) {
        self.symbols[id as usize].attrs = attrs;
    }

    /// The symbol table entry for an expression, when it is a symbol.
    pub fn symbol_id(&self, id: ExprId) -> Option<SymbolId> {
        match self.node(id) {
            Node::Symbol(symbol) => Some(*symbol),
            _ => None,
        }
    }

    /// The attributes of a call head. Non symbol heads have no attributes.
    pub fn head_attributes(&self, head: ExprId) -> Attributes {
        match self.node(head) {
            Node::Symbol(symbol) => self.symbol_attributes(*symbol),
            _ => Attributes::new(),
        }
    }

    /// Resolve and cache a built in head expression.
    pub(crate) fn builtin_head(&mut self, name: &'static str) -> ExprId {
        if let Some(head) = self.builtin_heads.get(name) {
            return *head;
        }
        let attrs = crate::builtins::attributes(name).unwrap_or_default();
        let symbol = self.intern_symbol(name, attrs);
        let head = self.symbol_expr(symbol);
        self.builtin_heads.insert(name, head);
        head
    }

    /// The cached core head handles.
    pub(crate) fn heads(&mut self) -> crate::kernel::Heads {
        if let Some(heads) = self.heads {
            return heads;
        }
        let heads = crate::kernel::Heads::new(self);
        self.heads = Some(heads);
        heads
    }

    /// Intern a call, applying flatten and canonical sorting for the head.
    pub fn intern_call(&mut self, head: ExprId, args: &[ExprId]) -> ExprId {
        let attrs = self.head_attributes(head);
        let mut flat: Vec<ExprId> = Vec::with_capacity(args.len());
        if attrs.has(Attributes::FLAT) {
            for &arg in args {
                match self.node(arg) {
                    Node::Call {
                        head: child_head,
                        args: child_args,
                    } if *child_head == head => flat.extend_from_slice(child_args),
                    _ => flat.push(arg),
                }
            }
        } else {
            flat.extend_from_slice(args);
        }
        if attrs.has(Attributes::ORDERLESS) {
            flat.sort_by(|left, right| self.canonical_cmp(*left, *right));
        }
        self.intern_node(Node::Call {
            head,
            args: flat.into_boxed_slice(),
        })
    }

    /// Intern a call without flatten or sorting.
    pub fn intern_call_raw(&mut self, head: ExprId, args: &[ExprId]) -> ExprId {
        self.intern_node(Node::Call {
            head,
            args: args.into(),
        })
    }

    /// Compare two expressions in canonical order.
    pub fn canonical_cmp(&self, left: ExprId, right: ExprId) -> Ordering {
        canonical_cmp(self, left, right)
    }

    /// Reclaim every node that is not reachable from the roots or the symbol
    /// table. Surviving handles remain valid because collection is non moving.
    pub fn collect(&mut self, roots: &[ExprId]) {
        let mut marked = vec![false; self.nodes.len()];
        let mut stack = Vec::new();
        stack.extend_from_slice(roots);
        for symbol in &self.symbols {
            if let Some(node) = symbol.node {
                stack.push(node);
            }
        }

        while let Some(id) = stack.pop() {
            let index = id.index();
            if index >= marked.len() || marked[index] {
                continue;
            }
            if self.generations.get(index).copied() != Some(id.generation()) {
                continue;
            }
            let Some(node) = self.nodes[index].as_ref() else {
                continue;
            };
            marked[index] = true;
            match node {
                Node::Call { head, args } => {
                    stack.push(*head);
                    stack.extend(args.iter().copied());
                }
                Node::Association(entries) => {
                    for entry in entries {
                        stack.push(entry.key);
                        stack.push(entry.value);
                    }
                }
                _ => {}
            }
        }

        let mut intern = HashMap::default();
        let nodes = &mut self.nodes;
        let generations = &mut self.generations;
        let free = &mut self.free;
        let mut live = self.live;
        for (index, slot) in nodes.iter_mut().enumerate() {
            match slot {
                Some(node) if marked[index] => {
                    intern.insert(
                        key_for(node),
                        ExprId::from_parts(index as u32, generations[index]),
                    );
                }
                Some(_) => {
                    *slot = None;
                    generations[index] = generations[index].wrapping_add(1);
                    if generations[index] == 0 {
                        generations[index] = 1;
                    }
                    free.push(index);
                    live -= 1;
                }
                None => {}
            }
        }
        self.live = live;
        self.intern = intern;
    }

    fn intern_node(&mut self, node: Node) -> ExprId {
        if let Node::Rat(value) = &node
            && value.den().is_one()
        {
            return self.intern_int(value.num());
        }
        let key = key_for(&node);
        if let Some(&id) = self.intern.get(&key) {
            return id;
        }
        let id = if let Some(index) = self.free.pop() {
            self.nodes[index] = Some(node);
            ExprId::from_parts(index as u32, self.generations[index])
        } else {
            let index = self.nodes.len();
            self.nodes.push(Some(node));
            self.generations.push(0);
            ExprId::from_parts(index as u32, 0)
        };
        self.live += 1;
        self.intern.insert(key, id);
        id
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

fn key_for(node: &Node) -> Key {
    match node {
        Node::Int(value) => Key::Int {
            sign: value.sign(),
            limbs: value.magnitude().into(),
        },
        Node::Rat(value) => Key::Rat {
            sign: value.num_sign(),
            num: value.num_limbs().into(),
            den: value.den_limbs().into(),
        },
        Node::Real(value) => {
            let (precision, sign, exponent, limbs, error) = value.key_parts();
            Key::Real {
                precision,
                sign,
                exponent,
                limbs,
                error,
            }
        }
        Node::String(value) => Key::String(value.clone()),
        Node::Bytes(value) => Key::Bytes(value.clone()),
        Node::Symbol(symbol) => Key::Symbol(*symbol),
        Node::Call { head, args } => Key::Call {
            head: *head,
            args: args.clone(),
        },
        Node::Association(entries) => Key::Association(entries.clone()),
    }
}
