use crate::types::*;
use anyhow::{anyhow, Result};
use ethers::{
    abi::Abi,
    contract::Contract,
    prelude::H256,
    providers::{Http, Middleware, Provider},
    types::{Address, Filter, U256},
};

use cid::Cid;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fs, str::FromStr};

use std::{
    io::{Cursor, Read},
    ops::{Add, Mul},
};
use tokio::sync::Mutex;

const WORD: usize = 32;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofBuddyMessageType {
    SubmitProof,
    Cancel,
    InitiateChainlinkFinalization,
    WithdrawEarnings,
}

pub struct VitalikProvider {
    provider: Mutex<Provider<Http>>,
    // TODO: eventually we will need to handle if the provider falls over halfway through submitting a transaction.
    //my_pending_transactions: HashMap<TxnHash, PendingTransaction<'a>>,
    pub contract: Mutex<Contract<Provider<Http>>>,
}

impl VitalikProvider {
    pub fn new(url: String, contract_address: String) -> Result<Self> {
        let provider = Provider::<Http>::try_from(url)?;
        let provider2 = provider.clone();
        let address = contract_address
            .parse::<Address>()
            .expect("could not parse contract address");
        let abi: Abi = serde_json::from_str(
            fs::read_to_string("contract_abi.json")
                .expect("can't read file")
                .as_str(),
        )
        .expect("couldn't load abi");

        // TODO is this the right place to be sticking mutexes?
        Ok(Self {
            provider: Mutex::new(provider),
            // my_pending_transactions: HashMap::new(),
            //timeout: Duration::from_secs(timeout_seconds),
            contract: Mutex::new(Contract::new(address, abi, provider2)),
        })
    }

    pub async fn get_proof_from_logs(
        &self,
        submitted_proof_in_block_num: BlockNum,
        deal_id: DealID,
    ) -> Result<Option<Vec<u8>>> {
        let contract = self.contract.lock().await;
        let address = contract.address();

        let filter = Filter::new()
            .select(submitted_proof_in_block_num.0)
            // TODO figure this guy out later :)
            .address(address)
            .topic1(H256::from_low_u64_be(deal_id.0));

        let provider = self.provider.lock().await;
        let block_logs = provider.get_logs(&filter).await?;

        // The first two 32 byte words of log data are a pointer and the size of the data.
        // TODO put this in banyan_shared!
        let data = &block_logs[0].data;
        if data.len() < WORD * 2 {
            return Ok(None); // Err(format!("Data is too short: {:?}", data));
        }
        // TODO This works probably as a solution but might be a bug. I could submit a really long block,
        // but since only takes last 8 bytes, it could slice the length value such that it read the
        // length as the actual size. That would cause it to only read the first x bytes of the proof,
        // and then it could read an incorrect proof as correct. BUTT the proof would have to have to
        // be incorrect in such a way that it had the correct proof in the first x bytes, and then other
        // incorrect stuff after, which means someone would have had to construct an correct proof and then
        // decide to append other stuff for some reason. I don't know how that would help an attacker.
        // Is this a bug? I am not sure.

        let mut a = [0u8; 8];
        a.clone_from_slice(&data[(WORD * 2) - 8..WORD * 2]);
        let data_size = u64::from_be_bytes({ a });
        if data.len() < WORD * 2 + data_size as usize {
            return Ok(None);
        }
        let data_bytes: Vec<u8> = (&data[WORD * 2..WORD * 2 + data_size as usize]).to_vec();
        Ok(Some(data_bytes))
    }

    pub async fn get_latest_block_num(&self) -> Result<BlockNum> {
        let provider = self.provider.lock().await;
        let block = provider.get_block_number().await?;
        Ok(BlockNum(block.as_u64()))
    }

    pub async fn get_block_hash_from_num(&self, block_number: BlockNum) -> Result<H256> {
        let provider = self.provider.lock().await;
        let block = provider
            .get_block(block_number.0)
            .await?
            .ok_or_else(|| anyhow!("block not found"))?;
        block.hash.ok_or_else(|| anyhow!("block hash not found"))
    }

    pub async fn get_proof_block_num_from_window(
        &self,
        deal_id: DealID,
        window_num: u64,
    ) -> Result<Option<BlockNum>> {
        let contract = self.contract.lock().await;
        let block_num = contract
            .method::<_, U256>("getProofBlock", (deal_id.0, window_num))?
            .call()
            .await?
            .as_u64();
        let res = BlockNum(block_num);
        if res == BlockNum(0) {
            Ok(None)
        } else {
            Ok(Some(res))
        }
    }

    pub async fn get_onchain(&self, deal_id: DealID) -> Result<OnChainDealInfo> {
        let offer_id = deal_id.0;
        let contract = self.contract.lock().await;

        let deal_start_block: BlockNum = BlockNum(
            contract
                .method::<_, U256>("getDealStartBlock", offer_id)?
                .call()
                .await?
                .as_u64(),
        );

        let deal_length_in_blocks: BlockNum = BlockNum(
            contract
                .method::<_, U256>("getDealLengthInBlocks", offer_id)?
                .call()
                .await?
                .as_u64(),
        );

        let proof_frequency_in_blocks: BlockNum = BlockNum(
            contract
                .method::<_, U256>("getProofFrequencyInBlocks", offer_id)?
                .call()
                .await?
                .as_u64(),
        );

        let price: TokenAmount = TokenAmount(
            contract
                .method::<_, U256>("getPrice", offer_id)?
                .call()
                .await?
                .as_u64(),
        );

        let collateral: TokenAmount = TokenAmount(
            contract
                .method::<_, U256>("getCollateral", offer_id)?
                .call()
                .await?
                .as_u64(),
        );

        let erc20_token_denomination: Token = Token(
            contract
                .method::<_, Address>("getErc20TokenDenomination", offer_id)?
                .call()
                .await?,
        );

        let cid_return: String = contract
            .method::<_, String>("getIpfsFileCid", offer_id)?
            .call()
            .await?;

        let ipfs_file_cid = Cid::from_str(cid_return.as_str())?;

        let file_size: u64 = contract
            .method::<_, U256>("getFileSize", offer_id)?
            .call()
            .await?
            .as_u64(); // TODO this panics! fix this situation. be careful

        let blake3_return: String = contract
            .method::<_, String>("getBlake3Checksum", offer_id)?
            .call()
            .await?;
        let blake3_checksum = bao::Hash::from_str(&blake3_return)?;

        let deal_info: OnChainDealInfo = OnChainDealInfo {
            deal_id,
            deal_start_block,
            deal_length_in_blocks,
            proof_frequency_in_blocks,
            price,
            collateral,
            erc20_token_denomination,
            ipfs_file_cid,
            file_size,
            blake3_checksum,
        };

        Ok(deal_info)
    }

    pub fn compute_target_window_start(
        deal_start_block: BlockNum,
        proof_frequency_in_blocks: BlockNum,
        window_num: usize,
    ) -> BlockNum {
        Add::add(
            Mul::mul(proof_frequency_in_blocks, window_num),
            deal_start_block,
        )
    }

    pub fn check_if_merkle_proof_is_valid(
        proof_bytes: Cursor<&Vec<u8>>,
        blake3_checksum: bao::Hash,
        chunk_offset: u64,
        chunk_size: u64,
    ) -> Result<bool> {
        if bao::decode::SliceDecoder::new(proof_bytes, &(blake3_checksum), chunk_offset, chunk_size)
            .read_to_end(&mut vec![])
            .is_ok()
        {
            return Ok(true);
        } else {
            return Ok(false);
        };
    }

    // the validator should be able to handle if proofs get sent twice on accident
    // return the block number that the proof made it into.
    pub async fn post_proof(&self, _deal_id: &DealID, _proof: Proof) -> Result<BlockNum> {
        unimplemented!("write me :)")
    }

    pub async fn accept_deal_on_chain(&self) -> Result<OnChainDealInfo> {
        unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
    }
}

// tests
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]

    async fn on_chain() -> Result<(), anyhow::Error> {
        let deal_id = DealID(1);
        let api_url = std::env::var("URL").expect("URL must be set.");
        let api_key = std::env::var("API_KEY").expect("API_KEY must be set.");
        let url = format!("{}{}", api_url, api_key);
        let contract_address =
            std::env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS must be set.");
        let provider = VitalikProvider::new(url, contract_address)
            .map_err(|e| format!("error with creating provider: {e}"))?;

        let deal_info = provider
            .get_onchain(deal_id)
            .await
            .map_err(|e| format!("Error in get_onchain: {:?}", e))?;
    }
}
