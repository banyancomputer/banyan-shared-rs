use crate::types::*;
use anyhow::{anyhow, Result};
use ethers::prelude::H256;
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{Filter, Log};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

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
    timeout: Duration,
}

impl VitalikProvider {
    pub fn new(url: String, timeout_seconds: u64) -> Result<Self> {
        Ok(Self {
            provider: Mutex::new(Provider::<Http>::try_from(url)?),
            // my_pending_transactions: HashMap::new(),
            timeout: Duration::from_secs(timeout_seconds),
        })
    }

    pub async fn get_latest_block_num(&self) -> Result<BlockNum> {
        let provider = self.provider.lock().await;
        let block = timeout(self.timeout, provider.get_block_number()).await??;
        Ok(BlockNum(block.as_u64()))
    }

    pub async fn get_block_hash_from_num(&self, block_number: BlockNum) -> Result<H256> {
        let provider = self.provider.lock().await;
        let block = timeout(self.timeout, provider.get_block(block_number.0))
            .await??
            .ok_or_else(|| anyhow!("block not found"))?;
        block.hash.ok_or_else(|| anyhow!("block hash not found"))
    }

    pub async fn get_logs_from_filter(&self, filter: Filter) -> Result<Vec<Log>> {
        let provider = self.provider.lock().await;
        let logs = timeout(self.timeout, provider.get_logs(&filter)).await??;
        Ok(logs)
    }

    pub async fn get_onchain(&self, _deal_id: DealID) -> Result<OnChainDealInfo> {
        unimplemented!("write me ;)")
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
