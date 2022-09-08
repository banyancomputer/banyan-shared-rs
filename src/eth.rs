use crate::types::*;
use anyhow::{anyhow, Result};
use ethers::{
    abi::{Abi, Tokenize},
    contract::Contract,
    prelude::{builders::ContractCall, AbiError, H256},
    providers::{Http, Middleware, Provider},
    types::{Address, Filter, Log, U256},
};

use multibase::decode;
use multihash::Multihash;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fs, io::Cursor, str::FromStr};

use tokio::{
    sync::Mutex,
    time::{timeout, Duration},
};

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
    contract: Mutex<Contract<Provider<Http>>>,
}

impl VitalikProvider {
    pub fn new(url: String, timeout_seconds: u64) -> Result<Self> {
        let provider = Provider::<Http>::try_from(url)?;
        
        let provider2 = provider.clone();
        let address = "0x9ee596734485268eF62db4f3E61d891E221504f6" //0xeb3d5882faC966079dcdB909dE9769160a0a00Ac
            .parse::<Address>()
            .expect("could not parse contract address");
        let abi: Abi = serde_json::from_str(
            fs::read_to_string("contract_abi.json")
                .expect("can't read file")
                .as_str(),
        )
        .expect("couldn't load abi");

        Ok(Self {
            provider: Mutex::new(provider),
            // my_pending_transactions: HashMap::new(),
            timeout: Duration::from_secs(timeout_seconds),
            contract: Mutex::new(Contract::new(address, abi, provider2)),
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

    pub async fn get_block_num_from_window(
        &self,
        deal_id: DealID,
        window_num: u64,
    ) -> Result<BlockNum> {
        let block_num = self
            .u256_method("getProofBlock", (deal_id.0, window_num))
            .await?
            .call()
            .await?
            .as_u64();
        let res = BlockNum(block_num);
        Ok(res)
    }

    pub async fn u256_method(
        &self,
        name: &str,
        args: impl Tokenize,
    ) -> Result<ContractCall<Provider<Http>, U256>, AbiError> {
        let contract = self.contract.lock().await;
        contract.method::<_, U256>(name, args)
    }

    pub async fn address_method(
        &self,
        name: &str,
        args: impl Tokenize,
    ) -> Result<ContractCall<Provider<Http>, Address>, AbiError> {
        let contract = self.contract.lock().await;
        contract.method::<_, Address>(name, args)
    }

    pub async fn string_method(
        &self,
        name: &str,
        args: impl Tokenize,
    ) -> Result<ContractCall<Provider<Http>, String>, AbiError> {
        let contract = self.contract.lock().await;
        contract.method::<_, String>(name, args)
    }

    pub async fn get_onchain(&self, deal_id: DealID) -> Result<OnChainDealInfo> {
        //unimplemented!("write me ;)")
        let offer_id = deal_id.0;

        let deal_start_block: BlockNum = BlockNum(
            self.u256_method("getDealStartBlock", offer_id)
                .await?
                .call()
                .await?
                .as_u64(),
        );
        println!("{:?}", deal_start_block);
        let deal_length_in_blocks: BlockNum = BlockNum(
            self.u256_method("getDealLengthInBlocks", offer_id)
                .await?
                .call()
                .await?
                .as_u64(),
        );
        println!("{:?}", deal_length_in_blocks);
        let proof_frequency_in_blocks: BlockNum = BlockNum(
            self.u256_method("getProofFrequencyInBlocks", offer_id)
                .await?
                .call()
                .await?
                .as_u64(),
        );

        let price: TokenAmount = TokenAmount(
            self.u256_method("getPrice", offer_id)
                .await?
                .call()
                .await?
                .as_u64(),
        );

        let collateral: TokenAmount = TokenAmount(
            self.u256_method("getCollateral", offer_id)
                .await?
                .call()
                .await?
                .as_u64(),
        );

        let erc20_token_denomination: Token = Token(
            self.address_method("getErc20TokenDenomination", offer_id)
                .await?
                .call()
                .await?,
        );

        let cid_return: String = self
            .string_method("getIpfsFileCid", offer_id)
            .await?
            .call()
            .await?;

        let code = "z".to_owned();
        let full_cid = format!("{}{}", code, cid_return);
        let (_, decoded) = decode(full_cid)?;
        let reader = Cursor::new(decoded);
        let ipfs_file_cid = cid::CidGeneric::new_v0(Multihash::read(reader)?)?;

        let file_size: u64 = self
            .u256_method("getFileSize", offer_id)
            .await?
            .call()
            .await?
            .as_u64();

        let blake3_return: String = self
            .string_method("getBlake3Checksum", offer_id)
            .await?
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
    
    // the validator should be able to handle if proofs get sent twice on accident
    // return the block number that the proof made it into.
    pub async fn post_proof(&self, _deal_id: &DealID, _proof: Proof) -> Result<BlockNum> {
        unimplemented!("write me :)")
    }

    pub async fn accept_deal_on_chain(&self) -> Result<OnChainDealInfo> {
        unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
    }
}
