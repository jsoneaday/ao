use std::sync::Arc;
use bundlr_sdk::tags::Tag;
use super::bytes::{ByteErrorType, DataBundle, DataItem};
use super::dal::{Gateway, Log, Signer, TxStatus};
use super::json::Process;

pub struct Builder<'a> {
    gateway: Arc<dyn Gateway>,
    signer: Arc<dyn Signer>,
    logger: &'a Arc<dyn Log>
}

