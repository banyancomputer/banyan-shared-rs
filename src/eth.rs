use crate::types::*;
use anyhow::{anyhow, Error, Result};
use ethers::{
    abi::Abi,
    contract::Contract,
    middleware::SignerMiddleware,
    prelude::H256,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Filter, Log, TransactionRequest, U256},
};
use ethers_contract_derive::EthEvent;
use lazy_static::lazy_static;
use std::convert::TryFrom;
use std::env;

// Load the Banyan Contract ABI into Memory
// Important: The ABI must be updated if the contract is updated
lazy_static! {
    // IMPORTANT: This is a reference to a Test Contract's ABI
    // TODO: Change to the real contract's ABI, and update onChainDealInfo
    // Contract Address: 0x7Da936F4A55D5044e1838Cc959935085662392F1
    static ref BANYAN_ABI_STR_REF: &'static str = include_str!("../abi/test.json");
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
        dbg!("Initializing EthClient from environment");
        // Read the Api Url from the environment. Default to the mainnet Infura API
        let api_url = env::var("ETH_API_URL")
            .unwrap_or_else(|_| "https://mainnet.infura.io/v3/".parse().unwrap());
        dbg!(format!("  API_URL: {}", &api_url));
        // Read the Api Key from the environment. Raise an error if it is not set
        let api_key = env::var("ETH_API_KEY").expect("ETH_API_KEY must be set");
        dbg!(format!("  API_KEY: {}", &api_key));
        // Try and Read the Chain ID from the environment. Default to 1 (mainnet)
        let chain_id = env::var("ETH_CHAIN_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .ok();
        dbg!(format!("  CHAIN_ID: {:?}", &chain_id));
        // Try and Read the Private Key from the environment. Default to None
        // TODO bugfix this silently fails if the private key is malformatted :|
        // TODO also this is dangerous!!!! should not store privkey in env!!!
        let private_key = env::var("ETH_PRIVATE_KEY").ok();
        dbg!(format!("  PRIVATE_KEY: {:?}", &private_key));
        // Read the Contract Address from the environment
        let contract_address: Address = (env::var("ETH_CONTRACT_ADDRESS")
            .expect("ETH_CONTRACT_ADDRESS must be set"))
        .parse()
        .expect("ETH_CONTRACT_ADDRESS must be a valid Ethereum Address");
        dbg!(format!("  CONTRACT_ADDRESS: {:?}", &contract_address));
        EthClient::new(api_url, api_key, chain_id, private_key, contract_address).unwrap()
    }
}

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
    /// ```
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

    /// Submit a Deal to the Banyan Contract
    /// # Arguments
    /// * `deal` - The Deal to submit a proposal for
    /// * 'gas_limit` - An (Optional) Gas Limit for the transaction
    /// * `gas_price` - An (Optional) Gas Price for the transaction
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
        // Create a new deal
        dbg!("Initializing new Trxn Request");
        let data = self.contract.encode("startOffer", deal)?;
        // TODO: Configurable Gas
        // TODO: Implement Timeout
        let tx = TransactionRequest::new()
            .to(self.contract.address())
            .data(data)
            .gas(gas_limit.unwrap_or(3_000_000u64)) // 3 million gas
            .gas_price(gas_price.unwrap_or(70_000_000_000u64)) // 70 Gwei
            .chain_id(self.chain_id);
        // Sign the transaction and listen for the event
        dbg!("Signing Request");
        // Attempt to sign the transaction and log any errors
        let pending_tx = match signer.send_transaction(tx, None).await {
            Ok(tx) => tx,
            Err(e) => {
                return Err(anyhow!("Error signing transaction: {}", &e.to_string()));
            }
        };
        // let pending_tx = signer.send_transaction(tx, None).await?;
        let receipt = pending_tx.await?;
        let tx_hash = receipt.as_ref().unwrap().transaction_hash;
        dbg!("Trxn Hash: {:?}", &tx_hash);
        let bn = receipt.as_ref().unwrap().block_number.unwrap();
        dbg!("Block Number: {:?}", &bn);
        let logs: Vec<NewOffer> = match self.contract.event().from_block(bn).query().await {
            Ok(logs) => logs,
            Err(e) => return Err(anyhow!("Error listening for transaction logs: {:?} ", &e)),
        };
        dbg!("Logs: ", &logs);
        let log = logs.first().ok_or_else(|| anyhow!("No logs found"))?;
        dbg!("Log: {:?}", &log);
        Ok(DealID(log.offer_id.as_u64()))
    }

    /// get_deal - get a deal from the Ethereum blockchain by its on-chain ID
    /// # Arguments
    /// * `deal_id` - The Deal ID to get
    /// # Returns
    /// * `Deal` - The on chain Deal
    pub async fn get_deal(&self, deal_id: DealID) -> Result<OnChainDealInfo, Error> {
        Ok(self
            .contract
            .method::<_, OnChainDealInfo>("getOffer", deal_id)?
            .call()
            .await?)
    }

    /* Proof Stuff */

    // the validator should be able to handle if proofs get sent twice on accident
    // return the block number that the proof made it into.
    pub async fn post_proof(&self, _deal_id: &DealID, _proof: Proof) -> Result<BlockNum> {
        unimplemented!("write me :)")
    }

    pub async fn accept_deal_on_chain(&self) -> Result<OnChainDealInfo> {
        unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
    }

    /* Chain Primitives */

    /// Get the current block number
    pub async fn get_latest_block_num(&self) -> Result<BlockNum> {
        Ok(BlockNum(self.provider.get_block_number().await?.as_u64()))
    }

    /// Get the current block hash for a given block number
    pub async fn get_block_hash_from_num(&self, block_number: BlockNum) -> Result<H256> {
        let block = self.provider
            .get_block(block_number.0)
            .await?
            .ok_or_else(|| anyhow!("block not found"))?;
        block.hash.ok_or_else(|| anyhow!("block hash not found"))
    }

    pub async fn get_logs_from_filter(&self, filter: Filter) -> Result<Vec<Log>> {
        Ok(self.provider.get_logs(&filter).await?)
    }

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
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    /// Test Init a new eth client from the environment.
    /// The environment variables for all fields must be set for this test to pass
    async fn eth_client_new() {
        // Init a new EthClient with our environment variables
        let eth_client = EthClient::default();
        if !eth_client.has_signer() {
            panic!("No signer available!");
        }
        // Try and get the current block number
        let block_num = eth_client.get_latest_block_num().await.unwrap();
        println!("Latest Block Number: {}", block_num.0);
    }

    #[tokio::test]
    /// Test sending a deal Proposal
    async fn send_deal_proposal() {
        use crate::deals::*;
        // Open a file to build our DealProposal
        let file = std::fs::File::open("./abi/escrow.json").unwrap();
        // Build a DealProposal from the file
        let dp = DealProposalBuilder::default().build(&file).unwrap();
        // Init a new EthClient with our environment variables
        let eth_client = EthClient::default();
        // Send the DealProposal
        let deal_id: DealID = eth_client
            .propose_deal(dp, None, None)
            .await
            .expect("Failed to send deal proposal");
        // Read the deal from the contract
        let deal = eth_client.get_deal(deal_id).await.unwrap();
        // Assert that the deal we read is the same as the one we sent
        assert_eq!(deal.deal_id, deal_id);
    }
}
