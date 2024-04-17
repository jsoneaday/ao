use bytes::{BufMut, Bytes};
use bundlr_sdk::{error::BundlrError, tags::{AvroDecode, Tag}};
use sha2::{Digest, Sha256, Sha384};
use base64_url;
use ring::rand::SecureRandom;

#[derive(Debug)]
pub enum ByteErrorType {
    ByteError(String)
}

impl From<BundlrError> for ByteErrorType {
    fn from(error: BundlrError) -> Self {
        ByteErrorType::ByteError(format!("Byte error: {}", error))
    }
}

impl From<&str> for ByteErrorType {
    fn from(error: &str) -> Self {
        ByteErrorType::ByteError(format!("Byte error: {}", error))
    }
}

#[derive(Clone)]
pub struct DataBundle {
    pub items: Vec<DataItem>,
    pub tags: Vec<Tag>
}

impl DataBundle {
    pub fn new(tags: Vec<u8>) -> Self {
        DataBundle { items: Vec::new(), tags }
    }

    pub fn add_item(&mut self, item: DataItem) {
        self.items.push(item);
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ByteErrorType> {
        let mut headers = vec![u8; 64 * self.items.len()];
        let mut binaries = Vec::new();

        for (index, item) in self.items.iter().enumerate() {
            let id = item.raw_id();

            let mut header = Vec::with_capacity(64);
            header.extend_from_slice(&long_to_32_byte_array(item.as_bytes()?.len() as u64)?);
            header.extend_from_slice(&id);

            headers.splice(64 * index..64 * (index + 1), header.iter().cloned());
            binaries.extend_from_slice(&item.as_bytes()?);
        }

        let mut buffer = Vec::new();
        buffer.extend_from_slice(&long_to_32_byte_array(self.items.len() as u64)?);
        buffer.extend_from_slice(&headers);
        buffer.extend_from_slice(&binaries);

        Ok(buffer)
    }
}

fn long_to_n_byte_array(n: usize, long: u64) -> Result<Vec<u8>, ByteErrorType> {
    let mut byte_array = vec![0u8; n];
    let mut value = long;

    for index in 0..n {
        let byte = (value & 0xFF) as u8;
        byte_array[index] = byte;
        value >>= 8;
    }

    Ok(byte_array)
}

fn long_to_32_byte_array(value: u64) -> Result<Vec<u8>, ByteErrorType> {
    long_to_n_byte_array(32, value);
}

enum Data {
    None,
    Bytes(Vec<u8>)
}

#[derive(Clone)]
pub struct DataItem {
    signature_type: SignerMap,
    pub signature: Vec<u8>,
    owner: Vec<u8>,
    target: Vec<u8>,
    anchor: Vec<u8>,
    tags: Vec<u8>,
    data: Data
}

#[derive(Clone)]
pub struct Config {
    pub sig_length: usize,
    pub pub_length: usize,
    pub sig_name: String
}

#[derive(PartialEq, Clone)]
pub enum SignerMap {
    None = -1,
    Arweave = 1,
}

impl SignerMap {
    pub fn get_config(&self) -> Config {
        Config {
            sig_length: 512,
            pub_length: 512,
            sig_name: "arweave".to_string()
        }
    }

    pub fn as_u16(&self) -> u16 {
        match self {
            SignerMap::Arweave => 1,
            _ => u16::MAX,
        }
    }
}

impl From<u16> for SignerMap {
    fn from(t: u16) -> Self {
        match t {
            1 => SignerMap::Arweave,
            _ => SignerMap::None,
        }
    }
}

pub const LIST_AS_BUFFER: &[u8] = "list".as_bytes();
pub const BLOB_AS_BUFFER: &[u8] = "blob".as_bytes();
pub const DATAITEM_AS_BUFFER: &[u8] = "dataitem".as_bytes();
pub const ONE_AS_BUFFER: &[u8] = "1".as_bytes();

pub enum DeepHashChunk {
    Chunk(Bytes),
    Chunks(Vec<DeepHashChunk>),
}

pub fn deep_hash_sync(chunk: DeepHashChunk) -> Result<Bytes, ByteErrorType> {
    match chunk {
        DeepHashChunk::Chunk(b) => {
            let tag = [BLOB_AS_BUFFER, b.len().to_string().as_bytes()].concat();
            let c = [sha384hash(tag.into()), sha384hash(b)].concat();
            Ok(Bytes::copy_from_slice(&sha384hash(c.into())))
        }
        DeepHashChunk::Chunks(chunks) => {
            let len = chunks.len() as f64;
            let tag = [LIST_AS_BUFFER, len.to_string().as_bytes()].concat();
            let acc = sha384hash(tag.into());
            deep_hash_chunks_sync(chunks, acc)
        }
    }
}

pub fn deep_hash_chunks_sync(
    mut chunks: Vec<DeepHashChunk>,
    acc: Bytes,
) -> Result<Bytes, ByteErrorType> {
    if chunks.is_empty() {
        return Ok(acc);
    };
    let acc = Bytes::copy_from_slice(&acc);
    let hash_pair = [acc, deep_hash_sync(chunks.remove(0))?].concat();
    let new_acc = sha384hash(hash_pair.into());
    deep_hash_chunks_sync(chunks, new_acc)
}

fn sha384hash(b: Bytes) -> Bytes {
    let mut hasher = Sha384::new();
    hasher.update(&b);
    Bytes::copy_from_slice(&hasher.finalize())
}

impl DataItem {
    pub fn new(target: Vec<u8>, data: Vec<u8>, tags: Vec<u8>, owner: Vec<u8>) -> Result<Self, ByteErrorType> {
        let mut randoms: [u8; 32] = [0; 32];
        let sr = ring::rand::SystemRandom::new();
        match sr.fill(&mut randoms) {
            Ok(()) => (),
            Err(e) => return Err(ByteErrorType::ByteError(e))
        }
        let anchor = randoms.to_vec();

        Ok(
            DataItem {
                signature_type: SignerMap::Arweave,
                signature: vec![],
                owner,
                target,
                anchor,
                tags,
                data: Data::Bytes(data)
            }
        )
    }

    pub fn get_message(&mut self) -> Result<Bytes, ByteErrorType> {
        let encoded_tags = if !self.tags.is_empty() {
            self.tags.encode()?
        } else {
            Bytes::deffault()
        };

        match &mut self.data {
            Data::None => Ok(Bytes::new()),
            Data::Bytes(data) => {
                let data_chunk = DeepHashChunk::Chunk(data.clone().into());
                let sig_type = &self.signature_type;
                let sig_type_bytes = sig_type.as_u16().to_string().as_bytes().to_vec();
                deep_hash_sync(DeepHashChunk::Chunks(vec![
                    DeepHashChunk::Chunk(DATAITEM_AS_BUFFER.into()),
                    DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
                    DeepHashChunk::Chunk(sig_type_bytes.to_vec().into()),
                    DeepHashChunk::Chunk(self.owner.to_vec().into()),
                    DeepHashChunk::Chunk(self.target.to_vec().into()),
                    DeepHashChunk::Chunk(self.anchor.to_vec().into()),
                    DeepHashChunk::Chunk(encoded_tags.clone())
                ]))
            }
        }
    }

    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty() && self.signature_type != SignerMap::None
    }

    fn from_info_bytes(buffer: &[u8]) -> Result<(Self, usize), ByteErrorType> {
        if buffer.len() < 2 {
            return Err(ByteErrorType::ByteError("Buffer too short for signature type".to_string()));
        }
        let sig_type_b = &buffer[0..2];
        let signature_type = u16::from_le_bytes(
            <[u8; 2]>::try_from(sig_type_b).map_err(|err| ByteErrorType::ByteError(err.to_string()))?
        );
        let signer = SignerMap::from(signature_type);

        let Config {
            pub_length,
            sig_length,
            ..
        } = signer.get_config();

        if buffer.len() < 2 + sig_length + pub_length {
            return Err(ByteErrorType::ByteError("Buffer too short for signature and public key".to_string()));
        }

        let signature = &buffer[2..2 + sig_length];
        let owner = &buffer[2 + sig_length..2 + sig_length + pub_length];

        let target_start = 2 + sig_length + pub_length;
        let target_present = u8::from_le_bytes(
            <[u8;1]>::try_from(&buffer[target_start..target_start + 1])
                .map_err(|err| ByteErrorType::ByteError(format!("target bytes error - {}", err.to_string())))?
        );
        let target = match target_present {
            0 => &[],
            1 => &buffer[target_start + 1..target_start + 33],
            _b => return Err(ByteErrorType::ByteError("target bytes error".to_string()))
        };

        let anchor_start = target_start + 1 + target.len();
        let anchor_present = u8::from_le_bytes(
            <[u8; 1]>::try_from(&buffer[anchor_start..anchor_start + 1])
                .map_err(|err| ByteErrorType::ByteError(format!("anchor bytes error - {}", err.to_string())))   
        );
        let anchor = match anchor_present {
            0 => &[],
            1 => &buffer[anchor_start + 1..anchor_start + 33],
            b => return Err(ByteErrorType::ByteError(format!("anchor bytes error - {}", b.to_string())))
        };

        let tags_start = anchor_start + 1 + anchor.len();
        let number_of_tags = u64::from_le_bytes(
            <[u8;8]>::try_from(&buffer[tags_start..tags_start + 8])
                .map_err(|err| ByteErrorType::ByteError(format!("tag bytes error - {}", err.to_string())))   
        );

        let number_of_tags_bytes = u64::from_le_bytes(
            <[u8;8]>::try_from(&buffer[tags_start + 8..tags_start + 16])
                .map_err(|err| ByteErrorType::ByteError(format!("tag bytes error - {}", err.to_string())))   
        );

        let mut b = buffer.to_vec();
        let mut tags_bytes = &mut b[tags_start + 16..tags_start + 16 + number_of_tags_bytes as usize];

        let tags = if number_of_tags_bytes > 0 {
            tags_bytes.decode()?
        } else {
            vec![]
        };

        if number_of_tags != tags.len() as u64 {
            return Err(ByteErrorType::ByteError("invalid tag encoding".to_string()));
        }

        let data_item = DataItem {
            signature_type: signer,
            signature: signature.to_vec(),
            owner: owner.to_vec(),
            target: target.to_vec(),
            anchor: anchor.to_vec(),
            tags,
            data: Data::None
        };

        Ok((data_item, tags_start + 16 + number_of_tags_bytes as usize))
    }
}