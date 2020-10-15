use scones::make_constructor;

#[make_constructor(pub(crate) fn mew2(value2, .., custom: String))]
pub struct Basic {
    value: i32,
    value2: i32,
    extra_extra: Vec<Vec<String>>
}
