#![feature(box_patterns)]
#![feature(box_syntax)]
#![feature(decl_macro)]
#![allow(unused_imports)]
#![deny(unused_must_use)]
#![deny(unreachable_patterns)]
#![deny(unused_mut)]
#![deny(unused_variables)]
#![deny(unused_doc_comments)]

#[rustfmt::skip]
#[path = "../gen/mod.rs"]
mod gen;

pub mod common;
pub mod converter;
pub mod high;
pub mod legacy;
pub mod low;
pub mod middle;
pub mod polymorphic;
pub mod typed;
