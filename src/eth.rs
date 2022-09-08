use crate::types::*;
use anyhow::{anyhow, Result};
use ethers::abi::Tokenize;
use ethers::prelude::builders::ContractCall;
use ethers::prelude::{H256, BaseContract, AbiError};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{Filter, Log, H160, U256, Address};
use ethers::contract::Contract;
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

pub struct VitalikContract {
    contract: Mutex<Contract<Provider<Http>>>,
    timeout: Duration
}

impl VitalikContract {
    pub fn new(address: H160, abi: impl Into<BaseContract>, client: Provider<Http>, timeout_seconds: u64) -> Result<Self> {
        Ok(Self {
            contract: Mutex::new(Contract::new(address, abi, client)),
            timeout: Duration::from_secs(timeout_seconds),
        })
    }

    pub async fn u256_method(&self, name: &str, args: impl Tokenize) -> Result<ContractCall<Provider<Http>, U256>, AbiError> {
        let contract = self.contract.lock().await;
        contract.method::<_, U256>(name, args)
    }

    pub async fn address_method(&self, name: &str, args: impl Tokenize) -> Result<ContractCall<Provider<Http>, Address>, AbiError> {
        let contract = self.contract.lock().await;
        contract.method::<_, Address>(name, args)
    }

    pub async fn string_method(&self, name: &str, args: impl Tokenize) -> Result<ContractCall<Provider<Http>, String>, AbiError> {
        let contract = self.contract.lock().await;
        contract.method::<_, String>(name, args)
    }
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
