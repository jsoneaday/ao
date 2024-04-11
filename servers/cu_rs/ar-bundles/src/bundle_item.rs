use std::future::Future;
use std::pin::Pin;
use std::any::Any;

use crate::signing::signer::Signer;
use crate::tags::Tag;

pub enum ResolvesTo<T> {
    Item(T),
    Future(Pin<Box<dyn Future<Output = T>>>),
    FutureFn(Box<dyn Fn(Vec<Box<dyn Any>>) -> Pin<Box<dyn Future<Output = T>>>>)
}

pub struct BundleItem {
  signature_type: ResolvesTo<i64>,
  raw_signature: ResolvesTo<Vec<u8>>,
  signature: ResolvesTo<String>,
  signature_length: ResolvesTo<i64>,
  raw_owner: ResolvesTo<Vec<u8>>,
  owner: ResolvesTo<String>,
  owner_length: ResolvesTo<i64>,
  raw_target: ResolvesTo<Vec<u8>>,
  target: ResolvesTo<String>,
  raw_anchor: ResolvesTo<Vec<u8>>,
  anchor: ResolvesTo<String>,
  raw_tags: ResolvesTo<Vec<u8>>,
  tags: ResolvesTo<Vec<Tag>>,
  raw_data: ResolvesTo<Vec<u8>>,
  data: ResolvesTo<String>
}

pub trait BundleItemFn {
    async fn sign(signer: Signer) -> Vec<u8>;

    async fn is_valid() -> bool;

    async fn verify(_args: Vec<Box<dyn Any>>) -> bool {
        unimplemented!("You must implement `verify`")
    }
}