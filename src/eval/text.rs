//! String, byte array, and association functions.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::expr::AssociationEntry;

use super::*;

impl Evaluator<'_> {
    pub(super) fn apply_text(&mut self, function: TextHead, args: &[ExprId]) -> Option<ExprId> {
        match function {
            TextHead::StringLength => {
                let text = self.string_value(args[0])?;
                Some(
                    self.arena
                        .intern_int(Int::from_i64(text.chars().count() as i64)),
                )
            }
            TextHead::StringJoin => {
                let mut result = String::new();
                for argument in args {
                    result.push_str(self.string_value(*argument)?);
                }
                Some(self.arena.intern_string(&result))
            }
            TextHead::StringSplit => {
                let text = self.string_value(args[0])?.to_string();
                let parts: Vec<String> = if args.len() == 1 {
                    text.split_whitespace().map(ToString::to_string).collect()
                } else {
                    let separator = self.string_value(args[1])?.to_string();
                    text.split(separator.as_str())
                        .map(ToString::to_string)
                        .collect()
                };
                let mut items = Vec::with_capacity(parts.len());
                for part in parts {
                    items.push(self.arena.intern_string(&part));
                }
                Some(self.list(&items))
            }
            TextHead::StringReplace => {
                let text = self.string_value(args[0])?.to_string();
                let from = self.string_value(args[1])?.to_string();
                let to = self.string_value(args[2])?.to_string();
                Some(self.arena.intern_string(&text.replace(&from, &to)))
            }
            TextHead::StringTake => {
                let characters: Vec<char> = self.string_value(args[0])?.chars().collect();
                let count = self.integer_value(args[1])?;
                let slice = take_chars(&characters, count)?;
                Some(
                    self.arena
                        .intern_string(&slice.into_iter().collect::<String>()),
                )
            }
            TextHead::StringDrop => {
                let characters: Vec<char> = self.string_value(args[0])?.chars().collect();
                let count = self.integer_value(args[1])?;
                let slice = drop_chars(&characters, count)?;
                Some(
                    self.arena
                        .intern_string(&slice.into_iter().collect::<String>()),
                )
            }
            TextHead::StringContains => {
                let text = self.string_value(args[0])?;
                let needle = self.string_value(args[1])?;
                Some(self.boolean(text.contains(needle)))
            }
            TextHead::StringReverse => {
                let text: String = self.string_value(args[0])?.chars().rev().collect();
                Some(self.arena.intern_string(&text))
            }
            TextHead::Characters => {
                let characters = self.string_value(args[0])?.to_string();
                let mut items = Vec::new();
                for character in characters.chars() {
                    items.push(self.arena.intern_string(&character.to_string()));
                }
                Some(self.list(&items))
            }
            TextHead::CharacterCode => {
                let character = self.string_value(args[0])?.chars().next()?;
                Some(
                    self.arena
                        .intern_int(Int::from_i64(i64::from(character as u32))),
                )
            }
            TextHead::FromCharacterCode => {
                let code = self.integer_value(args[0])?;
                let code = u32::try_from(code).ok()?;
                let character = char::from_u32(code)?;
                Some(self.arena.intern_string(&character.to_string()))
            }
            TextHead::ToString => {
                let text = crate::print::infix(self.arena, args[0]);
                Some(self.arena.intern_string(&text))
            }
            TextHead::ByteArrayLength => {
                let bytes = self.bytes_value(args[0])?;
                Some(self.arena.intern_int(Int::from_i64(bytes.len() as i64)))
            }
            TextHead::ByteArrayToList => {
                let bytes = self.bytes_value(args[0])?.to_vec();
                let mut items = Vec::with_capacity(bytes.len());
                for byte in bytes {
                    items.push(self.arena.intern_int(Int::from_i64(i64::from(byte))));
                }
                Some(self.list(&items))
            }
            TextHead::ByteArrayFromList => {
                let items = self.list_items(args[0])?.to_vec();
                let mut bytes = Vec::with_capacity(items.len());
                for item in items {
                    let byte = self.integer_value(item)?;
                    bytes.push(u8::try_from(byte).ok()?);
                }
                Some(self.arena.intern_bytes(&bytes))
            }
            TextHead::ByteArrayToString => {
                let bytes = self.bytes_value(args[0])?.to_vec();
                let text = String::from_utf8(bytes).ok()?;
                Some(self.arena.intern_string(&text))
            }
            TextHead::StringToByteArray => {
                let bytes = self.string_value(args[0])?.as_bytes().to_vec();
                Some(self.arena.intern_bytes(&bytes))
            }
            TextHead::AssociationLength => {
                let entries = self.association_entries(args[0])?;
                Some(self.arena.intern_int(Int::from_i64(entries.len() as i64)))
            }
            TextHead::AssociationKeys => {
                let entries = self.association_entries(args[0])?.to_vec();
                let keys: Vec<ExprId> = entries.iter().map(|entry| entry.key).collect();
                Some(self.list(&keys))
            }
            TextHead::AssociationValues => {
                let entries = self.association_entries(args[0])?.to_vec();
                let values: Vec<ExprId> = entries.iter().map(|entry| entry.value).collect();
                Some(self.list(&values))
            }
            TextHead::Lookup => {
                let entries = self.association_entries(args[0])?.to_vec();
                let key = args[1];
                let default = args.get(2).copied().unwrap_or(self.heads.undefined);
                Some(
                    entries
                        .iter()
                        .find(|entry| entry.key == key)
                        .map_or(default, |entry| entry.value),
                )
            }
            TextHead::KeyExistsQ => {
                let entries = self.association_entries(args[0])?;
                Some(self.boolean(entries.iter().any(|entry| entry.key == args[1])))
            }
            TextHead::AssociationMerge => {
                let mut entries: Vec<AssociationEntry> = Vec::new();
                for argument in args {
                    for entry in self.association_entries(*argument)?.to_vec() {
                        if let Some(existing) = entries
                            .iter_mut()
                            .find(|existing| existing.key == entry.key)
                        {
                            existing.value = entry.value;
                        } else {
                            entries.push(entry);
                        }
                    }
                }
                self.arena.intern_association(entries).ok()
            }
            TextHead::AssociationKeyTake => {
                let entries = self.association_entries(args[0])?.to_vec();
                let keys = self.list_items(args[1])?.to_vec();
                let selected: Vec<AssociationEntry> = entries
                    .into_iter()
                    .filter(|entry| keys.contains(&entry.key))
                    .collect();
                self.arena.intern_association(selected).ok()
            }
            TextHead::AssociationKeyDrop => {
                let entries = self.association_entries(args[0])?.to_vec();
                let keys = self.list_items(args[1])?.to_vec();
                let selected: Vec<AssociationEntry> = entries
                    .into_iter()
                    .filter(|entry| !keys.contains(&entry.key))
                    .collect();
                self.arena.intern_association(selected).ok()
            }
            TextHead::AssociationFromPairs => {
                let pairs = self.list_items(args[0])?.to_vec();
                let mut entries = Vec::with_capacity(pairs.len());
                for pair in pairs {
                    let values = self.list_items(pair)?.to_vec();
                    if values.len() != 2 {
                        return None;
                    }
                    entries.push(AssociationEntry {
                        key: values[0],
                        value: values[1],
                    });
                }
                self.arena.intern_association(entries).ok()
            }
            TextHead::AssociationToPairs => {
                let entries = self.association_entries(args[0])?.to_vec();
                let mut pairs = Vec::with_capacity(entries.len());
                for entry in entries {
                    let pair = [entry.key, entry.value];
                    pairs.push(self.list(&pair));
                }
                Some(self.list(&pairs))
            }
            TextHead::AssociationMap => {
                let entries = self.association_entries(args[0])?.to_vec();
                let mut mapped = Vec::with_capacity(entries.len());
                for entry in entries {
                    let value = self
                        .call_function(args[1], &[entry.key, entry.value])
                        .ok()?;
                    mapped.push(AssociationEntry {
                        key: entry.key,
                        value,
                    });
                }
                self.arena.intern_association(mapped).ok()
            }
        }
    }

    fn string_value(&self, expression: ExprId) -> Option<&str> {
        match self.arena.node(expression) {
            Node::String(value) => Some(value),
            _ => None,
        }
    }

    fn bytes_value(&self, expression: ExprId) -> Option<&[u8]> {
        match self.arena.node(expression) {
            Node::Bytes(value) => Some(value),
            _ => None,
        }
    }

    fn association_entries(&self, expression: ExprId) -> Option<&[AssociationEntry]> {
        match self.arena.node(expression) {
            Node::Association(entries) => Some(entries),
            _ => None,
        }
    }

    fn integer_value(&self, expression: ExprId) -> Option<i64> {
        match self.arena.node(expression) {
            Node::Int(value) => value.to_i64(),
            Node::Rat(value) if value.den().is_one() => value.num().to_i64(),
            _ => None,
        }
    }

    pub(super) fn boolean(&mut self, value: bool) -> ExprId {
        if value {
            self.heads.true_value
        } else {
            self.heads.false_value
        }
    }

    pub(super) fn list(&mut self, items: &[ExprId]) -> ExprId {
        self.arena.intern_call(self.heads.list, items)
    }
}

fn take_chars(characters: &[char], count: i64) -> Option<Vec<char>> {
    if count >= 0 {
        let count = count as usize;
        Some(characters[..count.min(characters.len())].to_vec())
    } else {
        let count = count.unsigned_abs() as usize;
        Some(characters[characters.len().saturating_sub(count)..].to_vec())
    }
}

fn drop_chars(characters: &[char], count: i64) -> Option<Vec<char>> {
    if count >= 0 {
        let count = count as usize;
        Some(characters[count.min(characters.len())..].to_vec())
    } else {
        let count = count.unsigned_abs() as usize;
        let end = characters.len().saturating_sub(count);
        Some(characters[..end].to_vec())
    }
}
