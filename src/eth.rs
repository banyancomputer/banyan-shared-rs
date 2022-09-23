use crate::{
    proofs::{self, gen_proof},
    types::*,
};
use anyhow::{anyhow, Error, Result};
use ethers::{
    abi::Abi,
    contract::Contract,
    middleware::SignerMiddleware,
    prelude::H256,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Bytes, Filter, Log, TransactionRequest, U256},
};
use ethers_contract_derive::EthEvent;
use lazy_static::lazy_static;
use std::convert::TryFrom;
use std::env;

use dotenv::dotenv;
use std::{
    fs::File,
    io::{Cursor, Read, Seek},
    ops::{Add, Div, Mul, Sub},
};
const WORD: usize = 32;

// Load the Banyan Contract ABI into Memory
// IMPORTANT: The ABI must be updated if the contract is updated
lazy_static! {
    // IMPORTANT: This is a reference to a Test Contract's ABI
    // TODO: Change to the real contract's ABI, and update onChainDealInfo
    // Contract Address: 0x7Da936F4A55D5044e1838Cc959935085662392F1
    static ref BANYAN_ABI_STR_REF: &'static str = include_str!("../abi/jonah_test.json");
}

/// The Event emitted by the Banyan Contract when a Deal is submitted
#[derive(Clone, Debug, Copy, EthEvent)]
struct NewOffer {
    #[ethevent(indexed)]
    pub creator: Address,
    #[ethevent(indexed)]
    pub executor: Address,
    pub offer_id: U256,
}

/// EthClient - Everything needed to interact with Banyan's Ethereum Stack
pub struct EthClient {
    /// An Eth Provider. This is required to interact with the Ethereum Blockchain.
    provider: Provider<Http>,
    /// The chain ID of the network we're connected to. This is Required for signing transactions.
    chain_id: u64,
    /// An (optional) Eth Signer for singing transactions. This is required for interacting with payable functions.
    signer: Option<SignerMiddleware<Provider<Http>, LocalWallet>>,
    /// A Deployed Solidity Contract Address. This is required to interact with the Banyan Contract.
    contract: Contract<Provider<Http>>,
}

impl Default for EthClient {
    /// Build a new EthClient from the environment
    // TODO kind sweet error handling
    fn default() -> Self {
        dotenv().ok();
        dbg!("Initializing EthClient from environment");
        // Read the Api Url from the environment. Default to the mainnet Infura API
        let api_url = env::var("ETH_API_URL")
            .unwrap_or_else(|_| "https://mainnet.infura.io/v3/".parse().unwrap());
        // Read the Api Key from the environment. Raise an error if it is not set
        let api_key = env::var("ETH_API_KEY").expect("ETH_API_KEY must be set");
        // Try and Read the Chain ID from the environment. Default to 1 (mainnet)
        let chain_id = env::var("ETH_CHAIN_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .ok();
        // Try and Read the Private Key from the environment. Default to None
        // TODO also this is dangerous!!!! should not store privkey in env!!!
        let private_key = env::var("ETH_PRIVATE_KEY").ok();
        // Read the Contract Address from the environment
        // TODO: Explicit Error Raise on Unparsable Address
        let contract_address: Address = (env::var("ETH_CONTRACT_ADDRESS")
            .expect("ETH_CONTRACT_ADDRESS must be set"))
        .parse()
        .expect("ETH_CONTRACT_ADDRESS must be a valid Ethereum Address");
        EthClient::new(api_url, api_key, chain_id, private_key, contract_address).unwrap()
    }
}

// TODO: Update docs
/// The EthProvider is a wrapper around the ethers-rs Provider that handles all Ethereum
/// interactions.
impl EthClient {
    /// Create a new EthClient - Uses EthClientBuilder::new()
    /// # Arguments
    /// * `api_url` - The URL of the Ethereum API to connect to. This is required to interact with
    ///                 the Ethereum Blockchain.
    /// * `api_key` - The API Key for the Ethereum API. This is required.
    /// * `chain_id` - The (Optional) Chain ID of the network we're connected to.
    ///                 Defaults to 1 (mainnet)
    /// * `private_key` - The (Optional) Private Key for the Ethereum Account we're using to sign.
    ///                 This is required for interacting with payable functions.
    /// * `contract_address` - The (Optional) Deployed Solidity Contract Address to interact with.
    /// // * `timeout` - The (Optional) Timeout for the Eth Client. 15 seconds by default.
    /// ```no_run
    /// use banyan_shared::eth::EthClient;
    /// use ethers::types::Address;
    ///
    /// let eth_client = EthClient::new(
    ///    "https://mainnet.infura.io/v3/".to_string(),
    ///   "API_KEY".to_string(),
    ///    Some(1),
    ///    Some("PRIVATE_KEY".to_string()),
    ///    "CONTRACT_ADDRESS".parse::<Address>().unwrap(),
    ///    // Some(10),
    /// ).unwrap();
    /// ```
    /// # Panics
    /// * If the API URL is invalid
    pub fn new(
        api_url: String,
        api_key: String,
        chain_id: Option<u64>,
        private_key: Option<String>,
        contract_address: Address,
        //timeout: Option<u64>,
    ) -> Result<EthClient, Error> {
        // Determine an API URL and Initialize the Provider
        let url = format!("{}{}", api_url, api_key);
        let provider = Provider::<Http>::try_from(url).expect("Failed to create provider");

        // Get the Chain ID. If None, set to 1
        let chain_id = chain_id.unwrap_or(1);

        // Check if we have a private key to set up a Signer
        let signer = if let Some(private_key) = &private_key {
            let wallet = private_key
                .parse::<LocalWallet>()
                .expect("Failed to parse private key");
            Some(SignerMiddleware::new(
                provider.clone(),
                wallet.with_chain_id(chain_id),
            ))
        } else {
            None
        };

        // Check if we have a contract address to set up a Contract
        let abi: Abi = serde_json::from_str(&BANYAN_ABI_STR_REF).expect("Failed to parse ABI");
        let contract = Contract::new(contract_address, abi, provider.clone());

        // Determine the timeout as a Duration in seconds, assign default if not provided
        // let timeout = Duration::from_secs(timeout.unwrap_or(15));
        Ok(EthClient {
            provider,
            chain_id,
            signer,
            contract,
            //timeout,
        })
    }

    /* Struct State Methods */

    /// Return whether theres's a signer configured
    pub fn has_signer(&self) -> bool {
        self.signer.is_some()
    }

    /* Banyan Functions */

    /* Deal Stuff */

    // TODO: Do we want to add optional event listening?
    /// Propose a Deal to the Banyan Contract
    /// # Arguments
    /// * `deal` - The DealProposal to submit a proposal for
    /// * 'gas_limit` - An (Optional) Gas Limit for the transaction
    /// * `gas_price` - An (Optional) Gas Price for the transaction
    /// ```no_run
    /// use banyan_shared::eth::EthClient;
    /// use banyan_shared::deals::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let file = std::fs::File::open("./abi/escrow.json").unwrap();
    ///     let client = EthClient::default();
    ///     let deal = DealProposalBuilder::default()
    ///         .with_file(file)
    ///         .build()
    ///         .unwrap();
    ///     let deal_id = client.propose_deal(deal, None, None).await.unwrap();
    /// }
    /// ```
    /// # Panics
    /// * If the Deal Proposal is invalid
    /// * If the client is not configured with a signer
    pub async fn propose_deal(
        &self,
        deal: DealProposal,
        gas_limit: Option<u64>,
        gas_price: Option<u64>,
    ) -> Result<DealID, Error> {
        // TODO: Implement a general purpose wrapper for payable functions
        if !self.has_signer() {
            return Err(anyhow!("No signer available"));
        }
        // Borrow our signer and contract
        let signer = self.signer.as_ref().unwrap();
        // Create a new deal proposal Transaction
        let data = self.contract.encode("startOffer", deal)?;
        let tx = TransactionRequest::new()
            .to(self.contract.address())
            .data(data)
            .gas(gas_limit.unwrap_or(1_000_000u64)) // 3 million gas
            .gas_price(gas_price.unwrap_or(80_000_000_000u64)) // 70 Gwei
            .chain_id(self.chain_id);
        // Sign the transaction and listen for the event
        let pending_tx = match signer.send_transaction(tx, None).await {
            Ok(tx) => tx,
            Err(e) => {
                return Err(anyhow!("Error signing transaction: {}", &e.to_string()));
            }
        };
        let receipt = pending_tx.await?;
        let tx_hash = receipt.as_ref().unwrap().transaction_hash;
        let bn = receipt.as_ref().unwrap().block_number.unwrap();
        // TODO: More sophisticated Filter
        let logs: Vec<NewOffer> = match self.contract.event().from_block(bn).query().await {
            Ok(logs) => logs,
            Err(e) => {
                return Err(anyhow!(
                    "Error listening for transaction ({:?}), logs: {:?} ",
                    &tx_hash,
                    &e
                ))
            }
        };
        let log = logs.first().ok_or_else(|| anyhow!("No logs found"))?;
        Ok(DealID(log.offer_id.as_u64()))
    }

    /// get_offer - get a deal from the Ethereum blockchain by its on-chain ID
    /// # Arguments
    /// * `deal_id` - The Deal ID to get
    /// # Returns
    /// * `Deal` - The on chain Deal
    pub async fn get_offer(&self, deal_id: DealID) -> Result<OnChainDealInfo, Error> {
        Ok(self
            .contract
            .method::<_, OnChainDealInfo>("getOffer", deal_id)?
            .call()
            .await?)
    }

    /* Proof Stuff */

    // TODO the validator should be able to handle if proofs get sent twice on accident
    // return the block number that the proof made it into.

    /// post_proof - post a proof to the Ethereum blockchain
    /// # Arguments
    /// * `deal_id` - The Deal ID to post a proof for
    /// * `bao_proof_data` - The BAO Proof Data to post
    /// * `target_block_start` - The target block start for the proof
    /// * `gas_limit` - An (Optional) Gas Limit for the transaction
    /// * `gas_price` - An (Optional) Gas Price for the transaction
    /// # Returns
    /// * `BlockNum` - The block number that the proof was posted in
    pub async fn post_proof(
        &self,
        deal_id: DealID,
        bao_proof_data: Bytes,
        target_block_start: BlockNum,
        gas_limit: Option<u64>,
        gas_price: Option<u64>,
    ) -> Result<BlockNum> {
        if !self.has_signer() {
            return Err(anyhow!("No signer available"));
        }
        dbg!("Posting for deal: {:?}", deal_id.0);
        // Borrow our signer and contract
        let signer = self.signer.as_ref().unwrap();
        // Create a new proof
        dbg!("Initializing new Proof Request");
        let proof: Proof = Proof {
            bao_proof_data,
            deal_id,
            target_block_start,
        };
        let data = self.contract.encode("saveProof", proof)?;
        let tx = TransactionRequest::new()
            .to(self.contract.address())
            .data(data)
            .gas(gas_limit.unwrap_or(1_000_000u64)) // 3 million gas
            .gas_price(gas_price.unwrap_or(70_000_000_000u64)) // 70 Gwei
            .chain_id(self.chain_id);
        // Sign the transaction and listen for the event
        dbg!("Signing Proof");
        // Attempt to sign the transaction and log any errors
        let pending_tx = match signer.send_transaction(tx, None).await {
            Ok(tx) => tx,
            Err(e) => {
                return Err(anyhow!("Error signing transaction: {}", &e.to_string()));
            }
        };
        let receipt = pending_tx.await?;
        let tx_hash = receipt.as_ref().unwrap().transaction_hash;
        dbg!("Trxn Hash: {:?}", &tx_hash);
        let bn = receipt.as_ref().unwrap().block_number.unwrap();
        dbg!("Block Number: {:?}", &bn);

        Ok(BlockNum(bn.as_u64()))
    }

    pub async fn accept_deal_on_chain(&self) -> Result<OnChainDealInfo> {
        unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
    }

    /* Chain Primitives */

    /// Get the current block number
    pub async fn get_latest_block_num(&self) -> Result<BlockNum> {
        Ok(BlockNum(self.provider.get_block_number().await?.as_u64()))
    }

    /// Get the current transaction count
    pub async fn get_current_transaction_count(&self) -> Result<u64> {
        let signer = self.signer.as_ref().unwrap();
        let address = signer.address();
        Ok(self
            .provider
            .get_transaction_count(address, None)
            .await?
            .as_u64())
    }

    /// Get the current block hash for a given block number
    pub async fn get_block_hash_from_num(&self, block_number: BlockNum) -> Result<H256> {
        let block = self
            .provider
            .get_block(block_number.0)
            .await?
            .ok_or_else(|| anyhow!("block not found"))?;
        block.hash.ok_or_else(|| anyhow!("block hash not found"))
    }

    /// Get ethereum logs given a filter
    pub async fn get_logs_from_filter(&self, filter: Filter) -> Result<Vec<Log>> {
        Ok(self.provider.get_logs(&filter).await?)
    }

    /// Get the block number a proof was logged in given the deal id and window number of that proof
    pub async fn get_proof_block_num_from_window(
        &self,
        deal_id: DealID,
        window_num: u64,
    ) -> Result<Option<BlockNum>> {
        let block_num = self
            .contract
            .method::<_, U256>("getProofBlock", (deal_id.0, window_num))?
            .call()
            .await?
            .as_u64();
        if block_num == 0 {
            Ok(None)
        } else {
            Ok(Some(BlockNum(block_num)))
        }
    }

    /// Get the address of a contract
    pub async fn get_contract_address(&self) -> Result<Address> {
        Ok(self.contract.address())
    }

    /// Get the proof data from ethereum logs given a block number and deal id (the topic!)
    /// # Arguments
    /// * `submitted_proof_in_block_num` - The block number the proof was submitted in
    /// * `deal_id` - The deal id of the proof
    pub async fn get_proof_from_logs(
        &self,
        submitted_proof_in_block_num: BlockNum,
        deal_id: DealID,
    ) -> Result<Option<Vec<u8>>> {
        let address = self.contract.address();

        let filter = Filter::new()
            .select(submitted_proof_in_block_num.0)
            // TODO figure this guy out later :)
            .address(address)
            .topic1(H256::from_low_u64_be(deal_id.0));

        let block_logs = self.get_logs_from_filter(filter).await?;

        // The first two 32 byte words of log data are a pointer and the size of the data.
        // TODO put this in banyan_shared!
        let data = &block_logs[0].data;
        if data.len() < WORD * 2 {
            return Ok(None);
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
        let data_size = u64::from_be_bytes(a);
        dbg!("data_size: {:?}", data_size);
        dbg!("data.len(): {:?}", data.len());
        if data.len() < WORD * 2 + data_size as usize {
            return Ok(None);
        }
        let data_bytes: Vec<u8> = (&data[WORD * 2..WORD * 2 + data_size as usize]).to_vec();
        Ok(Some(data_bytes))
    }

    /// Given a merkle proof, and the proper blake3 checksum, offset, and chunk size, check if the proof is valid
    /// # Arguments
    /// * `proof_bytes` - The merkle proof bytes
    /// * `blake3_checksum` - The blake3 hash of the data
    /// * `chunk_offset` - The offset of the chunk in the data
    /// * `chunk_size` - The size of the chunk in the data
    pub fn check_if_merkle_proof_is_valid(
        proof_bytes: Cursor<&Vec<u8>>,
        blake3_checksum: bao::Hash,
        chunk_offset: u64,
        chunk_size: u64,
    ) -> Result<bool> {
        Ok(bao::decode::SliceDecoder::new(
            proof_bytes,
            &(blake3_checksum),
            chunk_offset,
            chunk_size,
        )
        .read_to_end(&mut vec![])
        .is_ok())
    }

    /// Computes the target block number for a given window number, deal start block, and proof frequency. The API validaator uses
    /// this to determine the target_block, which it then uses to get the block hash, and then calls compute_random_block_choice_from_hash(...)
    /// to compute the correct chunk offset and size.
    pub fn compute_target_block_start(
        deal_start_block: BlockNum,
        proof_frequency_in_blocks: BlockNum,
        target_window_num: usize,
    ) -> BlockNum {
        Add::add(
            Mul::mul(proof_frequency_in_blocks, target_window_num),
            deal_start_block,
        )
    }

    /* Function to check if the deal is over or not */
    pub fn deal_over(current_block_num: BlockNum, deal_info: OnChainDealInfo) -> bool {
        current_block_num > Add::add(deal_info.deal_start_block, deal_info.deal_length_in_blocks)
    }

    // Below are a range of functions that help with our testing framework

    /// Helper for computing file length
    pub fn file_len(file_name: &str) -> usize {
        let mut file_content = Vec::new();
        let mut file = File::open(&file_name).expect("Unable to open file");
        file.read_to_end(&mut file_content).expect("Unable to read");
        file_content.len()
    }

    /// Helper for testing functions that create proofs
    /// # Arguments
    /// * `target_window_start` - The block number used to generate the chunk offset and chunk size
    /// * `file` - The file to generate the proof from
    /// * `file_length` - The length of the file
    /// * `quality` - Whether or not the proof is correct or incorrect
    pub async fn create_proof_helper(
        &self,
        target_window_start: BlockNum,
        file: &mut File,
        file_length: u64,
        quality: bool,
    ) -> Result<(bao::Hash, Bytes)> {
        file.rewind()?;
        let target_block_hash = self.get_block_hash_from_num(target_window_start).await?;
        let (obao_file, hash) = proofs::gen_obao(file)?;
        let obao_cursor = Cursor::new(obao_file);
        let mut slice: Vec<u8> = gen_proof(
            target_window_start,
            target_block_hash,
            file,
            obao_cursor,
            file_length,
        )
        .await
        .unwrap();

        if !quality {
            let last_index = slice.len() - 1;
            slice[last_index] ^= 1;
        }
        Ok((hash, Bytes::from(slice)))
    }

    /// Helper for testing functions that determines what window the current window for a deal
    /// # Arguments
    /// * `deal_start_block` - The block number that the deal started at
    /// * `proof_frequency_in_blocks` - The frequency at which proofs are submitted in the deal
    pub async fn compute_target_window(
        &self,
        deal_start_block: BlockNum,
        proof_frequency_in_blocks: BlockNum,
    ) -> Result<usize> {
        let current_block_num = self.get_latest_block_num().await?;
        let offset: BlockNum = Sub::sub(current_block_num, deal_start_block);
        //assert!(offset < deal_length_in_blocks);
        //assert_eq!(Rem::rem(offset, proof_frequency_in_blocks), BlockNum(0));
        let window_num = Div::div(offset, proof_frequency_in_blocks);
        Ok(usize::try_from(window_num.0)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    /// Test Init a new eth client from the environment.
    /// The environment variables for all fields must be set for this test to pass
    async fn eth_client_new() -> Result<(), anyhow::Error> {
        // Init a new EthClient with our environment variables
        let eth_client = EthClient::default();
        if !eth_client.has_signer() {
            panic!("No signer available!");
        }
        // Try and get the current block number
        let block_num: BlockNum = eth_client.get_latest_block_num().await?;
        println!("Latest Block Number: {}", block_num.0);
        Ok(())
    }

    #[tokio::test]
    /// Test sending a deal Proposal
    async fn send_deal_proposal() -> Result<(), anyhow::Error> {
        use crate::deals::*;
        // Open a file to build our DealProposal
        let file = std::fs::File::open("./abi/escrow.json").unwrap();
        // Build a DealProposal from the file
        let dp = DealProposalBuilder::default()
            .with_file(file)
            .build()
            .unwrap();
        // Init a new EthClient with our environment variables
        let eth_client = EthClient::default();
        // Send the DealProposal
        let deal_id: DealID = eth_client
            .propose_deal(dp, None, None)
            .await
            .expect("Failed to send deal proposal");
        // Read the deal from the contract
        let deal = eth_client.get_offer(deal_id).await.unwrap();
        // Assert that the deal we read is the same as the one we sent
        assert_eq!(deal.deal_length_in_blocks, BlockNum(10));
        Ok(())
    }

    #[tokio::test]
    async fn post_proof_to_chain() -> Result<(), anyhow::Error> {
        let mut file = File::open("../Rust-Chainlink-EA-API/test_files/ethereum.pdf").unwrap();
        let eth_client = EthClient::default();

        let deal_id = DealID(1);
        let deal = eth_client.get_offer(deal_id).await.unwrap();

        let target_window: usize = eth_client
            .compute_target_window(deal.deal_start_block, deal.proof_frequency_in_blocks)
            .await
            .expect("Failed to compute target window");

        let target_block = EthClient::compute_target_block_start(
            deal.deal_start_block,
            deal.proof_frequency_in_blocks,
            target_window,
        );
        // create a proof using the same file we used to create the deal
        let (_hash, proof) = eth_client
            .create_proof_helper(target_block, &mut file, deal.file_size.as_u64(), true)
            .await
            .expect("Failed to create proof");

        let block_num: BlockNum = eth_client
            .post_proof(deal_id, proof, target_block, None, None)
            .await
            .expect("Failed to post proof");

        let proof_bytes: Vec<u8> = match eth_client.get_proof_from_logs(block_num, deal_id).await? {
            Some(proof) => proof,
            None => {
                panic!("Failed to get proof from logs");
            }
        };

        assert_eq!(proof_bytes.len(), 1672);
        Ok(())
    }

    #[tokio::test]
    async fn check_good_proof() -> Result<(), anyhow::Error> {
        dotenv().ok();
        let mut file = File::open("../Rust-Chainlink-EA-API/test_files/ethereum.pdf").unwrap();
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
        // create a proof using the same file we used to create the deal
        let (hash, proof) = eth_client
            .create_proof_helper(target_block, &mut file, deal.file_size.as_u64(), true)
            .await
            .expect("Failed to create proof");

        let target_block_hash = eth_client.get_block_hash_from_num(target_block).await?;
        let (chunk_offset, chunk_size) = proofs::compute_random_block_choice_from_hash(
            target_block_hash,
            deal.file_size.as_u64(),
        );

        let proof_vec = proof.to_vec();
        assert_eq!(
            true,
            EthClient::check_if_merkle_proof_is_valid(
                Cursor::new(&proof_vec),
                hash,
                chunk_offset,
                chunk_size,
            )?
        );
        Ok(())
    }

    #[tokio::test]
    async fn check_bad_proof() -> Result<(), anyhow::Error> {
        dotenv().ok();
        let mut file = File::open("../Rust-Chainlink-EA-API/test_files/ethereum.pdf").unwrap();
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
        // create a proof using the same file we used to create the deal
        let (hash, proof) = eth_client
            .create_proof_helper(target_block, &mut file, deal.file_size.as_u64(), false)
            .await
            .expect("Failed to create proof");

        let target_block_hash = eth_client.get_block_hash_from_num(target_block).await?;
        let (chunk_offset, chunk_size) = proofs::compute_random_block_choice_from_hash(
            target_block_hash,
            deal.file_size.as_u64(),
        );

        let proof_vec = proof.to_vec();
        assert_eq!(
            false,
            EthClient::check_if_merkle_proof_is_valid(
                Cursor::new(&proof_vec),
                hash,
                chunk_offset,
                chunk_size,
            )?
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn oh_my_god_this_needs_tests() {
        unimplemented!("do it");
    }
}
