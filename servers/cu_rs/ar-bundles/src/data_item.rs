use std::any::Any;
use crate::{
    ar_data_base::get_signature_data, 
    bundle_item::{BundleItem, BundleItemFn}, 
    constants::SignatureConfig, 
    errors::ArBundleErrors, 
    signing::signer::SignerMaker, 
    tags::deserialize_tags, utils::byte_array_to_long
};
use async_trait::async_trait;
use crate::bundle_item::ResolvesTo;
use crate::ar_data_bundle::sign;
use arweave_rs::crypto::hash::sha256;

pub const MAX_TAG_BYTES: usize = 4096;
pub const MIN_BINARY_SIZE: usize = 80;

pub struct DataItem {
    pub base: BundleItem,
    binary: Vec<u8>,
    _id: Option<Vec<u8>>
}

impl DataItem {
    pub fn new(binary: Vec<u8>, keypair_path: &str) -> Self {
        Self {
            base: BundleItem {
                signature_type: ResolvesTo::Item(0),
                raw_signature: ResolvesTo::Item(vec![]),
                signature: ResolvesTo::Item("".to_string()),
                signature_length: ResolvesTo::Item(0),
                raw_owner: ResolvesTo::Item(vec![]),
                owner: ResolvesTo::Item("".to_string()),
                owner_length: ResolvesTo::Item(0),
                raw_target: ResolvesTo::Item(vec![]),
                target: ResolvesTo::Item("".to_string()),
                raw_anchor: ResolvesTo::Item(vec![]),
                anchor: ResolvesTo::Item("".to_string()),
                raw_tags: ResolvesTo::Item(vec![]),
                tags: ResolvesTo::Item(vec![]),
                raw_data: ResolvesTo::Item(vec![]),
                data: ResolvesTo::Item("".to_string()),
                keypair_path: ResolvesTo::Item(keypair_path.to_string())
            },
            binary,
            _id: None
        }
    }

    pub fn is_data_item(obj: Box<dyn Any>) -> bool {
        let test = obj.downcast_ref::<DataItem>();
        if test.is_some() {
            return true;
        }
        false
    }

    pub fn get_signature_type(&self) -> Result<SignatureConfig, ArBundleErrors> {
        let signature_type_val = byte_array_to_long(&self.binary[0..2]);
        if SignatureConfig::ARWEAVE as i64 == signature_type_val {
            return Ok(SignatureConfig::ARWEAVE);
        } else if SignatureConfig::ED25519 as i64 == signature_type_val {
            return Ok(SignatureConfig::ED25519);
        } else if  SignatureConfig::ETHEREUM as i64 == signature_type_val {
            return Ok(SignatureConfig::ETHEREUM);
        } else if SignatureConfig::SOLANA as i64 == signature_type_val {
            return Ok(SignatureConfig::SOLANA);
        } else if SignatureConfig::INJECTEDAPTOS as i64 == signature_type_val {
            return Ok(SignatureConfig::INJECTEDAPTOS);
        } else if SignatureConfig::MULTIAPTOS as i64 == signature_type_val {
            return Ok(SignatureConfig::MULTIAPTOS);
        } else if SignatureConfig::TYPEDETHEREUM as i64 == signature_type_val {
            return Ok(SignatureConfig::TYPEDETHEREUM);
        }
            
        Err(ArBundleErrors::SignatureConfigTypeNotFound)
    }

    fn get_tags_start(&self) -> usize {
        let target_start = self.get_target_start();
        let target_present = self.binary[target_start] == 1;
        let mut tags_start = target_start + (if target_present { 33 } else { 1 });
        let anchor_present = self.binary[tags_start] == 1;
        tags_start += if anchor_present { 33 } else { 1 };
    
        return tags_start;
    }

    fn get_target_start(&self) -> usize {
        return (2 + self.base.signature_length.as_ref() + self.base.owner_length.as_ref()) as usize;
    }

    pub fn get_raw_id(&self) -> [u8; 32] {
        sha256(self.base.raw_signature.as_ref())
    }

    pub fn get_raw(&self) -> Vec<u8> {
        self.binary.clone()
    }
}

#[async_trait]
impl BundleItemFn for DataItem {
    fn sign<T: SignerMaker>(&mut self, signer: &T) -> [u8; 32] {
        let signed = sign(self, signer);
        self._id = Some(signed.unwrap());

        self.get_raw_id()
    }

    fn is_valid<T: SignerMaker>(&self, signer: &T) -> bool {
        self.verify(&self.binary, signer)
    }

    fn verify<T: SignerMaker>(&self, buffer: &Vec<u8>, signer: &T) -> bool {
        if buffer.len() < MIN_BINARY_SIZE {
            return false;
        }

        let mut item = DataItem::new(buffer.clone(), signer.get_keypair_path().as_ref());
        let _sig_type = item.get_signature_type(); // will use if diff wallets ever supported
        let tags_start = item.get_tags_start();

        let number_of_tags = byte_array_to_long(&buffer[tags_start..(tags_start + 8)].to_vec());
        let number_of_tag_by_byte_array = buffer[tags_start + 8..tags_start + 16].to_vec();
        let number_of_tag_bytes = byte_array_to_long(&number_of_tag_by_byte_array);

        if number_of_tag_bytes as usize > MAX_TAG_BYTES { return false; }

        if number_of_tags > 0 {
            let rng = (tags_start + 16)..(tags_start + 16 + (number_of_tag_bytes as usize));
            let tags = deserialize_tags(buffer[rng].to_vec());

            if tags.len() != number_of_tags as usize {
                return false;
            }
        }

        let signature_data = get_signature_data(&mut item);
        // todo: switch to a call capable of using other signers when desired
        signer.verify(item.base.raw_owner.as_ref(), &signature_data.to_vec(), item.base.raw_signature.as_ref());

        false
    }
}