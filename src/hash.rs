use anyhow::{anyhow, Error, Result};
use multihash::{Code, Hasher, Multihash, MultihashDigest, Sha2_256};
use std::io;
use std::io::{BufReader, Read};

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

/// Our File Hasher
/// Janky as hell, but it works.
impl<'a> FileHasher<'a> {
    /// Create a new Hasher
    pub fn new(input: &'a std::fs::File) -> Self {
        Self { input }
    }

    /// Return a Sha2-256 Multihash and Blake3 Hash for a file
    pub fn hash(&mut self) -> Result<(Multihash, blake3::Hash), Error> {
        let mut multi_hasher = Sha2_256::default();
        let mut b3_hasher = blake3::Hasher::new();
        let mut buffer = [0; B3_HASHER_CHUNK_SIZE]; // TODO: What's the right size?
        let mut reader = BufReader::new(self.input);
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    return Ok((
                        Code::Sha2_256.wrap(multi_hasher.finalize()).unwrap(),
                        b3_hasher.finalize(),
                    ))
                }
                Ok(n) => {
                    b3_hasher.update(&buffer[..n]);
                    multi_hasher.update(&buffer[..n]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(anyhow!(e)),
            }
        }
    }
}
