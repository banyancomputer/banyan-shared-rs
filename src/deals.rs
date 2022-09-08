use crate::{
    hash::FileHasher,
    types::{Blake3HashToken, BlockNum, CidToken, DealProposal, TokenAddress, TokenMultiplier},
};
use anyhow::{Error, Result};
use cid::Cid;
use ethers::{abi::Address, types::U256};

/* Implements the deal proposal struct. */

impl DealProposal {
    pub fn builder() -> DealProposalBuilder {
        DealProposalBuilder::default()
    }
}

/// DealProposalBuilder - A builder for a deal proposal
/// This struct handles Configuring and Building a DealProposal
pub struct DealProposalBuilder {
    /// The address of the executor to to propose the deal to, as a string
    pub executor_address: String,
    /// The length of the deal in blocks, as an int
    pub deal_length_in_blocks: u64,
    /// The frequency of proofs to be submitted, as an int
    pub proof_frequency_in_blocks: u64,
    /// The amount of tokens to be paid to the executor per TiB, as a float
    pub price_per_tib: f64,
    /// The amount of tokens in collateral the executor must provide per TiB, as a float
    pub collateral_per_tib: f64,
    /// The Address of the token to use as denominator for the price and collateral, as a string
    pub erc20_token_denomination: String,
}

impl Default for DealProposalBuilder {
    fn default() -> Self {
        DealProposalBuilder {
            // note (al): this is my address, we should change this to a real address
            executor_address: "0x2C231Fb9B59b56CdDD413443D90628384b3F1d60".to_string(),
            deal_length_in_blocks: 0,
            /// TODO: Call API to get a good value
            proof_frequency_in_blocks: 10,
            price_per_tib: 0.0,
            collateral_per_tib: 0.0,
            // The Gorli test eth address
            erc20_token_denomination: "0xD87Ba7A50B2E7E660f678A895E4B72E7CB4CCd9C".to_string(),
        }
    }
}

impl DealProposalBuilder {
    /// Build a DealProposalConfig from a set of primitive types
    ///
    /// # Arguments
    ///
    /// * `executor_address` - The address of the executor to to propose the deal to, as a string
    /// * `deal_length_in_blocks` - The length of the deal in blocks, as an int
    /// * `proof_frequency_in_blocks` - The frequency of proofs to be submitted, as an int
    /// * `price_per_tib` - The amount of tokens to be paid to the executor per TiB, as a float
    /// * `collateral_per_tib` - The amount of tokens in collateral the executor must provide per TiB, as a float
    /// * `erc20_token_denomination` - The Address of the token to use as denominator for the price and collateral, as a string
    ///
    /// # Returns
    ///
    /// * `DealProposalBuilder` - A DealProposalBuilder struct
    ///
    /// # Errors
    /// TODO: Add error handling
    pub fn new(
        executor_address: String,
        deal_length_in_blocks: u64,
        proof_frequency_in_blocks: u64,
        price_per_tib: f64,
        collateral_per_tib: f64,
        erc20_token_denomination: String,
    ) -> DealProposalBuilder {
        DealProposalBuilder {
            executor_address,
            deal_length_in_blocks,
            proof_frequency_in_blocks,
            price_per_tib,
            collateral_per_tib,
            erc20_token_denomination,
        }
    }

    /// Build a DealProposal from a DealProposalConfig
    ///
    /// # Arguments
    ///
    /// * `file` - The file the deal should be created for
    ///
    /// # Returns
    ///
    /// * `DealProposal` - The DealProposal
    ///
    /// # Errors
    /// TODO: Add Errors
    pub fn build(&self, file: &std::fs::File) -> Result<DealProposal, Error> {
        let _file_size = file.metadata().unwrap().len();
        let num_tib = _file_size as f64 / 1024.0 / 1024.0 / 1024.0 / 1024.0;
        /* Build the DealProposal */
        // parse the executor address as a Token\
        let executor_address = self.executor_address.parse::<Address>().unwrap();

        // Set the duration of the deal
        let deal_length_in_blocks = BlockNum(self.deal_length_in_blocks as u64);
        let proof_frequency_in_blocks = BlockNum(self.proof_frequency_in_blocks as u64);

        // Calculate the on-Chain price and collateral
        let token_multiplier = TokenMultiplier::default();
        let price = token_multiplier * (num_tib * self.price_per_tib);
        let collateral = token_multiplier * (num_tib * self.collateral_per_tib);
        let erc20_token_denomination = TokenAddress(self.erc20_token_denomination.parse().unwrap());

        let file_size = U256::from(_file_size);

        // Calculate the CID of the file using Sha2-256 and Multihash

        // TODO: fix this
        let mh = FileHasher::new(file).multihash().unwrap();
        let cid = Cid::new_v1(0x12, mh);
        let ipfs_file_cid = CidToken(cid);

        let blake3_checksum = Blake3HashToken(FileHasher::new(file).b3hash().unwrap());
        Ok(DealProposal {
            executor_address,
            deal_length_in_blocks,
            proof_frequency_in_blocks,
            price,
            collateral,
            erc20_token_denomination,
            file_size,
            ipfs_file_cid,
            blake3_checksum,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_build_deal_proposal() {
        // Important: Update the test if the file changes
        let file = File::open("abi/escrow.json").unwrap();
        let deal_proposal = DealProposal::builder().build(&file).unwrap();

        // Check the Cid is correct
        // Should be: bafkreigfb3m7aoajp42rafefephqg7kcrxezpqtz4tsqhnpkofelwc5l5e
        assert_eq!(
            deal_proposal.ipfs_file_cid.to_string(),
            "bafkreigfb3m7aoajp42rafefephqg7kcrxezpqtz4tsqhnpkofelwc5l5e"
        );
        // Check the Blake3 hash is correct
        // Should be: 4bdfe5f0ed92451b9a1a7cf979f538cc31e8440ac1de85d27fe3d5a207b01dd4
        assert_eq!(
            deal_proposal.blake3_checksum.to_hex().to_string(),
            "4bdfe5f0ed92451b9a1a7cf979f538cc31e8440ac1de85d27fe3d5a207b01dd4"
        );
    }
}
