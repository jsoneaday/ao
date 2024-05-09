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

pub fn get_empty_string(value: Option<String>) -> String {
    if value.is_some() { 
        return value.unwrap();
    }
    "".to_string()
}

pub fn get_number_string(value: Option<i64>) -> String {
    if value.is_none() {
        return "".to_string();
    }
    value.unwrap().to_string()
}