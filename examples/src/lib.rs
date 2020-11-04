use scones::{make_builder, make_constructor};

/// A basic example which generates a default constructor.
///
/// It is defined as follows:
/// ```
/// # use scones::*;
/// #[make_constructor]
/// pub struct Basic {
///     pub int: i32,
///     pub string: String,
/// }
/// ```
#[make_constructor]
pub struct Basic {
    pub int: i32,
    pub string: String,
}

#[test]
pub fn basic_demo() {
    let _instance = Basic::new(123, "hello".to_string());
}

/// An example showing how to add extra arguments to a constructor and use those arguments to
/// initialize the existing fields.
///
/// It is defined as follows:
/// ```
/// # use scones::*;
/// #[make_constructor(pub new(a: i32, b: i32))]
/// /// ^ Returns a new instance of `CustomArgs` with `product` equal to `a * b` and `sum` equal to
/// /// ^ `a + b`.
/// pub struct CustomArgs {
///     #[value(a * b)]
///     pub product: i32,
///     #[value(a + b)]
///     pub sum: i32,
/// }
/// ```
/// Note that if the `#[value()]` annotations were ommitted for any field, the macro would add that
/// field as an additional argument to the constructor automatically.
#[make_constructor(pub new(a: i32, b: i32))]
/// ^ Returns a new instance of `CustomArgs` with `product` equal to `a * b` and `sum` equal to
/// ^ `a + b`.
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

/// An example showing how to efficiently create multiple constructors.
///
/// It is defined as follows:
/// ```
/// # use scones::*;
/// #[make_constructor]
/// /// ^ This is documentation for the first constructor. Notice how a, b, and c have been
/// /// ^ automatically generated for us.
/// #[make_constructor(pub new_identical(shared: i32))]
/// /// ^ This is documentation for the second constructor. Notice how the only argument is
/// /// ^ `shared`, since we have specified the code that sets up all the other fields in this case.
/// pub struct MultipleConstructors {
///     // Note that we do not provide a default `value` or `[value] for new` for any of these
///     // fields, so the macro will automatically add parameters for them in the `new` constructor.
///     #[value(shared for new_identical)]
///     pub a: i32,
///     #[value(shared for new_identical)]
///     pub b: i32,
///     #[value(shared for new_identical)]
///     pub c: i32,
///
///     /// This field will always be `true` when created with `new_identical` and `false` when
///     /// created with `new`.
///     #[value(true)]
///     #[value(false for new)]
///     pub identical: bool,
/// }
/// ```
#[make_constructor]
/// ^ This is documentation for the first constructor. Notice how a, b, and c have been
/// ^ automatically generated for us.
#[make_constructor(pub new_identical(shared: i32))]
/// ^ This is documentation for the second constructor. Notice how the only argument is
/// ^ `shared`, since we have specified the code that sets up all the other fields in this case.
pub struct MultipleConstructors {
    // Note that we do not provide a default `value` or `[value] for new` for any of these fields,
    // so the macro will automatically create parameters for them in the `new` constructor.
    #[value(shared for new_identical)]
    pub a: i32,
    #[value(shared for new_identical)]
    pub b: i32,
    #[value(shared for new_identical)]
    pub c: i32,

    /// This field will always be `true` when created with `new_identical` and `false` when
    /// created with `new`.
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

/// An example showing how to return a `Result` from a construtor.
///
/// It is defined as follows:
/// ```
/// # use scones::*;
/// #[make_constructor((text: &str) -> Result<Self, std::num::ParseIntError>)]
/// pub struct ReturnResult {
///     #[value(text.parse()?)]
///     pub number: i32
/// }
/// ```
/// Note that you must use `Result` and you cannot use any type alias for it. The macro will
/// make sure `::std::result::Result` specifically is used.
#[make_constructor((text: &str) -> Result<Self, std::num::ParseIntError>)]
pub struct ReturnResult {
    #[value(text.parse()?)]
    pub number: i32,
}

#[test]
pub fn return_result_demo() {
    let instance = ReturnResult::new("123").unwrap();
    assert_eq!(instance.number, 123);
    assert!(ReturnResult::new("alskdjf").is_err());
}

/// An example showing the semantics for tuple structs.
///
/// It is defined as follows:
/// ```
/// # use scones::*;
/// #[make_constructor]
/// #[make_constructor(pub default_number)]
/// #[make_builder((field_1?))]
/// pub struct TupleStruct(
///     #[value(30 for default_number)] i32,
///     #[value("Unnamed".to_owned() for TupleStructBuilder)] String,
/// );
/// ```
#[make_constructor]
#[make_constructor(pub default_number)]
#[make_builder((field_1?))]
pub struct TupleStruct(
    #[value(30 for default_number)] i32,
    #[value("Unnamed".to_owned() for TupleStructBuilder)] String,
);

#[test]
pub fn tuple_struct_demo() {
    let instance = TupleStructBuilder::new().field_0(20).build();
    assert_eq!(instance.1, "Unnamed");
}

/// An example showing how to create a builder.
///
/// It is defined as follows:
/// ```
/// # use scones::*;
/// #[make_builder]
/// pub struct BasicBuilt {
///     pub int: i32,
///     pub string: String,
/// }
/// ```
#[make_builder]
pub struct BasicBuilt {
    pub int: i32,
    pub string: String,
}

#[test]
pub fn basic_built_demo() {
    let _instance = BasicBuiltBuilder::new()
        .int(10)
        .string("alskdjf".to_owned())
        .build();
}

/// An example showing how to add optional fields to a builder.
///
/// It is defined as follows:
/// ```
/// # use scones::*;
/// #[make_builder(pub OptionalBuilder(optional: f32))]
/// /// ^ An example of how to use this builder is as follows:
/// /// ^ ```
/// /// ^ let instance = OptionalBuilder::new().required(12).build();
/// /// ^ assert_eq!(instance.constructed_from_optional, 0);
/// /// ^ let instance = OptionalBuilder::new().required(12).optional(5.0).build();
/// /// ^ assert_eq!(instance.constructed_from_optional, 5);
/// /// ^ let instance = OptionalBuilder::new().optional(12.0).required(5).build();
/// /// ^ assert_eq!(instance.constructed_from_optional, 12);
/// /// ^ ```
/// pub struct OptionalBuilt {
///     pub required: i32,
///     #[value(optional as i32)]
///     pub constructed_from_optional: i32,
/// }
/// ```
/// Note that you must use the literal text `Option` and not use a type alias. The macro will
/// automatically change this to use `::std::option::Option`.
#[make_builder(pub OptionalBuilder(optional: Option<f32>))]
/// ^ An example of how to use this builder is as follows:
/// ^ ```ignore
/// ^ let instance = OptionalBuilder::new().required(12).build();
/// ^ assert_eq!(instance.constructed_from_optional, 0);
/// ^ let instance = OptionalBuilder::new().required(12).optional(5.0).build();
/// ^ assert_eq!(instance.constructed_from_optional, 5);
/// ^ let instance = OptionalBuilder::new().optional(12.0).required(5).build();
/// ^ assert_eq!(instance.constructed_from_optional, 12);
/// ^ ```
pub struct OptionalBuilt {
    pub required: i32,
    #[value(optional.unwrap_or(0.0) as i32)]
    pub constructed_from_optional: i32,
}

#[test]
pub fn optional_built_demo() {
    let instance = OptionalBuilder::new().required(12).build();
    assert_eq!(instance.constructed_from_optional, 0);
    let instance = OptionalBuilder::new().required(12).optional(5.0).build();
    assert_eq!(instance.constructed_from_optional, 5);
    let instance = OptionalBuilder::new().optional(12.0).required(5).build();
    assert_eq!(instance.constructed_from_optional, 12);
}

/// An example showing how to use overrides in builders.
///
/// Overrides are sugar for accomplishing the job the `OptionalBuilt` example does with less
/// verbosity. This example is defined as follows:
/// ```
/// # use scones::*;
/// #[make_builder(pub OverridableBuilder(defaults_to_zero?))]
/// pub struct OverridableBuilt {
///     #[value(0)]
///     pub defaults_to_zero: i32,
/// }
/// ```
/// The resulting builder will allow building this struct without specifying a value for
/// `defaults_to_zero`, in which case zero will be used. At the same time, it allows a user of
/// the builder to override that default, without you having to explicitly add a custom parameter,
/// give it an `Option<>` type, and `unwrap_or` it in the `#[value()]` annotation.
#[make_builder(pub OverridableBuilder(defaults_to_zero?))]
pub struct OverridableBuilt {
    #[value(0)]
    pub defaults_to_zero: i32,
}

#[test]
pub fn overridable_built_demo() {
    let instance = OverridableBuilder::new().build();
    assert_eq!(instance.defaults_to_zero, 0);
    let instance = OverridableBuilder::new().defaults_to_zero(12).build();
    assert_eq!(instance.defaults_to_zero, 12);
}

/// An example showing that all this crate's features work with templated types.
///
/// It is defined as follows:
/// ```
/// # use scones::*;
/// #[make_builder]
/// #[make_builder(pub TemplatedTryBuilder -> Result<Self, i32>)]
/// #[make_constructor]
/// #[make_constructor(pub try_new -> Result<Self, i32>)]
/// pub struct Templated<T> where T: Sized {
///     pub data: T
/// }
/// ```
#[make_builder]
#[make_builder(pub TemplatedTryBuilder -> Result<Self, i32>)]
#[make_constructor]
#[make_constructor(pub try_new -> Result<Self, i32>)]
pub struct Templated<T>
where
    T: Sized,
{
    pub data: T,
}

#[test]
pub fn templated_demo() {
    let instance = Templated::new(123);
    assert_eq!(instance.data, 123);
    let instance = TemplatedBuilder::new().data("Hello World!").build();
    assert_eq!(instance.data, "Hello World!");
}

#[make_constructor]
#[derive(Debug)]
struct SconesAndDerive { }
