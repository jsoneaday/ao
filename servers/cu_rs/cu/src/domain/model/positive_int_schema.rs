use std::borrow::Cow;
use valid::{invalid_value, invalid_optional_value, ConstraintViolation, FieldName, Validate, Validation, ValidationError};
use regex::Regex;
use super::shared_validation::{INVALID_EMPTY, INVALID_NOT_MATCH_UNDERSCORE, INVALID_NOT_MATCH_NUMBER};

const LONG_UNDERSCORE_INTEGER: &str = r"^[0-9]+(?:_[0-9]+)*$";

pub fn parse_positive_int_schema(val: Option<String>, field_name: &str) -> Result<i64, ValidationError> {
    let validation = val.clone().validate(field_name.to_string(), &PositiveIntSchemaConstraint).result();
    let error_msg: &str = "Provided invalid value for positive_int_schema";
    if let Err(mut e) = validation {
        e.message = Some(Cow::from(error_msg));
        return Err(e);
    }

    if let None = val.clone() {
        return Ok(-1);
    }
    let regex = Regex::new(LONG_UNDERSCORE_INTEGER).unwrap(); 
    let _val = val.clone().unwrap();
    let _val = _val.as_str();
    if regex.is_match(_val) {
        let final_val = _val.replace("_", "");
        let final_val = final_val.parse::<i64>().unwrap();
        return Ok(final_val);
    } else if let Ok(val) = _val.parse::<i64>() {
        return Ok(val);
    }

    // just in case
    Err(ValidationError {
        message: Some(Cow::from(error_msg)),
        violations: vec![]
    })
}

pub struct PositiveIntSchemaConstraint;

impl Validate<PositiveIntSchemaConstraint, FieldName> for Option<String> {
    fn validate(self, context: impl Into<FieldName>, _constraint: &PositiveIntSchemaConstraint) -> valid::Validation<PositiveIntSchemaConstraint, Self> {        
        let mut violations: Vec<ConstraintViolation> = vec![];
        let context: FieldName = context.into();
        if let None = self.clone() {
            violations.push(invalid_optional_value(INVALID_EMPTY, context.clone(), None, None));
        } else {
            let _val = self.clone().unwrap();
            let _val = _val.as_str().trim();
                    
            let regex = Regex::new(LONG_UNDERSCORE_INTEGER).unwrap();
            if !regex.is_match(_val) {
                if let Err(_e) = _val.parse::<i64>() {
                    violations.push(invalid_value(INVALID_NOT_MATCH_NUMBER, context, self.clone().unwrap(), "number value".to_string()));
                } else {
                    violations.push(invalid_value(INVALID_NOT_MATCH_UNDERSCORE, context.clone(), self.clone().unwrap(), "number value".to_string()));
                }
            }
        }

        if violations.len() > 0 {
            return Validation::failure(violations);
        } 
        Validation::success(self)
    }
}