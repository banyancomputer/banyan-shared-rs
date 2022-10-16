// 1) If the OBAO is being stored on ipfs does that need to be streamed in too? The easy part of creating the
// the proof is to get only the chunk of the file I need. The hard part is reading only the parts of the obao
// in I need. What you really want is to store only the parts of the obao you need. But I don't think it makes sense
// conceptually to be storing the obao over ipfs. The obao doesn't need to be accessible to others and it means people
// can cheat.

use anyhow::Result;
use cid::Cid;
use futures::executor::{block_on, block_on_stream};
use futures::TryStreamExt;
use ipfs_api::{BackendWithGlobalOptions, GlobalOptions, IpfsApi, IpfsClient};
use std::io::Seek;
use std::sync::Arc;
use std::{
    io::{Cursor, Read},
    str::FromStr,
};

pub struct IpfsReader {
    api: Arc<IpfsClient>,
    cid: Cid,
    offset: u64,
    length: u64,
}

impl IpfsReader {
    pub fn new(api: Arc<IpfsClient>, cid: Cid) -> Result<Self> {
        let length = block_on(api.object_stat(&cid.to_string()))?.cumulative_size;
        Ok(Self {
            api,
            cid,
            offset: 0,
            length,
        })
    }
}

impl Read for IpfsReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let length_to_try = buf.len() as u64;
        // TODO make sliceextractor work with async!
        let bytes_from_ipfs = block_on_stream(self.api.cat_range(
            &self.cid.to_string(),
            self.offset as usize,
            length_to_try as usize,
        ));
        let mut bytes_read = 0;

        for bytes in bytes_from_ipfs {
            let bytes = bytes.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let bytes_len = bytes.len();
            buf[bytes_read..bytes_read + bytes_len].copy_from_slice(&bytes);
            bytes_read += bytes_len;
        }
        self.seek(std::io::SeekFrom::Current(bytes_read as i64))?;
        Ok(bytes_read)
    }
}

impl Seek for IpfsReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Start(offset) => {
                self.offset = offset;
                Ok(self.offset)
            }
            std::io::SeekFrom::Current(offset) => {
                let i64_offset: i64 = self.offset as i64 + offset;
                // Seeking to a negative offset is considered an error.
                if i64_offset < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "invalid seek to a negative position",
                    ));
                }
                self.offset = i64_offset as u64;
                Ok(self.offset)
            }
            std::io::SeekFrom::End(offset) => {
                let i64_offset: i64 = self.length as i64 + offset;
                if i64_offset < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "invalid seek to a negative position",
                    ));
                }
                self.offset = i64_offset as u64;
                Ok(self.offset)
            }
        }
    }
}

// As in pin this.
pub async fn write_bytes_to_ipfs(_bytes: Vec<u8>) -> Result<Cid> {
    let client = IpfsClient::default();
    let data = Cursor::new(_bytes);

    let res = client.add(data).await?;
    let hash: Cid = Cid::from_str(&res.hash)?;
    println!("Hash: {:?}", hash);
    Ok(hash)
}

// Do we have this file pinned
pub async fn do_we_have_this_cid_locally(cid: Cid) -> Result<bool> {
    let client = IpfsClient::default();
    match client.pin_ls(Some(&cid.to_string()), None).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// Unpin this file
pub async fn unpin_cid(cid: Cid) -> Result<()> {
    if do_we_have_this_cid_locally(cid).await? {
        let client = IpfsClient::default();
        client.pin_rm(&cid.to_string(), true).await?;
    }
    Ok(())
}

pub async fn download_and_pin_file_from_ipfs(cid: Cid) -> Result<()> {
    let client = IpfsClient::default();
    client.pin_add(&cid.to_string(), true).await?;
    Ok(())
}

// Get the bytes for this cid
pub async fn download_file_from_ipfs(cid: Cid) -> Result<Vec<u8>> {
    let client = BackendWithGlobalOptions::new(
        IpfsClient::default(),
        GlobalOptions::builder()
            .offline(true) // This is the entire trick!
            .build(),
    );
    let all = client
        .cat(&cid.to_string())
        .map_ok(|chunk| chunk.to_vec())
        .try_concat()
        .await?;
    Ok(all)
}

//testing
#[cfg(test)]
mod tests {
    use super::*;
    use cid::Cid;
    use multihash::Code;
    use multihash::MultihashDigest;
    use crate::{
        proofs::{self},
        types::*,
        eth::{EthClient},
        ipfs::{write_bytes_to_ipfs},
    };
    use anyhow::Result;

    #[tokio::test]
    async fn add_and_download_file() -> Result<()> {
        let cid = write_bytes_to_ipfs("hello world!".as_bytes().to_vec()).await?;
        dbg!(&cid.to_string());
        let file = download_file_from_ipfs(cid).await?;
        assert_eq!(file, "hello world!".as_bytes().to_vec());
        Ok(())
    }

    #[tokio::test]
    async fn file_is_local() -> Result<()> {
        let cid = write_bytes_to_ipfs("hello world2".as_bytes().to_vec()).await?;
        let bool = do_we_have_this_cid_locally(cid).await?;
        assert_eq!(bool, true);
        Ok(())
    }

    #[tokio::test]
    async fn file_is_not_local() -> Result<()> {
        let h = Code::Sha2_256.digest(b"beep boop");
        let cid = Cid::new_v0(h)?;
        let bool = do_we_have_this_cid_locally(cid).await?;
        assert_eq!(bool, false);
        Ok(())
    }

    #[tokio::test]
    async fn get_then_pin_then_check() -> Result<()> {
        let hash = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let cid = Cid::try_from(hash)?;
        download_and_pin_file_from_ipfs(cid).await?;
        //let file = download_file_from_ipfs(cid).await?;
        let bool = do_we_have_this_cid_locally(cid).await?;
        assert_eq!(bool, true);
        Ok(())
    }

    #[tokio::test]
    async fn unpin_then_check() -> Result<()> {
        let hash = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let cid = Cid::try_from(hash)?;
        unpin_cid(cid).await?;
        let bool = do_we_have_this_cid_locally(cid).await?;
        assert_eq!(bool, false);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_file_and_read () -> Result<()>
    {
        let file = "hello world!".as_bytes().to_vec();
        let cid = write_bytes_to_ipfs(file).await?;
        let mut buf: [u8; 12] = [0; 12];
        let client = Arc::new(IpfsClient::default());
        let mut ipfs_file: IpfsReader = IpfsReader::new(client, cid)?;
        ipfs_file.read(&mut buf)?;
        assert_eq!(buf, "hello world!".as_bytes());
        Ok(())
    } 

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ethereum_proof() -> Result<()> {
        let eth_client = EthClient::default();
        let deal = eth_client.get_offer(DealID(1)).await.unwrap();

        let target_window: usize = eth_client
            .compute_target_window(deal.deal_start_block, deal.proof_frequency_in_blocks)
            .await
            .expect("Failed to compute target window");

        let target_block = EthClient::compute_target_block_start(
            deal.deal_start_block,
            deal.proof_frequency_in_blocks,
            target_window,
        );

        let root = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let file_cid = Cid::try_from(root)?;
        let target_block_hash = eth_client.get_block_hash_from_num(target_block).await?;
        let (obao_file, hash) = proofs::gen_obao_ipfs(file_cid).await?;
        let obao_cid = write_bytes_to_ipfs(obao_file).await?;

        let proof: Vec<u8> = proofs::gen_proof_ipfs(
            target_block_hash,
            file_cid,
            obao_cid,
            deal.file_size.as_u64(),
        )
            .await
            .unwrap();
        let (chunk_offset, chunk_size) = proofs::compute_random_block_choice_from_hash(
            target_block_hash,
            deal.file_size.as_u64(),
        );
        assert_eq!(
            true,
            EthClient::check_if_merkle_proof_is_valid(
                Cursor::new(&proof),
                hash,
                chunk_offset,
                chunk_size,
            )?
        );
        Ok(())
    }
}
