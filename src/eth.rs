use crate::types::*;
use anyhow::{anyhow, Result};
use ethers::{
    abi::Abi,
    contract::Contract,
    prelude::H256,
    providers::{Http, Middleware, Provider},
    types::{Address, Filter, Log, U256},
};

use cid::Cid;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fs, str::FromStr};

use tokio::sync::Mutex;
use std::ops::{Add, Mul};

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

    pub async fn get_contract_address(&self) -> Result<Address> {
        let contract = self.contract.lock().await;
        Ok(contract.address())
    }

    pub async fn get_proof_from_logs(&self, submitted_proof_in_block_num: BlockNum, deal_id: BlockNum) -> Vec<Log> 
    {
        let address = self
        .get_contract_address()
        .await
        .map_err(|e| format!("Could not get contract address: {e}"))?;

        let filter = Filter::new()
            .select(submitted_proof_in_block_num.0)
            // TODO figure this guy out later :)
            .address(address)
            .topic1(H256::from_low_u64_be(deal_id.0));

        let block_logs = self.get_logs_from_filter(&filter).await.map_err(|e| {
            format!(
                "Couldn't get log from block {}: {}",
                submitted_proof_in_block_num.0, e
            )
        })?;
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

    pub async fn get_logs_from_filter(&self, filter: &Filter) -> Result<Vec<Log>> {
        let provider = self.provider.lock().await;
        let logs = provider.get_logs(filter).await?;
        Ok(logs)
    }

    /* 
    pub async fn get_logs_alternate(&self, filter: &Filter) -> Result<Vec<Log>> {
        let provider = self.provider.lock().await;
        let data_bytes = data.get(64..end);
        Ok(logs)
    }
    */

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

    pub fn compute_target_window_start(deal_start_block: BlockNum, proof_frequency_in_blocks: BlockNum, window_num: usize) -> BlockNum {
        Add::add(Mul::mul(proof_frequency_in_blocks,window_num), deal_start_block)
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

    async fn on_chain () ->  Result<(), anyhow::Error> 
    {
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