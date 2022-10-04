// 1) If the OBAO is being stored on ipfs does that need to be streamed in too? The easy part of creating the 
// the proof is to get only the chunk of the file I need. The hard part is reading only the parts of the obao 
// in I need. What you really want is to store only the parts of the obao you need. But I don't think it makes sense 
// conceptually to be storing the obao over ipfs. The obao doesn't need to be accessible to others and it means people 
// can cheat. 

use anyhow::Result;
use cid::{Cid, multihash::{Code, MultihashDigest}};
use std::{fs::File, str::FromStr, convert::TryFrom, io::{BufReader,Cursor, Write, Read}};
use ipfs_api::{request::Ls, response::LsResponse, ApiError, BackendWithGlobalOptions, Error as IpfsError,
    GlobalOptions, IpfsApi, IpfsClient};
use futures::TryStreamExt;

// As in pin this. 
pub async fn write_bytes_to_ipfs(_bytes: Vec<u8>) -> Result<Cid> {
    let client = IpfsClient::default();
    let data = Cursor::new(_bytes);

    let res = client.add(data).await?;
    let hash: Cid = Cid::from_str(&res.hash)?;
    println!("Hash: {:?}", hash);
    Ok(hash)
}

// TODO: Change ipfs_proof_buddy, since this is not needed if we are just passing ipfs cids to 
// stream files in instead of passing the file handle itself.
/*
pub async fn get_handle_for_cid(cid: Cid) -> Result<BufReader<File>> {
    let bytes = download_file_from_ipfs(cid).await?;
    let mut file = File::create("banyan_files/".to_owned() + &cid.to_string())?;
    file.write_all(&bytes)?;
    let reader = BufReader::new(file);
    return Ok(reader);
}
*/

// Do we have this file pinned
pub async fn do_we_have_this_cid_locally(cid: Cid) -> Result<bool> {
    let client = IpfsClient::default();
    match client.pin_ls(Some(&cid.to_string()), None).await
    {
        Ok(_) => Ok(true),
        Err(_) => Ok(false)
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

    #[tokio::test]
    async fn add_and_download_file() -> Result<()>{
        let cid = write_bytes_to_ipfs("hello world!".as_bytes().to_vec()).await?;
        let file = download_file_from_ipfs(cid).await?;
        assert_eq!(file, "hello world!".as_bytes().to_vec());
        Ok(())
    }

    #[tokio::test]
    async fn file_is_local() -> Result<()>{
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
    async fn get_then_pin_then_check () -> Result<()> {
        let hash = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let cid = Cid::try_from(hash)?;
        download_and_pin_file_from_ipfs(cid).await?;
        //let file = download_file_from_ipfs(cid).await?;
        let bool = do_we_have_this_cid_locally(cid).await?;
        assert_eq!(bool, true);
        Ok(())
    }

    #[tokio::test]
    async fn unpin_then_check () -> Result<()> {
        let hash = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let cid = Cid::try_from(hash)?;
        unpin_cid(cid).await?;
        let bool = do_we_have_this_cid_locally(cid).await?;
        assert_eq!(bool, false);
        Ok(())
    }
}