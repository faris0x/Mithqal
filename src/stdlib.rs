//! The standard library, written in the Mithqal language.
//!
//! Each module is a surface language source bundled into the binary. A session
//! resolves the import graph, evaluates every module, and makes only the names
//! a module exports visible.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::collections::HashMap;

use crate::syntax::{Stmt, parse_program};

/// One bundled library module.
pub(crate) struct Module {
    /// Module name used in diagnostics.
    pub(crate) name: &'static str,
    /// Module source.
    pub(crate) source: &'static str,
}

/// Bundled library modules in declaration order.
pub(crate) const MODULES: &[Module] = &[
    Module {
        name: "lists",
        source: include_str!("../lib/lists.mith"),
    },
    Module {
        name: "logic",
        source: include_str!("../lib/logic.mith"),
    },
    Module {
        name: "sets",
        source: include_str!("../lib/sets.mith"),
    },
    Module {
        name: "relations",
        source: include_str!("../lib/relations.mith"),
    },
    Module {
        name: "structures",
        source: include_str!("../lib/structures.mith"),
    },
    Module {
        name: "foundations",
        source: include_str!("../lib/foundations.mith"),
    },
    Module {
        name: "statistics",
        source: include_str!("../lib/statistics.mith"),
    },
    Module {
        name: "combinatorics",
        source: include_str!("../lib/combinatorics.mith"),
    },
    Module {
        name: "numbertheory",
        source: include_str!("../lib/numbertheory.mith"),
    },
    Module {
        name: "probability",
        source: include_str!("../lib/probability.mith"),
    },
    Module {
        name: "decision",
        source: include_str!("../lib/decision.mith"),
    },
    Module {
        name: "games",
        source: include_str!("../lib/games.mith"),
    },
    Module {
        name: "graphs",
        source: include_str!("../lib/graphs.mith"),
    },
    Module {
        name: "information",
        source: include_str!("../lib/information.mith"),
    },
    Module {
        name: "calculus",
        source: include_str!("../lib/calculus.mith"),
    },
];

/// Resolve a selected module closure in dependency order.
pub(crate) fn select_order(modules: &[Module], names: &[&str]) -> Result<Vec<usize>, String> {
    let mut index = HashMap::new();
    for (position, module) in modules.iter().enumerate() {
        if index.insert(module.name, position).is_some() {
            return Err(format!("duplicate module {}", module.name));
        }
    }
    let mut imports = vec![Vec::new(); modules.len()];
    for (position, module) in modules.iter().enumerate() {
        let program = parse_program(module.source)
            .map_err(|error| format!("module {} is invalid: {error}", module.name))?;
        for statement in &program.body {
            if let Stmt::Import { module: dependency } = statement {
                let Some(&target) = index.get(dependency.as_str()) else {
                    return Err(format!(
                        "module {} imports unknown module {dependency}",
                        module.name
                    ));
                };
                if !imports[position].contains(&target) {
                    imports[position].push(target);
                }
            }
        }
    }
    let mut selected = vec![false; modules.len()];
    fn mark(node: usize, imports: &[Vec<usize>], selected: &mut [bool]) {
        if selected[node] {
            return;
        }
        selected[node] = true;
        for dependency in &imports[node] {
            mark(*dependency, imports, selected);
        }
    }
    for name in names {
        let Some(&position) = index.get(*name) else {
            return Err(format!("unknown module {name}"));
        };
        mark(position, &imports, &mut selected);
    }
    let order = resolve_order(modules)?;
    Ok(order
        .into_iter()
        .filter(|position| selected[*position])
        .collect())
}

/// Resolve module load order from import declarations.
pub(crate) fn resolve_order(modules: &[Module]) -> Result<Vec<usize>, String> {
    let mut programs = Vec::with_capacity(modules.len());
    for module in modules {
        let program = parse_program(module.source)
            .map_err(|error| format!("module {} is invalid: {error}", module.name))?;
        programs.push(program);
    }
    let mut index = HashMap::new();
    for (position, module) in modules.iter().enumerate() {
        if index.insert(module.name, position).is_some() {
            return Err(format!("duplicate module {}", module.name));
        }
    }
    let mut imports = vec![Vec::new(); modules.len()];
    for (position, program) in programs.iter().enumerate() {
        for statement in &program.body {
            if let Stmt::Import { module } = statement {
                let Some(&target) = index.get(module.as_str()) else {
                    return Err(format!(
                        "module {} imports unknown module {module}",
                        modules[position].name
                    ));
                };
                if !imports[position].contains(&target) {
                    imports[position].push(target);
                }
            }
        }
    }
    let mut state = vec![State::Unvisited; modules.len()];
    let mut order = Vec::with_capacity(modules.len());
    for node in 0..modules.len() {
        visit(node, &imports, &mut state, &mut order, modules)?;
    }
    Ok(order)
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum State {
    Unvisited,
    Visiting,
    Done,
}

fn visit(
    node: usize,
    imports: &[Vec<usize>],
    state: &mut [State],
    order: &mut Vec<usize>,
    modules: &[Module],
) -> Result<(), String> {
    match state[node] {
        State::Done => return Ok(()),
        State::Visiting => {
            return Err(format!("module import cycle at {}", modules[node].name));
        }
        State::Unvisited => {}
    }
    state[node] = State::Visiting;
    for &next in &imports[node] {
        visit(next, imports, state, order, modules)?;
    }
    state[node] = State::Done;
    order.push(node);
    Ok(())
}

/// Names exported by the standard library, paired with their module.
#[cfg(test)]
pub(crate) fn exports() -> Vec<(String, String)> {
    let mut exports = Vec::new();
    for module in MODULES {
        let program = parse_program(module.source).expect("standard library module parses");
        for statement in &program.body {
            if let Stmt::Export { names } = statement {
                for name in names {
                    exports.push((module.name.to_string(), name.clone()));
                }
            }
        }
    }
    exports
}

/// True when a name is implemented by the standard library.
#[cfg(test)]
pub(crate) fn is_library_function(name: &str) -> bool {
    exports().iter().any(|(_, exported)| exported == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::HashSet;

    #[test]
    fn exports_are_defined_and_unique() {
        let mut seen = HashSet::new();
        for module in MODULES {
            let program = parse_program(module.source).unwrap();
            let defined: HashSet<&str> = program
                .body
                .iter()
                .filter_map(|statement| match statement {
                    Stmt::Def { name, .. } => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            for statement in &program.body {
                if let Stmt::Export { names } = statement {
                    for name in names {
                        assert!(
                            defined.contains(name.as_str()),
                            "module {} exports undefined function {name}",
                            module.name
                        );
                        assert!(
                            seen.insert(name.clone()),
                            "function {name} is exported twice"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn selected_order_includes_dependencies_only() {
        let order = select_order(MODULES, &["combinatorics"]).unwrap();
        let names: Vec<&str> = order.iter().map(|index| MODULES[*index].name).collect();
        assert!(names.contains(&"combinatorics"));
        assert!(names.contains(&"lists"));
        assert!(names.contains(&"sets"));
        assert!(!names.contains(&"statistics"));
        assert!(!names.contains(&"graphs"));
    }

    #[test]
    fn imports_resolve_in_dependency_order() {
        let modules = [
            Module {
                name: "beta",
                source: "mod beta\nimport alpha\nend\n",
            },
            Module {
                name: "alpha",
                source: "mod alpha\nend\n",
            },
        ];
        let order = resolve_order(&modules).unwrap();
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn import_cycles_and_unknown_modules_are_rejected() {
        let modules = [
            Module {
                name: "alpha",
                source: "mod alpha\nimport beta\nend\n",
            },
            Module {
                name: "beta",
                source: "mod beta\nimport alpha\nend\n",
            },
        ];
        assert!(resolve_order(&modules).is_err());
        let modules = [Module {
            name: "alpha",
            source: "mod alpha\nimport missing\nend\n",
        }];
        assert!(resolve_order(&modules).is_err());
    }
}
