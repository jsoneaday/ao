use crate::
{
    transaction::TxValue,
    hasher::sha384,
};

/// A function for recursive hash calculation to generate the merkle
/// root of all the neccessary transaction fields. The resulted merkle
/// root is the final data that ready to be signed.
pub fn recursive_hash(data: &TxValue) -> Vec<u8> {
    let result: Vec<u8>;
    match data {
        TxValue::Vec(vec_data) => {
            let tag_list_b = "list".to_string().into_bytes();
            let tag_len_b = vec_data.len().to_string().into_bytes();
            let mut tag = Vec::new();
            tag.extend(tag_list_b);
            tag.extend(tag_len_b);
            let tag_sha384 = sha384(&tag);
            result = recursive_hash_vec(data, &tag_sha384);
        },
        TxValue::Bytes(bytes_data)=> {
            let tag_blob_b = "blob".to_string().into_bytes();
            let tag_len_b = bytes_data.len().to_string().into_bytes();
            let mut tag = Vec::new();
            tag.extend(tag_blob_b);
            tag.extend(tag_len_b);
            let mut tagged_data = Vec::new();
            tagged_data.extend(sha384(&tag));
            tagged_data.extend(sha384(&bytes_data));
            result = sha384(&tagged_data);
        },
    }
    result
}

pub fn recursive_hash_vec(data: &TxValue, acc: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    if let TxValue::Vec(vec_data) = data {
        if vec_data.len() < 1 {
            result = acc.to_vec();
        } else {
            let mut new_acc_pair = Vec::new();
            new_acc_pair.extend(acc);
            new_acc_pair.extend(recursive_hash(&vec_data[0]));
            let new_acc = sha384(&new_acc_pair);
            let mut new_vec_data: Vec<TxValue> = Vec::new();
            new_vec_data.extend(vec_data.clone());
            new_vec_data.remove(0);
            let new_data =  TxValue::Vec(new_vec_data);
            result = recursive_hash_vec(&new_data, &new_acc);
        }
    }
    result
}
