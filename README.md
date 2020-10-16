# Scones

[![Scones on Crates.IO](https://img.shields.io/crates/v/scones)](https://crates.io/crates/scones)
[![Documentation](https://img.shields.io/badge/documentation-link-success)](https://docs.rs/scones)
[![Examples](https://img.shields.io/badge/examples-link-success)](https://docs.rs/scones_examples)

A crate for quick and powerful constructor/builder generation in Rust. Dual
licensed under `MIT OR Apache-2.0`. Example:

```rust
use scones::{make_builder, make_constructor};

#[make_builder]
#[make_constructor]
struct Basic {
    int: i32,
    string: String,
}
let instance = Basic::new(int, string);
let instance = BasicBuilder::new().string("str".to_owned()).int(12345).build();
// Triggers a compile-time error because we have not specified all fields yet:
// let instance = BasicBuilder::new().build();

#[make_constructor]
#[make_constructor(pub new_identical(shared: i32))]
pub struct MultipleConstructors {
    #[value(shared for new_identical)]
    pub a: i32,
    #[value(shared for new_identical)]
    pub b: i32,
    #[value(shared for new_identical)]
    pub c: i32,

    #[value(true)]
    #[value(false for new)]
    pub identical: bool,
}
let instance = MultipleConstructors::new(1, 2, 3);
let instance = MultipleConstructors::new_identical(123);

#[make_constructor]
#[make_constructor(pub default_number)]
#[make_builder((field_1?))]
pub struct TupleStruct(
    #[value(30 for default_number)] i32,
    #[value("Unnamed".to_owned() for TupleStructBuilder)] String,
);
let instance = TupleStruct::default_number(field_1);
let instance = TupleStructBuilder::new().field_0(12345).build();
```
