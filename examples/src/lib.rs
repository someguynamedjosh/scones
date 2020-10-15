use scones::make_constructor;

#[make_constructor(pub(crate) fn mew2(value2, .., custom: String))]
pub struct Basic {
    #[value = 0]
    value: i32,
    #[value = 0]
    value2: i32,
    extra_extra: Vec<Vec<String>>
}
