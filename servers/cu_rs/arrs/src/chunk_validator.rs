use std::error::Error;
use primitive_types::U256;

use crate::{
    hasher::sha256,
};

pub const NOTE_SIZE: usize = 32;
pub const HASH_SIZE: usize = 32;

/// A custom function that converts big endian bytes to U256.
pub fn be_bytes_to_u256(buffer: &[u8]) -> U256 {
    let mut value: U256 = U256::zero();
    let mut i = 0;
    while i < buffer.len() {
        value = value * 256;
        value = value + buffer[i];
        i = i + 1;
    }
    value
}

#[derive(Debug, Clone, Default)]
pub struct Validator {
    _offset: U256,
    _left_bound: U256,
    _right_bound: U256,
    _chunk_size: U256,
}

impl Validator {
    pub fn validate(
        id: Vec<u8>,
        dest: U256,
        left_bound: U256,
        right_bound: U256,
        path: Vec<u8>
    )
        -> Result<Self, Box<dyn Error>> {
        if right_bound <= U256::zero() {
            return Err(
                "Validation failed. Data size cannot be less or equal to 0. "
                    .into());
        }

        if dest >= right_bound {
            return
                Self::validate(
                    id, U256::zero(), right_bound - 1, right_bound, path);
        }

        if path.len() == HASH_SIZE + NOTE_SIZE {
            let path_data = path[0..HASH_SIZE].to_vec();
            let end_offset_buffer = path[
                path_data.len()..path_data.len() + NOTE_SIZE].to_vec();
            let hash_path_data = sha256(&path_data);
            let hash_end_offset_buf = sha256(&end_offset_buffer);
            let mut combined_hash = Vec::new();
            combined_hash.extend(&hash_path_data);
            combined_hash.extend(&hash_end_offset_buf);
            let path_data_hash = sha256(&combined_hash);
            if path_data_hash == id {
                return Ok(
                    Self {
                        _offset: right_bound - 1,
                        _left_bound: left_bound,
                        _right_bound: right_bound,
                        _chunk_size: right_bound - left_bound,
                    }
                );
            }
            return Err("Failed to validate this chunk".into());
        }
        let left = path[0..HASH_SIZE].to_vec();
        let right = path[left.len()..left.len() + HASH_SIZE].to_vec();
        let offset_buffer = path[
            left.len() + right.len()..left.len() + right.len() + NOTE_SIZE
        ].to_vec();
        let offset = be_bytes_to_u256(&offset_buffer);
        let remainder = path[left.len() + right.len() + offset_buffer.len()..]
            .to_vec();
        let mut combine_to_hash = Vec::new();
        combine_to_hash.extend(&sha256(&left));
        combine_to_hash.extend(&sha256(&right));
        combine_to_hash.extend(&sha256(&offset_buffer));
        let path_hash = sha256(&combine_to_hash);
        if path_hash == id {
            if dest < offset {
                return Self::validate(
                    left,
                    dest,
                    left_bound,
                    offset.min(right_bound),
                    remainder,
                );
            } else {
                return Self::validate(
                    right,
                    dest,
                    offset.max(left_bound),
                    right_bound,
                    remainder,
                );
            }
        }
        Err("Failed to validate this chunk".into())
    }
}
