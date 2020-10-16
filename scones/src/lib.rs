//! Scones is a library for generating constructors and builders without the verbosity it usually
//! requires. See the documentation for `#[make_constructor]` to see how to use this system. The
//! syntax and usage of `#[make_builder]` is similar to `#[make_constructor]` apart from a few
//! minor differences. A short example of how this crate works is as follows:
//!
//! ```
//! use scones::make_constructor;
//!
//! #[make_constructor]
//! #[make_constructor(pub inverse)]
//! #[make_constructor(pub identity)]
//! struct MyData {
//!     #[value(1 for identity)]
//!     val1: i32,
//!     #[value(-val1 for inverse)]
//!     #[value(1 for identity)]
//!     val2: i32,
//!     #[value(true)]
//!     always_true: bool
//! }
//!
//! let instance = MyData::new(10, 23);
//! let inverse = MyData::inverse(5);
//! let identity = MyData::identity();
//! ```

use std::marker::PhantomData;

/// Proc macro to generate builders for structs.
///
/// It is recommended to read the documentation of `#[make_constructor]` before reading this.
///
/// # Basic Usage
/// The simplest way to use this macro is without any additional arguments:
/// ```
/// use scones::make_builder;
///
/// #[make_builder]
/// struct MyStruct {
///     int: i32,
///     string: String,
/// }
///
/// let instance = MyStructBuilder::new()
///     .int(10)
///     .string("Hello World".to_owned())
///     .build();
/// ```
///
/// # Syntax
/// The full syntax of this macro is as follows:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_builder(visibility name params return_type)]
/// # */
/// ```
/// Each of these elements are optional but must always be present in the order listed above. If an
/// element is omitted, a default value is used instead. Invoking the macro without any of the
/// arguments listed above is equivalent to:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_builder(pub <StructName>Builder(..) -> Self)]
/// # */
/// ```
/// To make the visibility of the generated builder blank, provide a name but no visibility, like
/// so:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_builder(PrivateBuilder)]
/// # */
/// ```
///
/// ### Params
/// This argument can be used to provide additional parameters or make parameters optional.
/// It is a comma-seperated list of parameters enclosed in parenthesis. To add an extra parameter
/// (I.E. one which does not correspond to a field in your struct), use Rust's regular function
/// parameter syntax:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_builder((custom_param: i32))]
/// # */
/// ```
/// By default, this parameter will be required, meaning code that uses your builder will not
/// compile if it does not set a value for `custom_param`. If you want to make it optional, make
/// the type `Option<_>`. Note that the the macro is expecting the literal text `Option`, you
/// cannot use a type alias.
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_builder((optional: Option<i32>))]
/// # */
/// ```
/// Override fields can be specified with the following syntax, more on what this means later:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_builder((field_name?))]
/// # */
/// ```
///
/// ### Return Type
/// The return type can either be `-> Self` or `-> Result<Self, [any type]>`. Note that the macro
/// is expecting the literal text `Self` and/or `Result`, it is not capable of recognizing type
/// aliases like `std::fmt::Result`. Here is an example of how to make a builder that can return
/// an error:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_builder(-> Result<Self, FileError>)]
/// # */
/// ```
///
/// # Value Attributes
/// You can use the `#[value()]` attribute to add custom code for initializing a field:
/// ```
/// use scones::make_builder;
///
/// #[make_builder]
/// struct MyStruct {
///     #[value(123)]
///     data: i32
/// }
///
/// // We no longer need to specify a value for `data`.
/// let instance = MyStructBuilder::new().build();
/// ```
/// You can place any expression inside the parenthesis. Keep in mind that fields are initialized in
/// the order you declare them, so take care not to use parameters after they are moved:
/// ```compile_fail
/// use scones::make_builder;
///
/// #[make_builder]
/// struct MyStruct {
///     field_0: String,
///     #[value(field_0.clone())]
///     field_1: String,
/// }
/// ```
/// You can make a value attribute only apply to a certain builder by appending
/// `for BuilderName` to the end. You can do this multiple times for a single field of your
/// struct. If you have a value attribute without a `for` clause and multiple value attributes with
/// `for` clauses on the same field, the one without the clause will be used as a default for
/// whenever there is not a specific value attribute for a particular builder:
/// ```
/// use scones::make_builder;
///
/// #[make_builder(DefaultBuilder)]
/// #[make_builder(SpecificBuilder)]
/// struct MyStruct {
///     #[value(0)]
///     #[value(31415 for SpecificBuilder)]
///     data: i32,
/// }
///
/// let data_is_zero = DefaultBuilder::new().build();
/// let data_is_31415 = SpecificBuilder::new().build();
/// ```
/// When a field has a value attribute, the macro will not automatically add it to the parameters
/// for the builder. If you still want it to be a parameter despite this, you can explicitly add
/// it back to the parameter list of the builder:
/// ```
/// use scones::make_builder;
///
/// #[make_builder((data))]
/// struct MyStruct {
///     #[value(data + 2)]
///     data: i32
/// }
///
/// let data_is_10 = MyStructBuilder::new().data(8).build();
/// ```
///
/// # Required, Optional, and Override parameters
/// By default, all parameters for a builder are required. This means that the following code will
/// not compile:
/// ```compile_fail
/// use scones::make_builder;
///
/// #[make_builder]
/// struct MyStruct {
///     data: i32
/// }
///
/// // Ok
/// let instance = MyStructBuilder::new().data(0).build();
/// // Compile error! ("build() does not exist on type MyStructBuilder<Missing>")
/// let instance = MyStructBuilder::new().build();
/// ```
/// As mentioned before, you can add a parameter and explicitly give it an `Option<>` datatype
/// to make it optional, in which case it does not matter whether or not you specify its value
/// when using the builder, your code will still compile. One common use of this is to have a 
/// default value for a particular field, but allow a user to change it. The long way to do that
/// would be as follows:
/// ```
/// use scones::make_builder;
/// 
/// #[make_builder((data: Option<i32>))]
/// struct MyStruct {
///     #[value(data.unwrap_or(100))]
///     data: i32
/// }
/// ```
/// However, the case shown above is a fairly common and straightforward pattern, so the following
/// shortcut was created which produces identical results:
/// ```
/// use scones::make_builder;
/// 
/// #[make_builder((data?))]
/// struct MyStruct {
///     #[value(100)]
///     data: i32
/// }
/// ```
/// The usage of `data?` is called an "override" because it is not required, but when it is 
/// provided, it will *override* the default value of `data`.
///
/// # Templates and Tuple Structs
/// All the above semantics work with templated structs:
/// ```
/// use scones::make_builder;
///
/// #[make_builder]
/// // This also works with `where T: ToString`.
/// struct MyStruct<T: ToString> {
///     #[value(data.to_string())]
///     text: String,
///     data: T,
/// };
///
/// let instance = MyStructBuilder::new().data(123).build();
/// ```
/// All the above semantics are supported with tuple structs as well, the only difference being that
/// fields are given the names `field_0`, `field_1`, etc.
/// ```
/// use scones::make_builder;
///
/// #[make_builder]
/// struct MyTuple(
///     i32,
///     #[value(field_0)] i32,
/// );
///
/// let instance = MyTupleBuilder::new().field_0(123).build();
/// ```
pub use scones_macros::make_builder;

pub use scones_macros::generate_items__;
/// Proc macro to generate constructors for structs.
///
/// # Basic Usage
/// The simplest way to use this macro is without any additional arguments:
/// ```
/// use scones::make_constructor;
///
/// #[make_constructor]
/// struct MyStruct {
///     int: i32,
///     string: String,
/// }
///
/// // The macro generates:
/// // impl MyStruct {
/// //     pub fn new(int: i32, string: String) -> Self {
/// //         Self {
/// //             int,
/// //             string,
/// //         }
/// //     }
/// // }
/// ```
///
/// # Syntax
/// The full syntax of this macro is as follows:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_constructor(visibility name params return_type)]
/// # */
/// ```
/// Each of these elements are optional but must always be present in the order listed above. If an
/// element is omitted, a default value is used instead. Invoking the macro without any of the
/// arguments listed above is equivalent to:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_constructor(pub new(..) -> Self)]
/// # */
/// ```
/// To make the visibility of the generated function blank, provide a name but no visibility, like
/// so:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_constructor(private_new)]
/// # */
/// ```
///
/// ### Params
/// This argument can be used to rearrange the order of generated parameters or provide additional
/// parameters. It is a comma-seperated list of parameters enclosed in parenthesis. To specify the
/// location of a parameter for a particular field, use the name of that field:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_constructor((second_field, first_field))]
/// # */
/// ```
/// To add an extra parameter (I.E. one which does not correspond to a field in your struct), use
/// Rust's regular function parameter syntax:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_constructor((field, custom_param: i32))]
/// # */
/// ```
/// You can also use ellipses to specify where any other required parameters should be inserted.
/// If the macro detects that you have not explicitly given a position for a required parameter,
/// it will insert them wherever you place the ellipses:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// // Generates `pub fn new(field_a, field_b, custom_param) -> Self`.
/// #[make_constructor((.., custom_param: i32))]
/// # */
/// ```
///
/// ### Return Type
/// The return type can either be `-> Self` or `-> Result<Self, [any type]>`. Note that the macro
/// is expecting the literal text `Self` and/or `Result`, it is not capable of recognizing type
/// aliases like `std::fmt::Result`. Here is an example of how to make a constructor that can return
/// an error:
/// ```
/// # /* This little bit of trickery makes this not be tested without telling doc readers.
/// #[make_constructor(-> Result<Self, FileError>)]
/// # */
/// ```
///
/// # Value Attributes
/// You can use the `#[value()]` attribute to add custom code for initializing a field:
/// ```
/// use scones::make_constructor;
///
/// #[make_constructor]
/// struct MyStruct {
///     #[value(123)]
///     data: i32
/// }
///
/// // The macro generates:
/// // impl MyStruct {
/// //     fn new() -> Self {
/// //         Self {
/// //             data: 123,
/// //         }
/// //     }
/// // }
/// ```
/// You can place any expression inside the parenthesis. Keep in mind that fields are initialized in
/// the order you declare them, so take care not to use parameters after they are moved:
/// ```compile_fail
/// use scones::make_constructor;
///
/// #[make_constructor]
/// struct MyStruct {
///     field_0: String,
///     #[value(field_0.clone())]
///     field_1: String,
/// }
///
/// // The macro generates:
/// impl MyStruct {
///     pub fn new(field_0: String) -> Self {
///         Self {
///             field_0: field_0,
///             field_1: field_0.clone()
///         }
///     }
/// }
/// ```
/// You can make a value attribute only apply to a certain constructor by appending
/// `for constructor_name` to the end. You can do this multiple times for a single field of your
/// struct. If you have a value attribute without a `for` clause and multiple value attributes with
/// `for` clauses on the same field, the one without the clause will be used as a default for
/// whenever there is not a specific value attribute for a particular constructor:
/// ```
/// use scones::make_constructor;
///
/// #[make_constructor(default)]
/// #[make_constructor(specific)]
/// struct MyStruct {
///     #[value(0)]
///     #[value(31415 for specific)]
///     data: i32,
/// }
///
/// // The macro generates:
/// // impl MyStruct {
/// //     pub fn default() -> Self {
/// //         Self { data: 0 }
/// //     }
/// //     pub fn specific() -> Self {
/// //         Self { data: 31415 }
/// //     }
/// // }
/// ```
/// When a field has a value attribute, the macro will not automatically add it to the parameters
/// for the constructor. If you still want it to be a parameter despite this, you can explicitly add
/// it back to the parameter list of the constructor:
/// ```
/// use scones::make_constructor;
///
/// #[make_constructor((data))]
/// struct MyStruct {
///     #[value(data + 2)]
///     data: i32
/// }
///
/// // The macro generates:
/// // impl MyStruct {
/// //     pub fn new(data: i32) -> Self {
/// //         Self { data: data + 2 }
/// //     }
/// // }
/// ```
///
/// # Templates and Tuple Structs
/// All the above semantics work with templated structs:
/// ```
/// use scones::make_constructor;
///
/// #[make_constructor]
/// // This also works with `where T: ToString`.
/// struct MyStruct<T: ToString> {
///     #[value(data.to_string())]
///     text: String,
///     data: T,
/// };
///
/// // The macro generates:
/// // impl<T: ToString> MyTuple<T> {
/// //     pub fn new(data: T) -> Self {
/// //         Self {
/// //             text: data.to_string(),
/// //             data: data,
/// //         }
/// //     }
/// // }
/// ```
/// All the above semantics are supported with tuple structs as well, the only difference being that
/// fields are given the names `field_0`, `field_1`, etc.
/// ```
/// use scones::make_constructor;
///
/// #[make_constructor]
/// struct MyTuple(
///     i32,
///     #[value(field_0)] i32,
/// );
///
/// // The macro generates:
/// // impl MyTuple {
/// //     pub fn new(field_0: i32) -> Self {
/// //         Self(field_0, field_0)
/// //     }
/// // }
/// ```
pub use scones_macros::make_constructor;

/// Indicates that a particular required value has been provided in a builder.
pub struct Present;
/// Indicates that a particular required value has not been provided yet in a builder.
pub struct Missing;
#[doc(hidden)]
/// Used to implement builders.
pub struct BuilderFieldContainer<FieldType, IsPresent> {
    data: Option<FieldType>,
    marker_: PhantomData<IsPresent>,
}

impl<FieldType, IsPresent> BuilderFieldContainer<FieldType, IsPresent> {
    pub fn set(self, value: FieldType) -> BuilderFieldContainer<FieldType, Present> {
        BuilderFieldContainer {
            data: Some(value),
            marker_: PhantomData,
        }
    }
}

impl<FieldType> BuilderFieldContainer<FieldType, Missing> {
    pub fn missing() -> Self {
        Self {
            data: None,
            marker_: PhantomData,
        }
    }
}

impl<FieldType> BuilderFieldContainer<FieldType, Present> {
    pub fn present(value: FieldType) -> Self {
        Self {
            data: Some(value),
            marker_: PhantomData,
        }
    }

    pub fn into_value(self) -> FieldType {
        // The only way for IsPresent to be Present is if the user called set() in the past.
        self.data.unwrap()
    }
}
