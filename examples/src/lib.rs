use scones::{make_builder, make_constructor};

#[make_constructor]
pub struct Basic {
    pub int: i32,
    pub string: String,
}

#[test]
pub fn basic_demo() {
    let _instance = Basic::new(123, "hello".to_string());
}

#[make_constructor(pub new(a: i32, b: i32))]
pub struct CustomArgs {
    #[value(a * b)]
    pub product: i32,
    #[value(a + b)]
    pub sum: i32,
}

#[test]
pub fn custom_args_demo() {
    let instance = CustomArgs::new(10, -30);
    assert_eq!(instance.product, -300);
    assert_eq!(instance.sum, -20);
}

#[make_constructor]
/// ^ This is documentation for the first constructor.
#[make_constructor(pub new_identical(shared: i32))]
/// ^ This is documentation for the second constructor.
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

#[test]
pub fn multiple_constructors_demo() {
    let instance = MultipleConstructors::new(1, 2, 3);
    assert!(!instance.identical);
    let instance = MultipleConstructors::new_identical(50);
    assert!(instance.identical);
    assert_eq!(instance.a, instance.b);
    assert_eq!(instance.b, instance.c);
}

#[make_constructor(pub (text: &str) -> Result<Self, std::num::ParseIntError>)]
pub struct ReturnResult {
    #[value(text.parse()?)]
    pub number: i32
}

#[test]
pub fn return_result_demo() {
    let instance = ReturnResult::new("123").unwrap();
    assert_eq!(instance.number, 123);
    assert!(ReturnResult::new("alskdjf").is_err());
}

#[make_builder]
/// ^ This is additional documentation for the builder.
#[make_builder(TemplatedTryBuilder -> Result<Self, i32>)]
#[make_constructor]
#[make_constructor(try_new -> Result<Self, i32>)]
pub struct Templated<T> where T: Sized {
    pub data: T
}

#[test]
pub fn templated_demo() {
    let instance = Templated::new(123);
    assert_eq!(instance.data, 123);
    let instance = TemplatedBuilder::new().data("Hello World!").build();
    assert_eq!(instance.data, "Hello World!");
}
