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

// As in get this file? why do this right after download in estuary_talker? 
pub async fn get_handle_for_cid(cid: Cid) -> Result<BufReader<File>> {
    let bytes = download_file_from_ipfs(cid).await?;
    let mut file = File::create("banyan_files/".to_owned() + &cid.to_string())?;
    file.write_all(&bytes)?;
    let reader = BufReader::new(file);
    return Ok(reader);
}

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

/*
MFS VERSION 
ISSUE: MFS files have a different hash since they use a different dag builder protocol.
Core Q: How will the files be stored. Estuary is storing in a local drive configured when the node 
starts. Should we do the same thing for the storage providers? 
ISSUE: How can I stored the regular ipfs pinned files in a directory where I can access them? Like I am already pinning this file. 
Why am I calling ipfs get and downloading it again/ creating another version of it? 
Even if I use mfs, that is still a wrapper around the ipfs dag. Like fundamentally, they are storing the file in a very different way
and if I want a file handle, then I need to create a new file object. The reason I want a file handle is because I cant pass a DAG
into BAO. It needs to be normal file. 
And then also, when we want to build our proof, we need the file again, and like need it not as a fucking DAG. 

TIED TO THIS IS LIKE how do I stream the obao file and the ipfs file into memory to create the proof. 

Unix fs data format 

Maybe this is where we take the BoxStream<Bytes, Self::Error> into account? 


ipfs reader type that has the trait bufreader
test on a file thats a terabyte 

sliceExtractor just needs a trait with the type Read. 

hasher Add 
alignment of lookups 

what if the obao is all in memory and it gets umkilled (out of memory killed) 


*/

pub async fn mfs_download_and_pin (cid: Cid) -> Result<()> {
    let client = IpfsClient::default();
    let bytes = download_file_from_ipfs(cid).await?;
    let src = Cursor::new(bytes);
    let path = &("/testing/".to_owned() + &cid.to_string());
    client.files_write(path, true, true, src).await?;
    Ok(())
}

pub async fn mfs_stat (cid: Cid) -> Result<()> {
    let client = IpfsClient::default();
    let path = &("/testing/".to_owned() + &cid.to_string());
    let res = client.files_stat(path).await?;
    println!("MFS Stat: {:?}", res);
    Ok(())
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

    #[tokio::test]
    async fn get_file_handle () -> Result<()> {
        let hash = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let cid = Cid::try_from(hash)?;
        let mut file = get_handle_for_cid(cid).await?;
        let mut contents = vec![];
        file.read_to_end(&mut contents)?;
        let check_cid = Cid::new_v0(Code::Sha2_256.digest(&contents))?;
        assert_eq!(check_cid, cid);
        Ok(())
    }

    #[tokio::test]
    pub async fn mfs_write_test () -> Result<()> {
        let hash = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let cid = Cid::try_from(hash)?;
        mfs_download_and_pin(cid).await?;
        mfs_stat(cid).await?;
        Ok(())
    }

    #[tokio::test]
    pub async fn mfs_stat_test () -> Result<()> {
        let hash = "Qmd63gzHfXCsJepsdTLd4cqigFa7SuCAeH6smsVoHovdbE";
        let cid = Cid::try_from(hash)?;
        mfs_stat(cid).await?;
        Ok(())
    }

}