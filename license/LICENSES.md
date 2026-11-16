# Licenses

Mithqal is licensed under the GNU GPL version 3.0 or later. The full text is in [GPL-3.0.txt](GPL-3.0.txt).

## Native libraries

| Dependency | Version | License | Repository | License texts |
| --- | --- | --- | --- | --- |
| GMP | 6.3.0 | GNU LGPL version 3.0 or later, or GNU GPL version 2.0 or later | https://github.com/gmp-mirror/gmp | [gmp](gmp/) |
| MPFR | 4.2.1 | GNU LGPL version 3.0 or later | https://github.com/BrianGladman/mpfr | [mpfr](mpfr/) |
| MPC | 1.3.1 | GNU LGPL version 3.0 or later | https://github.com/BrianGladman/mpc | [mpc](mpc/) |

## Rust dependencies

These versions come from `Cargo.lock`. Some Windows-only crates are listed because the lock file records them for cross-platform builds.

| Dependency | Version | License | Repository | License texts |
| --- | --- | --- | --- | --- |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | https://github.com/bitflags/bitflags | [LICENSE-APACHE](rust/bitflags/LICENSE-APACHE), [LICENSE-MIT](rust/bitflags/LICENSE-MIT) |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | https://github.com/rust-lang/cfg-if | [LICENSE-APACHE](rust/cfg-if/LICENSE-APACHE), [LICENSE-MIT](rust/cfg-if/LICENSE-MIT) |
| cfg_aliases | 0.2.2 | MIT | https://github.com/katharostech/cfg_aliases | [LICENSE](rust/cfg_aliases/LICENSE), [NOTICES.md](rust/cfg_aliases/NOTICES.md) |
| clipboard-win | 5.4.3 | BSL-1.0 | https://github.com/DoumanAsh/clipboard-win | [LICENSE](rust/clipboard-win/LICENSE) |
| endian-type | 0.2.0 | MIT | https://github.com/Lolirofle/endian-type.git | [LICENSE](rust/endian-type/LICENSE) |
| error-code | 3.4.0 | BSL-1.0 | https://github.com/DoumanAsh/error-code | [LICENSE](rust/error-code/LICENSE) |
| home | 0.5.12 | MIT OR Apache-2.0 | https://github.com/rust-lang/cargo | [LICENSE-APACHE](rust/home/LICENSE-APACHE), [LICENSE-MIT](rust/home/LICENSE-MIT) |
| libc | 0.2.191 | MIT OR Apache-2.0 | https://github.com/rust-lang/libc | [LICENSE-APACHE](rust/libc/LICENSE-APACHE), [LICENSE-MIT](rust/libc/LICENSE-MIT) |
| log | 0.4.35 | MIT OR Apache-2.0 | https://github.com/rust-lang/log | [LICENSE-APACHE](rust/log/LICENSE-APACHE), [LICENSE-MIT](rust/log/LICENSE-MIT) |
| memchr | 2.8.3 | Unlicense OR MIT | https://github.com/BurntSushi/memchr | [COPYING](rust/memchr/COPYING), [LICENSE-MIT](rust/memchr/LICENSE-MIT) |
| nibble_vec | 0.1.0 | MIT | https://github.com/michaelsproul/rust_nibble_vec | [LICENSE](rust/nibble_vec/LICENSE) |
| nix | 0.31.3 | MIT | https://github.com/nix-rust/nix | [LICENSE](rust/nix/LICENSE) |
| radix_trie | 0.3.0 | MIT | https://github.com/michaelsproul/rust_radix_trie | [LICENSE](rust/radix_trie/LICENSE) |
| rustyline | 19.0.0 | MIT | https://github.com/kkawakam/rustyline | [LICENSE](rust/rustyline/LICENSE) |
| smallvec | 1.16.5 | MIT OR Apache-2.0 | https://github.com/servo/rust-smallvec | [LICENSE-APACHE](rust/smallvec/LICENSE-APACHE), [LICENSE-MIT](rust/smallvec/LICENSE-MIT) |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-segmentation | [LICENSE-APACHE](rust/unicode-segmentation/LICENSE-APACHE), [LICENSE-MIT](rust/unicode-segmentation/LICENSE-MIT) |
| unicode-width | 0.2.3 | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-width | [LICENSE-APACHE](rust/unicode-width/LICENSE-APACHE), [LICENSE-MIT](rust/unicode-width/LICENSE-MIT) |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT | https://github.com/alacritty/vte | [LICENSE-APACHE](rust/utf8parse/LICENSE-APACHE), [LICENSE-MIT](rust/utf8parse/LICENSE-MIT) |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | [license-apache-2.0](rust/windows-link/license-apache-2.0), [license-mit](rust/windows-link/license-mit) |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | [license-apache-2.0](rust/windows-sys/license-apache-2.0), [license-mit](rust/windows-sys/license-mit) |

The Rust sources are fetched from crates.io. The Cargo registry keeps the complete package archives.
