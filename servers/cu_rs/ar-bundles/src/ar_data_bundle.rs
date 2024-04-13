use crate::{ar_data_base::get_signature_data, data_item::DataItem, key_utils::get_crypto_driver, signing::signer::SignerMaker};

#[derive(Debug)]
pub struct ArDataBundles { 
    signature: Vec<u8>, 
    id: Vec<u8>
}

pub fn get_signature_and_id<T: SignerMaker>(item: &mut DataItem, signer: &T) -> ArDataBundles {
    let signature_data = get_signature_data(item);
  
    let signature_bytes = signer.sign(&signature_data.to_vec()).unwrap();
    let id_bytes = get_crypto_driver(item.base.keypair_path.as_ref()).hash(&signature_bytes);
  
    ArDataBundles { signature: signature_bytes.to_vec(), id: id_bytes.to_vec() }
}

pub fn sign<T: SignerMaker>(item: &mut DataItem, signer: &T) -> Result<Vec<u8>, ArDataBundles> {
    let ArDataBundles { signature, id } = get_signature_and_id(item, signer);
    item.get_raw()[2..signature.len()].copy_from_slice(&signature);
    Ok(id)
}