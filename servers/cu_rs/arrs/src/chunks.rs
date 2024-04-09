use crate::{
    hasher::{sha256},
    b64::{b64_encode},
};

use serde::{Deserialize, Serialize};
use primitive_types::U256;

pub const MAX_CHUNK_SIZE: usize = 256 * 1024;
pub const MIN_CHUNK_SIZE: usize = 32 * 1024;
pub const NOTE_SIZE: usize = 32;

/// A struct to store the information for each data chunk item.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Chunk {
    data_hash: Vec<u8>,
    min_byte_range: usize,
    max_byte_range: usize,
    chunk: Vec<u8>
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeafNode {
    id: Vec<u8>,
    data_hash: Vec<u8>,
    _min_byte_range: usize,
    max_byte_range: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchNode {
    id: Vec<u8>,
    byte_range: usize,
    max_byte_range: usize,
    left_child: MerkleNode,
    right_child: MerkleNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MerkleNode {
    Leaf(LeafNode),
    Branch(Box<BranchNode>),
}

impl Default for MerkleNode {
    fn default() -> Self {
        let default_leaf = LeafNode::default();
        Self::Leaf(default_leaf)
    }
}

impl MerkleNode {
    pub fn is_leaf(&self) -> bool {
        match self {
            Self::Leaf(_) => {
                true
            },
            _ => {
                false
            }
        }
    }

    pub fn is_branch(&self) -> bool {
        if self.is_leaf() {
            false
        } else {
            true
        }
    }

    pub fn leaf(&self) -> LeafNode {
        match self {
            Self::Leaf(leaf_node) => {
                leaf_node.clone()
            },
            _ => {
                panic!("This is not a LeafNode")
            }
        }
    }

    pub fn branch(&self) -> BranchNode {
        match self {
            Self::Branch(box_branch_node) => {
                *box_branch_node.clone()
            },
            _ => {
                panic!("This is not a BranchNode")
            }
        }
    }
}

pub fn hash_branch(left: MerkleNode, right: Option<MerkleNode>) -> MerkleNode {
    match left {
        MerkleNode::Leaf(ref left_node) => {
            if right.is_none() {
                MerkleNode::Leaf(left_node.clone())
            } else {
                let right_node = right.clone().unwrap().clone().leaf();
                let l_id_hash = sha256(&left_node.id);
                let r_id_hash = sha256(&right_node.id);
                let max_byte_range_b = usize_to_be_bytes_256(
                    left_node.max_byte_range);
                let max_byte_range_hash = sha256(&max_byte_range_b);
                let mut id_to_hash = Vec::new();
                id_to_hash.extend(&l_id_hash);
                id_to_hash.extend(&r_id_hash);
                id_to_hash.extend(max_byte_range_hash);
                let id = sha256(&id_to_hash);
                let byte_range = left_node.max_byte_range;
                let max_byte_range = right_node.max_byte_range;
                let left_child = MerkleNode::Leaf(left_node.clone());
                let right_child = MerkleNode::Leaf(right_node.clone());
                let branch_node = BranchNode {
                    id: id,
                    byte_range: byte_range,
                    max_byte_range: max_byte_range,
                    left_child: left_child,
                    right_child: right_child,
                };
                MerkleNode::Branch(Box::new(branch_node))
            }
        },
        MerkleNode::Branch(box_left_node) => {
            let left_node: BranchNode = *box_left_node.clone();
            let branch_node: BranchNode;
            if right.is_none() {
                branch_node = left_node.clone()
            } else if right.clone().unwrap().clone().is_branch() {
                let right_merkle_node = right.clone().unwrap().clone();
                let right_node = right_merkle_node.branch();
                let l_id_hash = sha256(&left_node.id);
                let r_id_hash = sha256(&right_node.id);
                let max_byte_range_b = usize_to_be_bytes_256(
                    left_node.max_byte_range);
                let max_byte_range_hash = sha256(&max_byte_range_b);
                let mut id_to_hash = Vec::new();
                id_to_hash.extend(&l_id_hash);
                id_to_hash.extend(&r_id_hash);
                id_to_hash.extend(max_byte_range_hash);
                let id = sha256(&id_to_hash);
                let byte_range = left_node.max_byte_range;
                let max_byte_range = right_node.max_byte_range;
                let left_child = MerkleNode::Branch(
                    Box::new(left_node.clone()));
                let right_child = MerkleNode::Branch(
                    Box::new(right_node.clone()));
                branch_node = BranchNode {
                    id: id,
                    byte_range: byte_range,
                    max_byte_range: max_byte_range,
                    left_child: left_child,
                    right_child: right_child,
                };
            } else {
                let right_merkle_node = right.clone().unwrap().clone();
                let right_node = right_merkle_node.leaf();
                let l_id_hash = sha256(&left_node.id);
                let r_id_hash = sha256(&right_node.id);
                let max_byte_range_b = usize_to_be_bytes_256(
                    left_node.max_byte_range);
                let max_byte_range_hash = sha256(&max_byte_range_b);
                let mut id_to_hash = Vec::new();
                id_to_hash.extend(&l_id_hash);
                id_to_hash.extend(&r_id_hash);
                id_to_hash.extend(max_byte_range_hash);
                let id = sha256(&id_to_hash);
                let byte_range = left_node.max_byte_range;
                let max_byte_range = right_node.max_byte_range;
                let left_child = MerkleNode::Branch(
                    Box::new(left_node.clone()));
                let right_child = MerkleNode::Leaf(right_node.clone());
                branch_node = BranchNode {
                    id: id,
                    byte_range: byte_range,
                    max_byte_range: max_byte_range,
                    left_child: left_child,
                    right_child: right_child,
                };
            }
            MerkleNode::Branch(Box::new(branch_node))
        }
    }
}

pub fn build_layers(nodes: &[MerkleNode], level: usize) -> MerkleNode {
    if nodes.len() < 2 {
        nodes[0].clone()
    } else {
        let mut next_layer: Vec<MerkleNode> = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            if i + 1 == nodes.len() {
                next_layer.push(hash_branch(nodes[i].clone(), None));
            } else {
                next_layer.push(hash_branch(nodes[i].clone(),
                                            Some(nodes[i + 1].clone())));
            }
            i = i + 2;
        }
        build_layers(&next_layer, level + 1)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Proof {
    offset: usize,
    proof: Vec<u8>
}

impl Proof {
    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn proof(&self) -> Vec<u8> {
        self.proof.clone()
    }
}

// pub fn generate_proofs(root: &MerkleNode) {
//     
// }

pub fn usize_to_be_bytes_256(note: usize) -> Vec<u8> {
    let mut note = note;
    let mut buffer = Vec::new();
    let mut i = NOTE_SIZE;
    while i > 0 {
        i = i - 1;
        let byte = note % 256;
        buffer.insert(0, byte as u8);
        note = (note - byte) / 256;
    }
    buffer
}

/// A custom function to convert U256 to big endian bytes. Haven't
/// used yet. This is for the future that some data types using in the
/// proof calculation will be ported to U256, instead of usize.
pub fn u256_to_be_bytes(note: U256) -> Vec<u8> {
    let mut note = note;
    let mut buffer = Vec::new();
    let mut i = NOTE_SIZE;
    while i > 0 {
        i = i - 1;
        let byte = note % U256::from(256);
        buffer.insert(0, byte.as_u128() as u8);
        note = (note - byte) / 256;
    }
    buffer
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkJson {
    data_root: String,
    data_size: String,
    data_path: String,
    offset: String,
    chunk: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Chunks {
    data_size: usize,
    data_root: Vec<u8>,
    root_node: MerkleNode,
    chunks: Vec<Chunk>,
    leaves: Vec<MerkleNode>,
    proofs: Vec<Proof>,
}

impl Chunks {
    pub fn new() -> Self {
        Self {
            data_size: Default::default(),
            data_root: Vec::new(),
            root_node: MerkleNode::default(),
            chunks: Vec::new(),
            leaves: Vec::new(),
            proofs: Vec::new(),
        }
    }

    pub fn finalize(&mut self, raw_data: &[u8]) {
        self.chunk_data(raw_data);
        self.generate_leaves();
        self.generate_root();
        self.generate_proofs(
            &self.root_node.clone(),
            &Vec::<u8>::new(),
            0,
        );
    }

    pub fn data_root(&self) -> Vec<u8> {
        self.data_root.clone()
    }

    pub fn data_size(&self) -> usize {
        self.data_size
    }

    pub fn chunks_len(&self) -> usize {
        self.chunks.len()
    }

    pub fn proofs(&self) -> Vec<Proof> {
        self.proofs.clone()
    }

    pub fn chunks(&self) -> Vec<Chunk> {
        self.chunks.clone()
    }

    pub fn chunk_data(&mut self, raw_data: &[u8]) {
        self.data_size = raw_data.len();
        let mut rest: Vec<u8> = raw_data.to_vec();
        let mut cursor = 0;
        while rest.len() >= MAX_CHUNK_SIZE {
            let mut chunk_size = MAX_CHUNK_SIZE;
            let next_chunk_size = rest.len() - MAX_CHUNK_SIZE;
            if next_chunk_size > 0 && next_chunk_size < MIN_CHUNK_SIZE {
                chunk_size = rest.len().div_ceil(2);
            }
            let chunk = &rest[0..chunk_size];
            let data_hash = sha256(chunk);
            cursor = cursor + chunk.len();
            let min_byte_range = cursor - chunk.len();
            let max_byte_range = cursor;
            let chunk = &raw_data[min_byte_range..max_byte_range];
            self.chunks.push(
                Chunk {
                    data_hash: data_hash,
                    min_byte_range: min_byte_range,
                    max_byte_range: max_byte_range,
                    chunk: chunk.to_vec(),
                }
            );
            rest = rest[chunk_size..].to_vec();
        }
        let min_byte_range = cursor;
        let max_byte_range = cursor + rest.len();
        let chunk = &raw_data[min_byte_range..max_byte_range];
        self.chunks.push(
            Chunk {
                data_hash: sha256(&rest),
                min_byte_range: min_byte_range,
                max_byte_range: max_byte_range,
                chunk: chunk.to_vec(),
            }
        );
    }

    pub fn generate_leaves(&mut self) {
        let chunk_size = self.chunks.len();
        let mut i = 0;
        while i < chunk_size {
            let chunk = self.chunks[i].clone();
            let hash_data_hash = sha256(&chunk.data_hash);
            let hash_max_byte_range = sha256(
                &usize_to_be_bytes_256(chunk.max_byte_range));
            let mut id_to_hash = Vec::new();
            id_to_hash.extend(&hash_data_hash);
            id_to_hash.extend(&hash_max_byte_range);
            let id = sha256(&id_to_hash);
            let leaf = LeafNode {
                id: id,
                data_hash: chunk.data_hash,
                _min_byte_range: chunk.min_byte_range,
                max_byte_range: chunk.max_byte_range,
            };
            self.leaves.push(MerkleNode::Leaf(leaf));
            i = i + 1;
        }
    }

    pub fn generate_root(&mut self) {
        let root_merkle = build_layers(&self.leaves, 0);
        if root_merkle.is_branch() {
            let root_node = root_merkle.branch();
            self.root_node = MerkleNode::Branch(Box::new(root_node.clone()));
            self.data_root = root_node.id.clone();
        } else {
            let root_node = root_merkle.leaf();
            self.root_node = MerkleNode::Leaf(root_node.clone());
            self.data_root = root_node.id.clone();
        }
    }

    pub fn generate_proofs(&mut self,
                           merkle_node: &MerkleNode,
                           proof: &[u8],
                           depth: usize) {
        match merkle_node {
            MerkleNode::Leaf(node) => {
                let mut proof_vec: Vec<u8> = Vec::new();
                proof_vec.extend(proof);
                proof_vec.extend(&node.data_hash);
                proof_vec.extend(usize_to_be_bytes_256(node.max_byte_range));
                let a_proof = Proof {
                    offset: node.max_byte_range - 1,
                    proof: proof_vec.clone(),
                };
                self.proofs.insert(0, a_proof);
            },
            MerkleNode::Branch(box_node) => {
                let node = *box_node.clone();
                let mut partial_proof: Vec<u8> = Vec::new();
                
                let left_child_id: Vec<u8>;
                if node.left_child.is_leaf() {
                    left_child_id = node.left_child.leaf().id;
                } else {
                    left_child_id = node.left_child.branch().id;
                }
                
                let right_child_id: Vec<u8>;
                if node.right_child.is_leaf() {
                    right_child_id = node.right_child.leaf().id
                } else {
                    right_child_id = node.right_child.branch().id;
                }
                let byte_range = usize_to_be_bytes_256(node.byte_range);
                partial_proof.extend(proof);
                partial_proof.extend(left_child_id);
                partial_proof.extend(right_child_id);
                partial_proof.extend(byte_range);
                let _resolve_r = self.generate_proofs(&node.right_child,
                                                       &partial_proof,
                                                       depth + 1);
                let _resolve_l = self.generate_proofs(&node.left_child,
                                                       &partial_proof,
                                                       depth + 1);
            }
        }
    }

    pub fn chunk_json(&self, idx: usize) -> String {
        let data_root_string = b64_encode(&self.data_root);
        let data_size_string = self.data_size.to_string();
        let data_path_string = b64_encode(&self.proofs[idx].proof);
        let offset_string = self.proofs[idx].offset.to_string();
        let chunk = &self.chunks[idx].chunk;
        let chunk_string = b64_encode(chunk);
        let ready_to_json = ChunkJson {
            data_root: data_root_string,
            data_size: data_size_string,
            data_path: data_path_string,
            offset: offset_string,
            chunk: chunk_string,
        };
        // println!("{:#?}", ready_to_json);
        serde_json::to_string_pretty(&ready_to_json).unwrap()
    }
}
