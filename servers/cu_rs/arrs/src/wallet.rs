use openssl::{rsa::Rsa,
              bn::BigNum,
              pkey::{Private,},
};
// use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use base64ct::{Base64UrlUnpadded, Encoding};
use reqwest;
use indexmap::IndexMap;
use std::error::Error;
use crate::{
    transaction::Transaction,
    b64::{b64_encode, b64_decode},
    hasher::sha256,
    uploader::Uploader,
    downloader::Downloader,
    public_client::ArPublic,
};

/// The main struct that contains your Arweave wallet, and its methods
/// and fuctions will cover most features of Arweave.
#[derive(Debug, Clone)]
pub struct ArWallet {
    ar_public: ArPublic,
    priv_key: Rsa<Private>,
    key_data: IndexMap<String, Value>,
    uploader: Uploader,
}

impl ArWallet {
    /// Initiate a new Arweave wallet defautly. It generates a new
    /// wallet, and use the default <https://arweave.net/> as gateway.
    pub fn new() -> Self {
        let ar_public = ArPublic::new();
        let bits = 4096;
        let priv_key = Rsa::generate(bits).unwrap();
        let p = priv_key.p().unwrap();
        let q = priv_key.q().unwrap();
        let d = priv_key.d();
        let e = priv_key.e();
        let qi = priv_key.iqmp().unwrap();
        let dp = priv_key.dmp1().unwrap();
        let dq = priv_key.dmq1().unwrap();
        let n = priv_key.n();
        let p_jsb64 = json!(b64_encode(&p.to_vec()));
        let q_jsb64 = json!(b64_encode(&q.to_vec()));
        let d_jsb64 = json!(b64_encode(&d.to_vec()));
        let e_jsb64 = json!(b64_encode(&e.to_vec()));
        let qi_jsb64 = json!(b64_encode(&qi.to_vec()));
        let dp_jsb64 = json!(b64_encode(&dp.to_vec()));
        let dq_jsb64 = json!(b64_encode(&dq.to_vec()));
        let n_jsb64 = json!(b64_encode(&n.to_vec()));

        let kty = json!("RSA");
        let key_data: IndexMap<String, Value> = IndexMap::from([
            (String::from("p"), p_jsb64),
            (String::from("kty"), kty),
            (String::from("q"), q_jsb64),
            (String::from("d"), d_jsb64),
            (String::from("e"), e_jsb64),
            (String::from("qi"), qi_jsb64),
            (String::from("dp"), dp_jsb64),
            (String::from("dq"), dq_jsb64),
            (String::from("n"), n_jsb64),
        ]);
        let uploader = Uploader::default();
        Self {
            ar_public,
            priv_key,
            key_data,
            uploader,
        }
    }
    
    /// Initiate a new Arweave wallet by importing a jwk file. It
    /// generates a new wallet, and use the default
    /// <https://arweave.net/> as gateway.
    pub fn from_jwk(jwk_string: &str) -> Self {
        let ar_public = ArPublic::new();
        let key_data: IndexMap<String, Value>
            = serde_json::from_str(&jwk_string).unwrap();
        let priv_key = Self::generate_key_from_key_data(key_data.clone());
        let uploader = Uploader::default();
        Self {
            ar_public,
            priv_key,
            key_data,
            uploader,
        }
    }

    /// Convert an already deserialized jwk data to the type
    /// `Rsa<Private>`.
    pub fn generate_key_from_key_data (kd: IndexMap<String, Value>)
                                       -> Rsa<Private> {
        let p = BigNum::from_slice(&b64_decode(kd["p"].as_str()
                                              .unwrap())
                                   .unwrap()).unwrap();
        let q = BigNum::from_slice(&b64_decode(kd["q"].as_str()
                                              .unwrap())
                                   .unwrap()).unwrap();
        let e = BigNum::from_slice(&b64_decode(kd["e"].as_str()
                                              .unwrap())
                                   .unwrap()).unwrap();
        let n = BigNum::from_slice(&b64_decode(kd["n"].as_str()
                                              .unwrap())
                                   .unwrap()).unwrap();
        let d = BigNum::from_slice(&b64_decode(kd["d"].as_str()
                                              .unwrap())
                                   .unwrap()).unwrap();
        let qi = BigNum::from_slice(&b64_decode(kd["qi"].as_str()
                                              .unwrap())
                                   .unwrap()).unwrap();
        let dp = BigNum::from_slice(&b64_decode(kd["dp"].as_str()
                                              .unwrap())
                                   .unwrap()).unwrap();
        let dq = BigNum::from_slice(&b64_decode(kd["dq"].as_str()
                                              .unwrap())
                                   .unwrap()).unwrap();
        let new_key = Rsa::from_private_components(
            n, e, d, p, q, dp, dq, qi
        ).unwrap();
        
        new_key
    }
        
    /// Import a jwk. Running this method will instantly replace the
    /// original key from the wallet. Should consider export and have
    /// a backup of your original key before doing it. Otherwise, you
    /// may permanently lose your key and all the AR crypto you have.
    pub fn import_jwk (&mut self, jwk_str: &str) {
        let kd: Value = serde_json::from_str(jwk_str).unwrap();
        println!("{:?}", kd);
        // self.key_data = kd.clone();
        // let new_key = Self::generate_key_from_key_data(kd);
        // self.priv_key = new_key;
    }

    /// Export the wallet as jwk format, return as a Rust String
    /// type. This is how we backup and save our Arweave wallet to
    /// somewhere else. You can later send this String to a server or
    /// save it as a local json file.
    pub fn export_jwk(&self) -> String {
        let kd: IndexMap<String, Value> = self.key_data.clone();
        let jwk_string = serde_json::to_string_pretty(&kd).unwrap();
        jwk_string
    }

    /// Get the reference of the arweave public client.
    pub fn ar_public(&self) -> &ArPublic {
        &self.ar_public
    }

    /// Get the current gateway.
    pub fn gateway(&self) -> String {
        self.ar_public.gateway()
    }
    
    /// Change Arweave gateway. The format should look like
    /// `https://arweave.net/`.
    pub fn set_gateway(&mut self, address: &str) {
        self.ar_public.set_gateway(address);
    }

    /// Get the current http client.
    pub fn http_client(&self) -> &reqwest::Client {
        &self.ar_public.http_client()
    }
    
    /// Return the Arweave address (id) of the wallet.
    pub fn address(&self) -> String {
        let address_bytes = sha256(&self.priv_key.n().to_vec());
        let address = Base64UrlUnpadded::encode_string(&address_bytes);
        address
    }

    /// Get the current private key. do not share this key to
    /// other. This method is more or less for internal use. For
    /// personal key backup, you probably want to try `export_jwk()`.
    pub fn private_key(&self) -> Rsa<Private> {
        self.priv_key.clone()
    }

    /// Return the owner of the Arweave wallet.
    pub fn owner(&self) -> String {
        self.key_data["n"].as_str().unwrap().to_string()
    }

    /// Check your AR crypto balance in your wallet.The HTTP API will
    /// return all amounts as Winston strings. Winston is the smallest
    /// possible unit of AR. 1 AR = 1000000000000 Winston (12 zeros)
    /// and 1 Winston = 0.000000000001 AR.
    pub async fn balance(&self) -> Result<String, Box<dyn Error>> {
        let address = self.address();
        let mut api_url: String = self.ar_public.gateway();
        api_url.push_str("wallet/");
        api_url.push_str(&address);
        api_url.push_str("/balance");
        let res =self.ar_public.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get the last transaction id of this wallet.
    pub async fn last_tx(&self) -> Result<String, Box<dyn Error>> {
        let address = self.address();
        let mut api_url: String = self.ar_public.gateway();
        api_url.push_str("wallet/");
        api_url.push_str(&address);
        api_url.push_str("/last_tx");
        let res =self.ar_public.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    pub async fn tx_anchor(&self) -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.ar_public.gateway();
        api_url.push_str("tx_anchor");
        let res =self.ar_public.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get a price of an AR transfer transaction. The first argument
    /// is the address of the receiver. Return a Winston string.
    pub async fn ar_transaction_price(&self, target: &str)
                                      -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.ar_public.gateway();
        api_url.push_str("price/0/");
        api_url.push_str(target);
        let res =self.ar_public.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get a price of a data transaction. The first argument is the
    /// data size. Return a Winston string.
    pub async fn data_transaction_price(&self, size: usize)
                                        -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.ar_public.gateway();
        api_url.push_str("price/");
        api_url.push_str(&size.to_string());
        let res =self.ar_public.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get a price of a combined AR transfer and data
    /// transaction. The first argument is the address of the
    /// receiver. The second argument is the data size. Return a
    /// Winston string.
    pub async fn ar_data_transaction_price(&self, target: &str, size: usize)
                                           -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.ar_public.gateway();
        api_url.push_str("price/");
        api_url.push_str(&size.to_string());
        api_url.push_str("/");
        api_url.push_str(&target);
        let res =self.ar_public.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Create a new AR transfer transaction by using the current
    /// Arweave wallet.
    pub async fn create_ar_transaction(&self, target: &str,  quantity_ar: f64,)
                                -> Transaction {
       Transaction::new_ar(&self, target, quantity_ar).await
    }

    /// Create a new Data transaction by using the current Arweave
    /// wallet.
    pub async fn create_data_transaction(&mut self, raw_data: &[u8])
                                         -> Transaction {
        self.uploader = Uploader::new(&raw_data);
        Transaction::new_data(&self, &raw_data).await
    }

    /// Create a new combined Ar transfer **and ** Data transaction by using
    /// the current Arweave wallet.
    pub async fn create_ar_data_transaction(
        &mut self,
        target: &str,
        quantity_ar: f64,
        raw_data: &[u8]
    ) -> Transaction {
        self.uploader = Uploader::new(&raw_data);
        Transaction::new_ar_data(&self, &target, quantity_ar, &raw_data).await
    }

    /// Get the current resumable uploader. You will get an empty
    /// uploader and it will do nothing if you have not create a data
    /// transaction yet. However, you don't have to create a data
    /// transaction if you just want to resume a previou uploading.
    pub fn uploader(&self) -> Uploader {
        self.uploader.clone()
    }

    /// Upload a file. Make sure you create, sign, and submit a data
    /// transaction first before doing upload. You can also resume a
    /// previous upload, in such case, a data transaction is not
    /// needed.
    pub async fn upload(&mut self)
                        -> Result<String, Box<dyn Error>> {
        let result = self.uploader.upload(&self.clone()).await;
        match result {
            Ok(s) => Ok(s),
            Err(e) => Err(e),
        }
    }

    /// Get a transaction offset and size. This method is useful for
    /// creating a resumable chunks downloader.
    pub async fn transaction_offset_size(&self, id: &str)
                                         -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.ar_public.gateway();
        api_url.push_str("tx/");
        api_url.push_str(id);
        api_url.push_str("/offset");
        let res =self.ar_public.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get a single chunk of a file by inputting one of its
    /// offset. This method is useful for creating a resumable chunks
    /// downloader.
    pub async fn chunk(&self, offset: &str)
                       -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.ar_public.gateway();
        api_url.push_str("chunk/");
        api_url.push_str(offset);
        let res =self.ar_public.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get the reference of the downloader. You have to setup a new
    /// download or resume a previous download before it can do
    /// anything.
    pub fn downloader(&self) -> &Downloader {
        &self.ar_public.downloader()
    }

    /// Setup a new download by inputting a data transaction id. This
    /// can be anyone's data transaction. It is not neccessary to
    /// belong the current wallet.
    pub async fn new_download(&mut self, id: &str)
                              -> Result<(), Box<dyn Error>> {
        self.ar_public.new_download(id).await?;
        Ok(())
    }

    /// Download the current chunk based on the information of the
    /// Downloader. You have to setup a new download or resume a
    /// previous download before it can actually download anything.
    pub async fn download(&mut self)
                          -> Result<String, Box<dyn Error>> {
        Ok(self.ar_public.download().await?)
    }
}
