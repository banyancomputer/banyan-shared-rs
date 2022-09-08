use anyhow::Result;
use multihash::{Code, Hasher, Multihash, MultihashDigest, Sha2_256};
use std::io::Read;

/*
 * A Really simple hasher lib.
 * Status: Just trying to get stuff to work.
 * Not designed to take advantage of the parallelism of Blake3.
 * This should just wrap the Blake3 Hash function with the IO interface we need.
 * TODO: Audit, Research, Make better
 */

/// How big of a buffer to use when reading from a file - 16Kb
const B3_HASHER_CHUNK_SIZE: usize = 65536;

/// A Blake3 Hasher
pub struct FileHasher<'a> {
    /// A File to Hash
    input: &'a std::fs::File,
}

impl<'a> FileHasher<'a> {
    /// Create a new Hasher
    pub fn new(input: &'a std::fs::File) -> Self {
        Self { input }
    }

    /// Return a Sha2-256 MultiHash of the file
    pub fn multihash(&mut self) -> Result<Multihash> {
        let mut hasher = Sha2_256::default();
        let mut buffer = [0; B3_HASHER_CHUNK_SIZE]; // TODO: What's the right size?
        loop {
            let count = self.input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        Ok(Code::Sha2_256.wrap(hasher.finalize()).unwrap())
    }

    /// Hash the input - Return as a Hash
    pub fn b3hash(&mut self) -> Result<blake3::Hash> {
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0; B3_HASHER_CHUNK_SIZE];
        loop {
            let bytes_read = self.input.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(hasher.finalize())
    }
}
