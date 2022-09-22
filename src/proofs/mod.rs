pub mod window;

use crate::types::*;
use anyhow::Result;
use bao::encode::SliceExtractor;
use ethers::abi::ethereum_types::BigEndianHash;
use ethers::prelude::H256;
use std::io::{Read, Seek, Cursor};

/// 1024 bytes per bao chunk
const CHUNK_SIZE: u64 = 1024;

fn get_num_chunks(size: u64) -> u64 {
    (size as f32 / CHUNK_SIZE as f32).ceil() as u64
}

/// returns tuple (chunk_offset, chunk_size) for the Nth bao hash that you need to grab :)
pub fn compute_random_block_choice_from_hash(block_hash: H256, file_length: u64) -> (u64, u64) {
    let chunk_number = (block_hash.into_uint() % get_num_chunks(file_length)).as_u64();
    let chunk_offset = chunk_number * CHUNK_SIZE;
    let chunk_size = if chunk_number == get_num_chunks(file_length) - 1 {
        file_length - chunk_offset
    } else {
        CHUNK_SIZE
    };
    (chunk_offset, chunk_size)
}

// TODO: eventually do not load the entire file into memory.
pub fn gen_obao<R: Read>(reader: &mut R) -> Result<(Vec<u8>, bao::Hash)> {
    let mut file_content = Vec::new();
    reader
        .read_to_end(&mut file_content)
        .expect("Unable to read");

    let (obao, hash) = bao::encode::outboard(&file_content);
    Ok((obao, hash)) // return the outboard encoding
}

pub async fn gen_proof<R: Read + Seek>(
    _block_number: BlockNum,
    block_hash: H256,
    file_handle: R,
    obao_handle: Cursor<Vec<u8>>,
    file_length: u64,
) -> Result<Vec<u8>> {
    let (chunk_offset, chunk_size) = compute_random_block_choice_from_hash(block_hash, file_length);

    let mut bao_proof_data = vec![];
    let _ = SliceExtractor::new_outboard(file_handle, obao_handle, chunk_offset, chunk_size)
        .read_to_end(&mut bao_proof_data)?;

    Ok(bao_proof_data)
}
