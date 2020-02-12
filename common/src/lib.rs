//#![deny(
//    warnings,
//    anonymous_parameters,
//    unused_extern_crates,
//    unused_import_braces,
//    trivial_casts,
//    variant_size_differences,
//    trivial_numeric_casts,
//    unused_qualifications,
//    clippy::all
//)]

#[macro_use]
extern crate specs_derive;
extern crate hexplay;

use specs;

pub mod attack;
pub mod char_attr;
pub mod common;
pub mod components;
pub mod config;
pub mod console;
pub mod grf;
pub mod map;
pub mod packets;
pub mod systems;
