pub mod window;

use crate::types::*;
use anyhow::Result;
use bao::encode::SliceExtractor;
use ethers::abi::ethereum_types::BigEndianHash;
use ethers::prelude::H256;
use std::io::{Cursor, Read, Seek};

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

#[cfg(test)]
mod tests {
    use crate::types::BlockNum;
    use ethers::abi::ethereum_types::BigEndianHash;
    use ethers::prelude::H256;
    use ethers::prelude::U256;
    use std::io::Read;

    #[test]
    fn test_compute_random_block_choice_from_hash() {
        // TODO test behavior around int overflows?
        use crate::proofs::compute_random_block_choice_from_hash;
        let file_length = 2049;
        let block_hash = H256::from_uint(&U256::from(0u64));
        let (chunk_offset, chunk_size) =
            compute_random_block_choice_from_hash(block_hash, file_length);
        assert_eq!(chunk_offset, 0);
        assert_eq!(chunk_size, 1024);
        let block_hash = H256::from_uint(&U256::from(1u64));
        let (chunk_offset, chunk_size) =
            compute_random_block_choice_from_hash(block_hash, file_length);
        assert_eq!(chunk_offset, 1024);
        assert_eq!(chunk_size, 1024);
        let block_hash = H256::from_uint(&U256::from(2u64));
        let (chunk_offset, chunk_size) =
            compute_random_block_choice_from_hash(block_hash, file_length);
        assert_eq!(chunk_offset, 2048);
        assert_eq!(chunk_size, 1);
        let block_hash = H256::from_uint(&U256::from(379u64)); // 379 % 3 = 1
        let (chunk_offset, chunk_size) =
            compute_random_block_choice_from_hash(block_hash, file_length);
        assert_eq!(chunk_offset, 1024);
        assert_eq!(chunk_size, 1024);
    }

    #[test]
    fn test_get_num_chunks() {
        use crate::proofs::get_num_chunks;
        assert_eq!(get_num_chunks(1024), 1);
        assert_eq!(get_num_chunks(1025), 2);
        assert_eq!(get_num_chunks(2048), 2);
        assert_eq!(get_num_chunks(2049), 3);
    }

    #[tokio::test]
    async fn test_obao_genproof_roundtrip() {
        use crate::proofs::{gen_obao, gen_proof};
        use std::io::Cursor;
        let file_content = b"hello world".to_vec();
        let (obao, bao_hash) = gen_obao(Cursor::new(&file_content)).unwrap();
        let fake_block_hash = H256::from_uint(&U256::from(379u64)); // 379 % 3 = 1

        let proof = gen_proof(
            BlockNum(3),
            fake_block_hash,
            Cursor::new(&file_content),
            Cursor::new(&obao),
            file_content.len() as u64,
        )
        .await
        .unwrap();
        let mut proof_reader = Cursor::new(proof.bao_proof_data);
        let mut proof_content = vec![];
        proof_reader.read_to_end(&mut proof_content).unwrap();

        let validated_data = bao::decode::decode(&proof_content, &bao_hash).unwrap();
        assert_eq!(validated_data, file_content);
    }

    #[tokio::test]
    async fn test_proof_validation() {
        unimplemented!("wooooohoooo!");
    }
}
