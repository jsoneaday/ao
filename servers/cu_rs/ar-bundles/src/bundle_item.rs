use async_trait::async_trait;
use crate::signing::signer::SignerMaker;
use crate::tags::Tag;

pub enum ResolvesTo<T> {
    Item(T),
    // Future(Pin<Box<dyn Future<Output = T>>>),
    // FutureFn(Box<dyn Fn(Vec<Box<dyn Any>>) -> Pin<Box<dyn Future<Output = T>>>>)
}

impl<T> AsRef<T> for ResolvesTo<T> {
    fn as_ref(&self) -> &T {
        match self {
            ResolvesTo::Item(val) => &val,
        }
    }
}

impl<T> AsMut<T> for ResolvesTo<T> {
    fn as_mut(&mut self) -> &mut T {
        match self {
            ResolvesTo::Item(ref mut val) => val,
        }
    }
}

impl ToString for ResolvesTo<i64> {
    fn to_string(&self) -> String {
        match self {
            ResolvesTo::Item(val) => format!("{}", val),
        }
    }
}

pub struct BundleItem {
  pub signature_type: ResolvesTo<i64>,
  pub raw_signature: ResolvesTo<Vec<u8>>,
  pub signature: ResolvesTo<String>,
  pub signature_length: ResolvesTo<i64>,
  pub raw_owner: ResolvesTo<Vec<u8>>,
  pub owner: ResolvesTo<String>,
  pub owner_length: ResolvesTo<i64>,
  pub raw_target: ResolvesTo<Vec<u8>>,
  pub target: ResolvesTo<String>,
  pub raw_anchor: ResolvesTo<Vec<u8>>,
  pub anchor: ResolvesTo<String>,
  pub raw_tags: ResolvesTo<Vec<u8>>,
  pub tags: ResolvesTo<Vec<Tag>>,
  pub raw_data: ResolvesTo<Vec<u8>>,
  pub data: ResolvesTo<String>,
  pub keypair_path: ResolvesTo<String>
}

#[async_trait]
pub trait BundleItemFn {
    fn sign<T: SignerMaker>(&mut self, signer: &T) -> [u8; 32];

    fn is_valid<T: SignerMaker>(&self, signer: &T) -> bool;

    fn verify<T: SignerMaker>(&self, _args: &Vec<u8>, _signer: &T) -> bool {
        unimplemented!("You must implement `verify`")
    }
}