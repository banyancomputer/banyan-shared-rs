use crate::types::*;
use anyhow::{anyhow, Error, Result};
use ethers::{
    abi::Abi,
    contract::Contract,
    middleware::SignerMiddleware,
    prelude::H256,
    providers::{JsonRpcClient, Middleware, Provider},
    signers::Signer,
    types::{Address, Filter, Log, TransactionRequest, U256},
};
use ethers_contract_derive::EthEvent;
use lazy_static::lazy_static;
use std::sync::Arc;

// Load the Banyan Contract ABI into Memory
// IMPORTANT: The ABI must be updated if the contract is updated
lazy_static! {
    static ref BANYAN_ABI_STR_REF: &'static str = include_str!("../abi/Escrow.json");
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
pub struct EthClient<P: JsonRpcClient> {
    provider: Arc<Provider<P>>,
    /// The chain ID of the network we're connected to. This is Required for signing transactions.
    chain_id: u64,
    /// A Deployed Solidity Contract Address. This is required to interact with the Banyan Contract.
    contract: Contract<Arc<Provider<P>>>,
}

// TODO: Update docs
/// The EthProvider is a wrapper around the ethers-rs Provider that handles all Ethereum
/// interactions.
impl<P: JsonRpcClient + 'static> EthClient<P> {
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
    /// use std::str::FromStr;
    /// use ethers::providers::{Provider, Http};
    ///
    /// let provider = Provider::<Http>::try_from("https://mainnet.infura.io/v3/",)
    ///     .expect("could not instantiate HTTP Provider");
    /// let contract_addr =
    ///     Address::from_str("0x0000000000000000000000000000000000000000").unwrap();
    /// // Init a new EthClient with our environment variables
    /// let eth_client = EthClient::new(provider, 1, contract_addr).unwrap();
    /// ```
    /// # Panics
    /// * If the API URL is invalid
    pub fn new(
        provider: Provider<P>,
        chain_id: u64,
        contract_address: Address,
        //timeout: Option<u64>,
    ) -> Result<EthClient<P>, Error> {
        // // Check if we have a private key to set up a Signer
        // let signer = if let Some(private_key) = &private_key {
        //     let wallet = private_key
        //         .parse::<LocalWallet>()
        //         .expect("Failed to parse private key");
        //     Some(SignerMiddleware::new(
        //         provider.clone(),
        //         wallet.with_chain_id(chain_id),
        //     ))
        // } else {
        //     None
        // };

        let provider = Arc::new(provider);
        // Check if we have a contract address to set up a Contract
        let abi: Abi = serde_json::from_str(&BANYAN_ABI_STR_REF).expect("Failed to parse ABI");
        let contract = Contract::new(contract_address, abi, provider.clone());

        // Determine the timeout as a Duration in seconds, assign default if not provided
        // let timeout = Duration::from_secs(timeout.unwrap_or(15));
        Ok(EthClient {
            provider,
            chain_id,
            contract,
        })
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
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    /// use ethers::providers::{Provider, Http};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let file = std::fs::File::open("./abi/escrow.json").unwrap();
    ///
    ///     let provider = Provider::<Http>::try_from("https://mainnet.infura.io/v3/",)
    ///         .expect("could not instantiate HTTP Provider");
    ///     let contract_addr =
    ///         Address::from_str("0x0000000000000000000000000000000000000000").unwrap();
    ///     // Init a new EthClient with our environment variables
    ///     let eth_client = EthClient::new(provider, 1, contract_addr).unwrap();
    ///     let deal = DealProposalBuilder::default()
    ///         .with_file(file)
    ///         .build()
    ///         .unwrap();
    ///     let deal_id = client.propose_deal(deal, None, None).await.unwrap();
    /// }
    /// ```
    /// # Panics
    /// * If the Deal Proposal is invalid
    pub async fn propose_deal<S: Signer>(
        &self,
        deal: DealProposal,
        gas_limit: Option<u64>,
        gas_price: Option<u64>,
        signer: S,
    ) -> Result<DealID, Error> {
        let signer_middleware = SignerMiddleware::new(self.provider.clone(), signer);
        // Create a new deal proposal Transaction
        let data = self.contract.encode("startOffer", deal)?;
        let tx = TransactionRequest::new()
            .to(self.contract.address())
            .data(data)
            // TODO fix how we're doing gas
            .gas(gas_limit.unwrap_or(3_000_000u64)) // 3 million Wei
            .gas_price(gas_price.unwrap_or(70_000_000_000u64)) // 70 Gwei
            .chain_id(self.chain_id);
        // Sign the transaction and listen for the event
        let pending_tx = match signer_middleware.send_transaction(tx, None).await {
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

    /// get_deal - get a deal from the Ethereum blockchain by its on-chain ID
    /// # Arguments
    /// * `deal_id` - The Deal ID to get
    /// # Returns
    /// * `Deal` - The on chain Deal
    pub async fn get_deal(&self, deal_id: DealID) -> Result<OnChainDealInfo, Error> {
        Ok(self
            .contract
            .method::<_, OnChainDealInfo>("getOffer", deal_id)?.call().await?)
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
        let block = self
            .provider
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
    use ethers::core::rand::thread_rng;
    use ethers::prelude::Http;
    use ethers::prelude::LocalWallet;

    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    /// Test Init a new eth client from the environment.
    /// The environment variables for all fields must be set for this test to pass
    async fn eth_client_new() {
        // make a new provider
        let provider = Provider::<Http>::try_from(
            "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27",
        )
        .expect("could not instantiate HTTP Provider");
        let contract_addr =
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap();
        // Init a new EthClient with our environment variables
        let eth_client = EthClient::new(provider, 1, contract_addr).unwrap();
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
        let dp = DealProposalBuilder::default()
            .with_file(file)
            .build()
            .unwrap();
        // make a new provider
        let provider = Provider::<Http>::try_from("https://mainnet.infura.io/v3/")
            .expect("could not instantiate HTTP Provider");
        let contract_addr =
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap();
        // Init a new EthClient with our environment variables
        let eth_client = EthClient::new(provider, 1, contract_addr).unwrap();
        let wallet = LocalWallet::new(&mut thread_rng());
        // Send the DealProposal
        let deal_id: DealID = eth_client
            .propose_deal(dp, None, None, wallet)
            .await
            .expect("Failed to send deal proposal");
        // Read the deal from the contract
        let _deal = eth_client.get_deal(deal_id).await.unwrap();
        // Assert that the deal we read is the same as the one we sent
        //assert_eq!(dp, deal);
        unimplemented!("write me :)")
    }
}
