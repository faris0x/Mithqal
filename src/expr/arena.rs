//! The expression arena and symbol table.

use core::cmp::Ordering;
use std::collections::HashMap;

use crate::hash::FnvBuildHasher;
use crate::num::{Int, Rat};

use super::node::{Attributes, ExprId, Node, SymbolId};
use super::order::canonical_cmp;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Key {
    Int {
        sign: i8,
        limbs: Box<[u64]>,
    },
    Rat {
        sign: i8,
        num: Box<[u64]>,
        den: Box<[u64]>,
    },
    Symbol(SymbolId),
    Call {
        head: ExprId,
        args: Box<[ExprId]>,
    },
}

struct SymbolInfo {
    name: String,
    attrs: Attributes,
    node: Option<ExprId>,
}

/// An arena of hash consed expression nodes and a symbol table.
pub struct Arena {
    nodes: Vec<Option<Node>>,
    intern: HashMap<Key, ExprId, FnvBuildHasher>,
    free: Vec<ExprId>,
    live: usize,
    symbols: Vec<SymbolInfo>,
    symbol_names: HashMap<String, SymbolId, FnvBuildHasher>,
}

impl Arena {
    /// Create an empty arena.
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new(),
            intern: HashMap::default(),
            free: Vec::new(),
            live: 0,
            symbols: Vec::new(),
            symbol_names: HashMap::default(),
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

    /// Read a node by handle.
    pub fn node(&self, id: ExprId) -> &Node {
        self.nodes[id as usize]
            .as_ref()
            .expect("expression handle is not live")
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

    /// Intern a symbol name and set its attributes.
    pub fn intern_symbol(&mut self, name: &str, attrs: Attributes) -> SymbolId {
        if let Some(&id) = self.symbol_names.get(name) {
            self.symbols[id as usize].attrs = attrs;
            return id;
        }
        let id = self.symbols.len() as SymbolId;
        self.symbols.push(SymbolInfo {
            name: name.to_string(),
            attrs,
            node: None,
        });
        self.symbol_names.insert(name.to_string(), id);
        id
    }

    /// Return an existing symbol or create it without changing attributes.
    pub fn symbol_or_intern(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.symbol_names.get(name) {
            return id;
        }
        let id = self.symbols.len() as SymbolId;
        self.symbols.push(SymbolInfo {
            name: name.to_string(),
            attrs: Attributes::new(),
            node: None,
        });
        self.symbol_names.insert(name.to_string(), id);
        id
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

    /// The symbol name.
    pub fn symbol_name(&self, id: SymbolId) -> &str {
        &self.symbols[id as usize].name
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

    /// Intern a call, applying flatten and canonical sorting for the head.
    pub fn intern_call(&mut self, head: ExprId, args: &[ExprId]) -> ExprId {
        let attrs = self.head_attributes(head);
        let mut flat: Vec<ExprId> = Vec::with_capacity(args.len());
        for &arg in args {
            if attrs.has(Attributes::FLAT) {
                if let Node::Call {
                    head: child_head,
                    args: child_args,
                } = self.node(arg)
                {
                    if *child_head == head {
                        flat.extend_from_slice(child_args);
                        continue;
                    }
                }
            }
            flat.push(arg);
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
            let index = id as usize;
            if index >= marked.len() || marked[index] {
                continue;
            }
            let Some(node) = self.nodes[index].as_ref() else {
                continue;
            };
            marked[index] = true;
            if let Node::Call { head, args } = node {
                stack.push(*head);
                stack.extend(args.iter().copied());
            }
        }

        let mut intern = HashMap::default();
        for (index, slot) in self.nodes.iter_mut().enumerate() {
            match slot {
                Some(node) if marked[index] => {
                    intern.insert(key_for(node), index as ExprId);
                }
                Some(_) => {
                    *slot = None;
                    self.free.push(index as ExprId);
                    self.live -= 1;
                }
                None => {}
            }
        }
        self.intern = intern;
    }

    fn intern_node(&mut self, node: Node) -> ExprId {
        if let Node::Rat(value) = &node {
            if value.den().is_one() {
                return self.intern_int(value.num());
            }
        }
        let key = key_for(&node);
        if let Some(&id) = self.intern.get(&key) {
            return id;
        }
        let id = if let Some(id) = self.free.pop() {
            self.nodes[id as usize] = Some(node);
            id
        } else {
            let id = self.nodes.len() as ExprId;
            self.nodes.push(Some(node));
            id
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
        Node::Symbol(symbol) => Key::Symbol(*symbol),
        Node::Call { head, args } => Key::Call {
            head: *head,
            args: args.clone(),
        },
    }
}
