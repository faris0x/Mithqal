use mithqal::Int;
use mithqal::expr::{Arena, AssociationEntry, Node};
use mithqal::print::{canonical, infix};

#[test]
fn strings_print_and_round_trip_through_the_printer() {
    let mut arena = Arena::new();
    let value = arena.intern_string("a\"b\\c\n\t\0");
    assert_eq!(canonical(&arena, value), "\"a\\\"b\\\\c\\n\\t\\0\"");
    assert_eq!(infix(&arena, value), "\"a\\\"b\\\\c\\n\\t\\0\"");
    assert!(
        matches!(arena.try_node(value), Some(Node::String(text)) if &**text == "a\"b\\c\n\t\0")
    );
}

#[test]
fn byte_arrays_print_as_byte_array_calls() {
    let mut arena = Arena::new();
    let value = arena.intern_bytes(&[0, 1, 127, 255]);
    assert_eq!(canonical(&arena, value), "byteArray(0,1,127,255)");
    assert_eq!(infix(&arena, value), "byteArray(0,1,127,255)");
}

#[test]
fn associations_sort_keys_and_reject_duplicates() {
    let mut arena = Arena::new();
    let a = arena.intern_symbol("a", Default::default());
    let b = arena.intern_symbol("b", Default::default());
    let a = arena.symbol_expr(a);
    let b = arena.symbol_expr(b);
    let one = arena.intern_int(Int::from_i64(1));
    let two = arena.intern_int(Int::from_i64(2));
    let value = arena
        .intern_association(vec![
            AssociationEntry { key: b, value: two },
            AssociationEntry { key: a, value: one },
        ])
        .unwrap();
    assert_eq!(canonical(&arena, value), "<|a->1,b->2|>");
    assert_eq!(infix(&arena, value), "<|a->1,b->2|>");
    assert!(
        arena
            .intern_association(vec![
                AssociationEntry { key: a, value: one },
                AssociationEntry { key: a, value: two },
            ])
            .is_err()
    );
}

#[test]
fn canonical_order_includes_the_new_atoms() {
    let mut arena = Arena::new();
    let string = arena.intern_string("x");
    let bytes = arena.intern_bytes(b"x");
    let symbol = arena.symbol_or_intern("x");
    let symbol = arena.symbol_expr(symbol);
    let association = arena.intern_association(Vec::new()).unwrap();
    assert!(arena.canonical_cmp(string, bytes) == std::cmp::Ordering::Less);
    assert!(arena.canonical_cmp(bytes, symbol) == std::cmp::Ordering::Less);
    assert!(arena.canonical_cmp(symbol, association) == std::cmp::Ordering::Less);
}

#[test]
fn collection_keeps_rooted_association_entries() {
    let mut arena = Arena::new();
    let key = arena.intern_string("key");
    let value = arena.intern_string("value");
    let association = arena
        .intern_association(vec![AssociationEntry { key, value }])
        .unwrap();
    arena.collect(&[association]);
    assert!(arena.try_node(association).is_some());
    assert!(arena.try_node(key).is_some());
    assert!(arena.try_node(value).is_some());
    arena.collect(&[]);
    assert!(arena.try_node(association).is_none());
    assert!(arena.try_node(key).is_none());
    assert!(arena.try_node(value).is_none());
}
