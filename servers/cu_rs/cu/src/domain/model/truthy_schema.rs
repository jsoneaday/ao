use valid::{invalid_value, FieldName, Validate, Validation, ValidationError};

pub const INVALID_NOT_BOOLEAN: &str = "invalid-not-boolean";

pub fn parse_boolean_schema(val: Option<String>) -> Result<bool, ValidationError> {
    let result = val.validate("DISABLE_PROCESS_CHECKPOINT_CREATION", &TruthyConstraint).result();
    match result {
        Ok(_val) => Ok(true),
        Err(e) => Err(e)
    }
}

pub struct TruthyConstraint;

impl TruthyConstraint {
    pub fn is_boolean(&self, val: Option<String>) -> bool {        
        if val.is_some() {
            let unwrapped_val = val.unwrap();
            let val_str = unwrapped_val.as_str();
            if val_str == "true" || val_str == "false" {
                return true;
            }
        }
        return false;
    }
}

impl Validate<TruthyConstraint, FieldName> for Option<String> {
    fn validate(self, context: impl Into<FieldName>, constraint: &TruthyConstraint) -> Validation<TruthyConstraint, Self> {
        if constraint.is_boolean(self.clone()) {
            return Validation::success(self);
        }
        Validation::failure(vec![invalid_value("invalid-boolean", context, if self.is_none() { "".to_string() } else { self.unwrap() }, "value must be js boolean".to_string())])
    }
}