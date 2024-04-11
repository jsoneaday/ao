use crate::data_item::DataItem;
use crate::signing::signer::Signer;
use crate::tags::{AVSCTap, Tag};

pub struct DataItemCreateOptions {
    pub target: Option<String>,
    pub anchor: Option<String>,
    pub tags: Option<Vec<Tag>>
}

pub enum Data {
    StringData(String),
    BinaryData(Vec<u8>)
}

pub fn create_data(data: Data, signer: &Signer, opts: Option<&DataItemCreateOptions>) -> DataItem {
    let _owner = &signer.public_key;

    let _target = if opts.is_some() && opts.unwrap().target.is_some() {
        Some(base64_url::encode(&opts.unwrap().target.as_ref().unwrap()))
    } else { None };
    let target_length = 1 + (if let Some(_target) = _target {
        base64_url::decode(&_target).unwrap().len()
    } else { 0 });
    let _anchor = if opts.is_some() && opts.unwrap().anchor.is_some() {
        Some(opts.unwrap().anchor.as_ref().unwrap().bytes())
    } else { None };
    let anchor_length = 1 + (if let Some(_anchor) = _anchor {
        _anchor.len()
    } else { 0 });
    let _tags = if opts.is_some() && opts.unwrap().tags.is_some() && opts.unwrap().tags.as_ref().unwrap().len() > 0 {
        Some(AVSCTap::serialize_tags(opts.unwrap().tags.as_ref().unwrap()))
    } else {
        None
    };
    let tags_length = 16 + (if let Some(_tags) = _tags {
        _tags.unwrap().len()
    } else { 0 });
    let _data = match data {
        Data::StringData(string_data) => string_data.as_bytes().to_vec(),
        Data::BinaryData(binary_data) => binary_data
    };
    let data_length = _data.len();

    DataItem
}