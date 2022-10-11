pub mod window;

use crate::{ipfs::IpfsReader, types::*};
use anyhow::{anyhow, Result};
use bao::encode::SliceExtractor;
use cid::Cid;
use ethers::abi::ethereum_types::BigEndianHash;
use ethers::prelude::H256;
use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    sync::Arc,
};

use ipfs_api::{IpfsClient};

/// 1024 bytes per bao chunk
const CHUNK_SIZE: u64 = 1024;
const DAG_BLOCK_SIZE: usize = 256000; // 256kb

struct FakeSeeker<R: Read> {
    reader: R,
    bytes_read: u64,
}

impl<R: Read> FakeSeeker<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            bytes_read: 0,
        }
    }
}

impl<R: Read> Read for FakeSeeker<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.reader.read(buf)?;
        self.bytes_read += n as u64;
        Ok(n)
    }
}

impl<R: Read> Seek for FakeSeeker<R> {
    fn seek(&mut self, _: SeekFrom) -> std::io::Result<u64> {
        // Do nothing and return the current position.
        Ok(self.bytes_read)
    }
}

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

// TODO Is there a more efficient solution to this than reading Block by Block? I think this is good but maybe not ...
// Freeing bytes_read from memory?
pub async fn gen_obao_ipfs(cid: Cid) -> Result<(Vec<u8>, bao::Hash)> {
    let mut encoded_incrementally = Vec::new();
    let encoded_cursor = std::io::Cursor::new(&mut encoded_incrementally);
    let mut encoder = bao::encode::Encoder::new_outboard(encoded_cursor);

    let client = Arc::new(IpfsClient::default());
    let mut ipfs_file: IpfsReader = IpfsReader::new(client, cid)?;
    loop {
        let mut buf: [u8; DAG_BLOCK_SIZE] = [0; DAG_BLOCK_SIZE];
        let bytes_read = ipfs_file.read(&mut buf)?;
        dbg!(bytes_read);
        if bytes_read == 0 {
            break;
        }
        encoder.write_all(&buf[..bytes_read])?;
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


pub async fn gen_proof_ipfs(
    block_hash: H256,
    file_cid: Cid,
    obao_file: Cursor<Vec<u8>>,
    file_length: u64,
) -> Result<Vec<u8>> {
    let (chunk_offset, chunk_size) = compute_random_block_choice_from_hash(block_hash, file_length);
    let client = IpfsClient::default();
    //let mut buf = Vec::with_capacity(chunk_size.try_into().unwrap());
    // length is 0 now and thats fine. 
    let mut buf: [u8; CHUNK_SIZE as usize] = [0; CHUNK_SIZE as usize];
    let mut ipfs_file: IpfsReader = IpfsReader::new(Arc::new(client.clone()), file_cid)?;
    ipfs_file.seek(SeekFrom::Start(chunk_offset))?;
    let bytes_read = ipfs_file.read(&mut buf)?;
    if bytes_read != chunk_size as usize {
        return Err(anyhow!("Bytes read: {:} does not equal chunk size: {:}", bytes_read, chunk_size));
    }
    // issue its calling len on the buf and seeing 0 
    // can I find a way to use a static u8 array? Why cant I just define with size. 

    dbg!(buf.len());
    let mut bao_proof_data = vec![];
    let _ = SliceExtractor::new_outboard(
        FakeSeeker::new(&buf[..chunk_size.try_into().unwrap()]),
        obao_file,
        chunk_offset.try_into().unwrap(),
        chunk_size,
    )
    .read_to_end(&mut bao_proof_data)?;
    Ok(bao_proof_data)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;

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
