use anyhow::Result;
use cid::Cid;
use std::fs::File;
use std::io::BufReader;

pub async fn write_bytes_to_ipfs(_bytes: Vec<u8>) -> Result<Cid> {
    unimplemented!("idk what the right way to do this is??");
}

pub async fn get_handle_for_cid(_cid: Cid) -> Result<BufReader<File>> {
    unimplemented!("https://open.spotify.com/track/2enPRFda84VE2wtI8c86Uf?si=714947276bc3400b")
}

pub async fn _do_we_have_this_cid_locally(_cid: Cid) -> Result<bool> {
    unimplemented!("https://open.spotify.com/track/4vjvx7Zxkb4AltGcZ0BBvI?si=3c7928800a1f4f3b")
}

pub async fn download_file_from_ipfs(_cid: Cid, _length: u64) -> Result<()> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_me_test_me_say_that_youll_test_me() {
        unimplemented!("do it");
    }
}
