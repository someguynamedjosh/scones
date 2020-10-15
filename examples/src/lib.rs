use scones::make_builder;
use scones::make_constructor;

// #[make_constructor]
// pub struct Basic {
//     pub int: i32,
//     pub string: String,
// }

// #[make_constructor(pub fn new(a: i32, b: i32))]
// pub struct CustomArgs {
//     #[value(a * b)]
//     pub product: i32,
//     #[value(a + b)]
//     pub sum: i32,
// }

// #[make_constructor]
// #[make_constructor(pub fn new_identical(shared: i32))]
// pub struct MultipleConstructors {
//     #[value(shared for new_identical)]
//     pub a: i32,
//     #[value(shared for new_identical)]
//     pub b: i32,
//     #[value(shared for new_identical)]
//     pub c: i32,

//     #[value(true)]
//     #[value(false for new)]
//     pub identical: bool,
// }

pub struct Test {
    a: i32,
    b: i32,
}

pub struct TestBuilder<AStatus__, BStatus__> {
    a: ::scones::BuilderFieldContainer<i32, AStatus__>,
    b: ::scones::BuilderFieldContainer<i32, BStatus__>,
}

impl TestBuilder<::scones::Missing, ::scones::Missing> {
    pub fn new() -> Self {
        Self {
            a: ::scones::BuilderFieldContainer::missing(),
            b: ::scones::BuilderFieldContainer::missing(),
        }
    }
}

impl<AStatus__, BStatus__> TestBuilder<AStatus__, BStatus__> {
    pub fn a(self, value: i32) -> TestBuilder<::scones::Present, BStatus__> {
        TestBuilder {
            a: ::scones::BuilderFieldContainer::present(value),
            b: self.b,
        }
    }

    pub fn b(self, value: i32) -> TestBuilder<AStatus__, ::scones::Present> {
        TestBuilder {
            a: self.a,
            b: ::scones::BuilderFieldContainer::present(value),
        }
    }
}

impl TestBuilder<::scones::Present, ::scones::Present> {
    pub fn build(self) -> Test {
        Test {
            a: self.a.into_value(),
            b: self.b.into_value(),
        }
    }
}

pub fn test() {
    let value = TestBuilder::new().a(12).b(24).a(12).build();
}

#[make_builder((a?))]
pub struct Test2 {
    #[value(12)]
    a: i32,
    b: i32,
}
