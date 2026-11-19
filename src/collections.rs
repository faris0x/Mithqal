//! Collection types that work with and without the standard library.

#[cfg(feature = "std")]
pub(crate) use std::collections::{HashMap, HashSet};

#[cfg(not(feature = "std"))]
pub(crate) use hashbrown::{HashMap, HashSet};

pub(crate) use alloc::collections::{BTreeMap, BTreeSet};
