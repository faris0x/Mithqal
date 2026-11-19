//! A small deterministic hasher for hot maps.

#[allow(unused_imports)]
use crate::prelude::*;

use core::hash::{BuildHasherDefault, Hasher};

/// FNV-1a 64 bit hasher.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FnvHasher(u64);

impl Default for FnvHasher {
    fn default() -> Self {
        FnvHasher(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= byte as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

/// Build hasher type for the hot maps.
pub(crate) type FnvBuildHasher = BuildHasherDefault<FnvHasher>;
