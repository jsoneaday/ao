use crate::data_item::DataItem;
use crate::errors::ArBundleErrors;
use crate::signing::signer::Signer;
use crate::tags::{serialize_tags, Tag};
use crate::utils::{long_to_8_byte_array, short_to_2_byte_array};

pub struct DataItemCreateOptions {
    pub target: Option<String>,
    pub anchor: Option<String>,
    pub tags: Option<Vec<Tag>>
}

pub enum Data {
    StringData(String),
    BinaryData(Vec<u8>)
}

pub fn create_data(data: Data, signer: &Signer, opts: Option<&DataItemCreateOptions>) -> Result<DataItem, ArBundleErrors> {
    let _owner = &signer.public_key;

    let _target = if opts.is_some() && opts.unwrap().target.is_some() {
        let target = &opts.unwrap().target.as_ref().unwrap();
        let mut output = vec![];
        base64_url::encode_to_vec(target, &mut output);
        Some(output)
    } else { None };
    let target_length = 1 + (if let Some(_target) = _target.clone() {
        _target.len()
    } else { 0 });
    let _anchor = if opts.is_some() && opts.unwrap().anchor.is_some() {
        Some(opts.unwrap().anchor.as_ref().unwrap().as_bytes())
    } else { None };
    let anchor_length = 1 + (if let Some(_anchor) = _anchor.clone() {
        _anchor.len()
    } else { 0 });
    let _tags = if opts.is_some() && opts.unwrap().tags.is_some() && opts.unwrap().tags.as_ref().unwrap().len() > 0 {
        match serialize_tags(opts.unwrap().tags.as_ref().unwrap()) {
            Ok(_tags) => Some(_tags),
            Err(_) => None
        }
    } else {
        None
    };
    let tags_length = 16 + (if let Some(_tags) = _tags.clone() {
        _tags.len()
    } else { 0 });
    let _data = match data {
        Data::StringData(string_data) => string_data.as_bytes().to_vec(),
        Data::BinaryData(binary_data) => binary_data
    };
    let data_length = _data.len();

    let length = 2 + signer.signature_length + signer.owner_length + target_length + anchor_length + tags_length + data_length;
    let mut bytes = vec![0; length];

    bytes[0..2].copy_from_slice(&short_to_2_byte_array(signer.signature_type).unwrap());
    bytes[2..2 + signer.signature_length as usize].fill(0);

    if _owner.len() != signer.owner_length as usize {
        return Err(
            ArBundleErrors::IoFailure(
                std::io::Error::new(std::io::ErrorKind::Other, 
                    format!("Owner must be {} bytes, but was incorrectly {}", signer.owner_length, _owner.len())
                )
            )
        );
    }
    bytes[2 + signer.signature_length..2 + signer.signature_length + signer.owner_length].copy_from_slice(&_owner);
    
    let position = 2 + signer.signature_length + signer.owner_length;
    bytes[position] = if let Some(_target) = _target.clone() { 1 } else { 0 };
    if let Some(_target) = _target {
        if _target.len() != 32 {
            return Err(ArBundleErrors::IoFailure(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Target must be 32 bytes but was incorrectly {}", _target.len())
                )
            ))
        }
        bytes[position + 1..position + 1 + _target.len()].copy_from_slice(&_target);
    }

    let anchor_start = position + target_length;
    let mut tags_start = anchor_start + 1;
    bytes[anchor_start] = if let Some(_anchor) = _anchor.clone() { 1 } else { 0 };
    if let Some(_anchor) = _anchor.clone() {
        tags_start += _anchor.len();
        if _anchor.len() != 32 {
            return Err(ArBundleErrors::IoFailure(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Anchor must be 32 bytes")
                )
            ))
        }
        bytes[anchor_start + 1..anchor_start + 1 + _anchor.len()].copy_from_slice(&_anchor);
    }

    let tags_length_or_0 = if opts.is_some() && opts.unwrap().tags.is_some() {
        opts.unwrap().tags.as_ref().unwrap().len()
    } else { 0 };
    let long_to_8_byte_array_result = long_to_8_byte_array(tags_length_or_0 as i64);
    bytes[tags_start..tags_start + long_to_8_byte_array_result.len()].copy_from_slice(&long_to_8_byte_array_result);
    let tags_bytes_length = if _tags.clone().is_some() {
        _tags.clone().unwrap().len()
    } else { 0 };
    let bytes_count = long_to_8_byte_array(tags_bytes_length as i64);
    bytes[tags_start + 8..tags_start + 8 + bytes_count.len()].copy_from_slice(&bytes_count);
    if _tags.clone().is_some() {
        bytes[tags_start + 16..tags_start + 16 + _tags.clone().unwrap().len()].copy_from_slice(&_tags.unwrap());
    }

    let data_start = tags_start + tags_length;

    bytes[data_start..data_start + _data.len()].copy_from_slice(&_data);

    Ok(DataItem::new(bytes, &signer.keypair_path))
}