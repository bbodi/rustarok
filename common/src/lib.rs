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

pub mod common;
pub mod components;
pub mod grf;
pub mod packets;
pub mod systems;
