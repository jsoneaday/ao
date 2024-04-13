use crate::{data_item::DataItem, key_utils::CryptoDriver};

pub fn get_signature_data(item: &mut DataItem) -> [u8; 48] {
    let mut vec: Vec<u8> = vec![];
    vec.append(CryptoDriver::string_to_buffer("dataitem").to_vec().as_mut());
    vec.append(CryptoDriver::string_to_buffer("1").to_vec().as_mut());
    vec.append(CryptoDriver::string_to_buffer(&item.base.signature_type.to_string()).to_vec().as_mut());
    vec.append(item.base.raw_owner.as_mut());
    vec.append(item.base.raw_target.as_mut());
    vec.append(item.base.raw_anchor.as_mut());
    vec.append(item.base.raw_tags.as_mut());
    vec.append(item.base.raw_data.as_mut());

    return CryptoDriver::deep_hash(&vec);
}
