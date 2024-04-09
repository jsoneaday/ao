use crate::{
    wallet::ArWallet,
    recursive_hash::recursive_hash,
    b64::{b64_encode},
    hasher::{sha256},
    chunks::{Chunks}
};

// use serde_json::{Value};
use base64ct::{Base64UrlUnpadded, Encoding};
use serde::{Deserialize, Serialize};
use std::error::Error;
use openssl::{
    sign::{Signer, Verifier},
    rsa::{Padding},
    pkey::PKey,
    hash::MessageDigest,
};
// use serde_json::{json, Value};

/// In this project, all the bytes should be represented in Vec<u8>,
/// so the &Bytes will be &[u8], similar to the relation between
/// String and &str.
type Bytes = Vec<u8>;

/// The idea of this enum is from serde_json::Value
/// (<https://docs.rs/serde_json/latest/serde_json/enum.Value.html>). This
/// simply enables a bit of dynamic typing for a recursive calculation
/// which preparing the transaction data to be ready to sign.
#[derive(Debug, Clone)]
pub enum TxValue {
    Bytes(Bytes),
    Vec(Vec<TxValue>),
}

/// Tag struct is the item used in the Transaction field "tags".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

/// This is a data structure which only contains the neccessary
/// transaction data to be serialized to json, and then later posted
/// on the Arweave network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    format: u32,
    id: String,
    last_tx: String,
    owner: String,
    tags: Vec<Tag>,
    target: String,
    quantity: String,
    data: String,
    data_root: String,
    data_size: String,
    reward: String,
    signature: String,
}

/// A struct and its implementations to handle most of the transaction
/// activities, such as creating, signing, and sending a transaction.
#[derive(Debug, Clone)]
pub struct Transaction {
    arwallet: ArWallet,
    tx_data: TransactionData,
    chunks: Chunks,
}

impl Transaction {

    /// Create a new AR transfer transaction by importing an Arweave
    /// wallet. The first argument is the address of the receiver, and
    /// the second is the quantity of the AR you want to send.
    pub async fn new_ar(arwallet: &ArWallet, target: &str, quantity_ar: f64,)
                        -> Self {
        
        let quantity_winston = quantity_ar * 1000000000000.0;
        let quantity_winston = quantity_winston as u64;
        let quantity = quantity_winston.to_string();
        let tags: Vec<Tag>
            = Vec::new();
        let reward = arwallet.ar_transaction_price(target)
            .await
            .unwrap();
        let tx_data = TransactionData {
            format: 2,
            id: "".to_string(),
            last_tx: arwallet.tx_anchor().await.unwrap(),
            owner: arwallet.owner(),
            tags,
            target: target.to_string(),
            quantity,
            data: "".to_string(),
            data_root: "".to_string(),
            data_size: "0".to_string(),
            reward,
            signature: "".to_string(),
        };
        Self {
            arwallet: arwallet.clone(),
            tx_data,
            chunks: Chunks::new(),
        }
    }
    /// Create a new data transaction by using a given Arweave
    /// wallet.
    pub async fn new_data(arwallet: &ArWallet, raw_data: &[u8]) -> Self {
        let mut tx = Self::new_ar(arwallet, "", 0.0).await.clone();
        let mut chunks = Chunks::new();
        chunks.finalize(&raw_data);
        tx.chunks = chunks.clone();
        tx.tx_data.data_root = b64_encode(&tx.chunks.data_root());
        tx.tx_data.data_size = tx.chunks.data_size().to_string();
        tx.tx_data.reward =arwallet.data_transaction_price(
            tx.chunks.data_size()).await.unwrap();
        tx.tx_data.last_tx = arwallet.tx_anchor().await.unwrap();
        tx
    }

    /// Create a new combined AR transfer **and** Data transaction by
    /// using a given Arweave wallet.
    pub async fn new_ar_data(
        arwallet: &ArWallet,
        target: &str,
        quantity_ar: f64,
        raw_data: &[u8]) -> Self {
        let mut tx = Self::new_ar(arwallet, target, quantity_ar).await.clone();
        let mut chunks = Chunks::new();
        chunks.finalize(&raw_data);
        tx.chunks = chunks.clone();
        tx.tx_data.data_root = b64_encode(&tx.chunks.data_root());
        tx.tx_data.data_size = tx.chunks.data_size().to_string();
        tx.tx_data.reward =arwallet.ar_data_transaction_price(
            &target,
            tx.chunks.data_size(),
        ).await.unwrap();
        tx.tx_data.last_tx = arwallet.tx_anchor().await.unwrap();
        tx
    }

    /// Generate the data to be signed. It puts very item into a
    /// `TxValue::Vec`, then it will run a custom recursive Sha384
    /// hash calculation, and return the result. This recursive
    /// calculation is a mendatory requirement in the Arweave
    /// transaction V2.
    pub fn sign_data_root(&self) -> Bytes {
        let tx_data = &self.tx_data;
        let format_b = tx_data.format.to_string().into_bytes();
        let format_b = TxValue::Bytes(format_b);
        let owner_b = Base64UrlUnpadded::decode_vec(&tx_data.owner).unwrap();
        let owner_b = TxValue::Bytes(owner_b);
        let target_b = Base64UrlUnpadded::decode_vec(&tx_data.target).unwrap();
        let target_b = TxValue::Bytes(target_b);
        let data_root_b = Base64UrlUnpadded::decode_vec(&tx_data.data_root)
            .unwrap();
        let data_root_b = TxValue::Bytes(data_root_b);
        let data_size_b = tx_data.data_size.clone().into_bytes();
        let data_size_b = TxValue::Bytes(data_size_b);
        let quantity_b = tx_data.quantity.clone().into_bytes();
        let quantity_b = TxValue::Bytes(quantity_b);
        let reward_b = tx_data.reward.clone().into_bytes();
        let reward_b = TxValue::Bytes(reward_b);
        let last_tx_b = Base64UrlUnpadded::decode_vec(&tx_data.last_tx)
            .unwrap();
        let last_tx_b = TxValue::Bytes(last_tx_b);
        let mut tags_b_vec: TxValue = TxValue::Vec(Vec::new());
        let mut i = 0;
        while i < tx_data.tags.len() {
            let name = tx_data.tags[i].name.clone();
            let value = tx_data.tags[i].value.clone();
            let name_b = TxValue::Bytes(
                Base64UrlUnpadded::decode_vec(&name).unwrap());
            let value_b = TxValue::Bytes(
                Base64UrlUnpadded::decode_vec(&value).unwrap());
            let mut tag_b_vec: TxValue = TxValue::Vec(Vec::new());
            if let TxValue::Vec(ref mut real_vec) = tag_b_vec {
                real_vec.push(name_b);
                real_vec.push(value_b);
            }
            if let TxValue::Vec(ref mut real_vec) = tags_b_vec {
                real_vec.push(tag_b_vec);
            }
            i = i + 1;
        }
        let sign_data: TxValue = TxValue::Vec(
            vec![
                format_b,
                owner_b,
                target_b,
                quantity_b,
                reward_b,
                last_tx_b,
                tags_b_vec,
                data_size_b,
                data_root_b,
            ]
        );
        recursive_hash(&sign_data)
    }

    /// Sign the current transaction. Signature will be saved in the
    /// signature field. It will also generate the transaction id.
    pub fn sign(&mut self) {
        let sign_data_root = self.sign_data_root();
        // println!("{:?}", sign_data_root);
        let private_key = self.arwallet.private_key();
        let signing_key = PKey::from_rsa(private_key).unwrap();
        let mut signer = Signer::new(MessageDigest::sha256(), &signing_key)
            .unwrap();
        signer.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
        // signer.set_rsa_pss_saltlen(RsaPssSaltlen::custom(32)).unwrap();
        signer.update(&sign_data_root).unwrap();
        let signature = signer.sign_to_vec().unwrap();
        let signature_b64 = Base64UrlUnpadded
            ::encode_string(&signature);
        self.tx_data.signature = signature_b64;
        let tx_id_b = sha256(&signature);
        self.tx_data.id = Base64UrlUnpadded
            ::encode_string(&tx_id_b);
    }

    /// Verify the transaction signature. If the transaction cannot be
    /// signed correctly, it will reture an error message, and this
    /// transaction cannot be submitted.
    pub fn verify(&self) -> Result<String, String> {
        let sign_data_root = self.sign_data_root();
        let private_key = self.arwallet.private_key();
        let signing_key = PKey::from_rsa(private_key).unwrap();
        let mut verifier = Verifier::new(MessageDigest::sha256(),
                                         &signing_key)
            .unwrap();
        verifier.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
        // verifier.set_rsa_pss_saltlen(RsaPssSaltlen::custom(32)).unwrap();
        verifier.update(&sign_data_root).unwrap();
        let signature = Base64UrlUnpadded::decode_vec(&self.tx_data.signature)
            .unwrap();
        let verify_result = verifier.verify(&signature).unwrap();
        if verify_result {
            Ok("The Transaction is verified successfully."
               .to_string())
        } else {
            Err("Failed to verify".to_string())
        }
    }

    /// Submit the current transaction. You have to make sure that
    /// this transaction has been signed. 
    pub async fn submit(&self) -> Result<String, Box<dyn Error>> {
        let verified: bool;
        match self.verify() {
            Ok(_) => verified = true,
            Err(_) => verified = false,
        }
        if verified {
            let tx_data = &self.tx_data;
            let tx_data_json = serde_json::to_string_pretty(tx_data).unwrap();
            let mut api_url = self.arwallet.gateway();
            api_url.push_str("tx");
            let res = self.arwallet.http_client().post(&api_url)
                .body(tx_data_json).send().await;
            
            match res {
                Ok(r) => {
                    let res_status = r.status().as_u16();
                    if res_status == 200 {
                        Ok("Transaction has been submitted successfully."
                           .to_string())
                    } else {
                        let res_text = r.text().await.unwrap();
                        let mut err_string = "Error: ".to_string();
                        err_string.push_str(&res_status.to_string());
                        err_string.push_str(" - ");
                        err_string.push_str(&res_text);
                        Err(err_string.into())
                    }
                },
                Err(e) => Err(e.into()),
            }
        } else {
            Err("Transaction failed to sign can't be submitted"
                .to_string().into())
        }
    }

    /// Get the Arweave standard transaction data. 
    pub fn tx_data(&self) -> TransactionData {
        self.tx_data.clone()
    }

    /// Get the Arweave standard transaction data in json format.
    pub fn tx_data_json(&self) -> String {
        serde_json::to_string_pretty(&self.tx_data()).unwrap()
    }

    /// Add a tag to the transaction.
    pub fn add_tag(&mut self, name: &str, value: &str) {
        let name = b64_encode(name.as_bytes());
        let value = b64_encode(value.as_bytes());
        let tag = Tag {
            name: name,
            value: value,
        };
        self.tx_data.tags.push(tag);
    }

    /// Get the chunks instance.
    pub fn chunks(&self) -> Chunks {
        self.chunks.clone()
    }
}
