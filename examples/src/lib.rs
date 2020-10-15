use scones::make_builder;
use scones::make_constructor;

#[make_constructor]
pub struct Basic {
    pub int: i32,
    pub string: String,
}

#[make_constructor(pub new(a: i32, b: i32))]
pub struct CustomArgs {
    #[value(a * b)]
    pub product: i32,
    #[value(a + b)]
    pub sum: i32,
}

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

#[make_constructor]
pub struct Templated<T> where T: Sized {
    pub data: T
}
