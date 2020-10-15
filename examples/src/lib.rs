use scones::make_constructor;

#[make_constructor(pub(crate) fn mew2())]
pub struct Basic {
    #[value(0 for mew2)]
    value: i32,
    #[value(12)]
    #[value(value for new)]
    value2: i32,
    extra_extra: Vec<Vec<String>>
}
