pub mod window;

use crate::types::*;
use anyhow::Result;
use bao::encode::SliceExtractor;
use cid::Cid;
use ethers::abi::ethereum_types::BigEndianHash;
use ethers::prelude::H256;
use std::{io::{Cursor, Read, Seek, Write}, fs::File};

use ipfs_api::{request::Ls, response::LsResponse, ApiError, BackendWithGlobalOptions, Error as IpfsError,
    GlobalOptions, IpfsApi, IpfsClient};
use futures::TryStreamExt;

/// 1024 bytes per bao chunk
const CHUNK_SIZE: u64 = 1024;
const DAG_BLOCK_SIZE: usize = 256000; // 256kb

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

pub fn gen_obao_incremental<R: Read + Seek>(reader: &mut R) -> Result<(Vec<u8>, bao::Hash)> {

    let mut encoded_incrementally = Vec::new();
    let encoded_cursor = std::io::Cursor::new(&mut encoded_incrementally);
    let mut encoder = bao::encode::Encoder::new_outboard(encoded_cursor);

    loop {
        let mut buffer = [0; CHUNK_SIZE as usize];
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        encoder.write_all(&buffer[..bytes_read])?;
    }
    let hash = encoder.finalize()?;
    Ok((encoded_incrementally, hash)) // return the outboard encoding   
}

// TODO Is there a more efficient solution to this than reading Block by Block? I think this is good but maybe not ... 
pub async fn gen_obao_ipfs(cid: Cid) -> Result<(Vec<u8>, bao::Hash)> {
    let mut encoded_incrementally = Vec::new();
    let encoded_cursor = std::io::Cursor::new(&mut encoded_incrementally);
    let mut encoder = bao::encode::Encoder::new_outboard(encoded_cursor);
    let client = IpfsClient::default();
    let mut offset = 0;
    loop {
        let bytes = client
            .cat_range(&cid.to_string(),offset, DAG_BLOCK_SIZE)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?;
        let bytes_read = bytes.len();
        println!("bytes_read: {}", bytes_read);
        if bytes_read == 0 {
            break;
        }
        encoder.write_all(&bytes)?;
        offset += bytes_read;
    }
    let hash = encoder.finalize()?;
    Ok((encoded_incrementally, hash)) 
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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]

    async fn compare_obao() -> Result<()> {
        let mut file = File::open("../Rust-Chainlink-EA-API/test_files/ethereum.pdf").unwrap();
        let (obao, hash) = gen_obao(&mut file).unwrap();

        let root = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let cid = Cid::try_from(root)?;
        let (obao_ipfs, hash_ipfs) = gen_obao_ipfs(cid).await.unwrap();
        assert_eq!(hash, hash_ipfs);
        assert_eq!(obao, obao_ipfs);
        Ok(())
    }
    

}