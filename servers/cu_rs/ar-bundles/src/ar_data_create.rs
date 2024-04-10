use crate::signing::signer::Signer;

pub struct DataItemCreateOptions {
    pub target: Option<String>,
    pub anchor: Option<String>,
    pub tags: Option<Vec<Tag>>
}

pub enum Data {
    StringData(String),
    BinaryData(Vec<u8>)
}

pub fn create_data(data: Data, signer: Signer, opts: Option<DataItemCreateOptions>) -> DataItem {
    let _owner = signer.public_key;

    let _target = if opts.is_some() && opts.unwrap().target.is_some() {
        Some(base64url::encode(opts.unwrap().target.unwrap().bytes()))
    } else { None };
    let target_length = 1 + (if let Some(_target) = _target {
        base64url::decode(&_target).unwrap().len()
    } else { 0 });
    let _anchor = if opts.is_some() && opts.unwrap().anchor.is_some() {
        Some(opts.unwrap().anchor.unwrap().bytes())
    } else { None };
    let anchor_length = 1 + (if let Some(_anchor) = _anchor {
        _anchor.len()
    } else { 0 });
    
}