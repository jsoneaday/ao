pub trait IsOptionString {}
impl IsOptionString for Option<String> {}

pub trait IsNoneOrEmpty: IsOptionString {
    fn is_none_or_empty(&self) -> bool;
}

impl IsNoneOrEmpty for Option<String> {
    fn is_none_or_empty(&self) -> bool {
        if self.is_none() || self.clone().unwrap().is_empty() {
            return true;
        }
        false
    }
}