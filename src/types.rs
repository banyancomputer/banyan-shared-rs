use blake3::Hash as B3Hash;
use cid::Cid;
use ethers::{
    abi::{InvalidOutputType, Token, Tokenizable, Tokenize},
    // TODO: Can we import this somewhere / do we need this?
    types::{Address, U256},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sled::IVec;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::{Add, Mul, Sub};

/// A Wrapper around the CID struct from the cid crate
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CidToken(pub Cid);

impl Display for CidToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl CidToken {
    pub fn cid(&self) -> Cid {
        self.0
    }
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

/// Impl Tokenizable for CidToken - This allows us to use CidToken as a Token in the ethers crate
impl Tokenizable for CidToken {
    /// Convert a Token::String to a CidToken
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::String(s) => Ok(CidToken(Cid::try_from(s).unwrap())),
            other => Err(InvalidOutputType(format!(
                "Expected `String`, got {:?}",
                other
            ))),
        }
    }
    /// Convert a CidToken to a Token::String
    fn into_token(self) -> Token {
        Token::String(self.0.to_string())
    }
}

impl Serialize for CidToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CidToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(CidToken(Cid::try_from(s).unwrap()))
    }
}

/// A Wrapper around the Hash struct from the bao crate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blake3HashToken(pub B3Hash);

impl Display for Blake3HashToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_hex())
    }
}

impl Blake3HashToken {
    /// Return the underlying bao::Hash
    pub fn hash(&self) -> B3Hash {
        self.0
    }
    /// Return the underlying bao::Hash as a Hex String
    pub fn to_hex(&self) -> String {
        self.0.to_hex().to_string()
    }
    /// Return the underlying blake3::Hash as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

/// Impl Tokenizable for Blake3HashToken - This allows us to use CidToken as a Token in the ethers crate
impl Tokenizable for Blake3HashToken {
    fn into_token(self) -> Token {
        Token::String(self.to_hex())
    }
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::String(s) => Ok(Blake3HashToken(B3Hash::from_hex(s).unwrap())),
            other => Err(InvalidOutputType(format!(
                "Expected `String`, got {:?}",
                other
            ))),
        }
    }
}

impl Serialize for Blake3HashToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.as_bytes())
    }
}

impl<'de> Deserialize<'de> for Blake3HashToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let b: [u8; 32] = <[u8; 32]>::deserialize(deserializer)?;
        Ok(Blake3HashToken(B3Hash::from(b)))
    }
}

/// DealIDs - The onChain ID of a deal submitted to Ethereum
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct DealID(pub u64);

impl DealID {
    /// Return the underlying u64 of the DealID
    pub fn id(&self) -> u64 {
        self.0
    }
}

impl Display for DealID {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.id())
    }
}

/// Imple Tokenizable for DealID - this allows us to treat it like a Token with with Ethers Crate
impl Tokenizable for DealID {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::Uint(u) => Ok(DealID(u.as_u64())),
            other => Err(InvalidOutputType(format!(
                "Expected `Token::Uint()`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::Uint(self.0.into())
    }
}

#[allow(clippy::from_over_into)]
impl Into<IVec> for DealID {
    fn into(self) -> IVec {
        IVec::from(&self.0.to_le_bytes())
    }
}

impl From<IVec> for DealID {
    fn from(iv: IVec) -> Self {
        let bytes = iv.as_ref();
        let mut deal_id_bytes = [0u8; 8];
        deal_id_bytes.copy_from_slice(&bytes[..8]);
        DealID(u64::from_le_bytes(deal_id_bytes))
    }
}

/// Block Number - a wrapper around u64 to specify an Ethereum block number
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct BlockNum(pub u64);

impl Display for BlockNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

/// Imple Tokenizable for BlockNum - this allows us to treat it like a Token with with Ethers Crate
impl Tokenizable for BlockNum {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::Uint(u) => Ok(BlockNum(u.as_u64())),
            other => Err(InvalidOutputType(format!(
                "Expected `Token::Uint()`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::Uint(self.0.into())
    }
}

impl Add for BlockNum {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        BlockNum(self.0 + other.0)
    }
}

impl Sub for BlockNum {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        BlockNum(self.0 - other.0)
    }
}

impl Mul<u64> for BlockNum {
    type Output = Self;
    fn mul(self, other: u64) -> Self {
        BlockNum(self.0 * other)
    }
}

impl Mul<usize> for BlockNum {
    type Output = Self;
    fn mul(self, other: usize) -> Self {
        let u = other as u64;
        BlockNum(self.0 * u)
    }
}

/// Token Multiplier - a wrapper around u64 to specify a multiplier for a token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TokenMultiplier(pub u64);

// Our default multiplier is 1e18
impl Default for TokenMultiplier {
    fn default() -> Self {
        TokenMultiplier(1_000_000_000_000_000_000)
    }
}

/// Multiply a TokenMultiplier as a float and return the result as U256
/// Warning: Non-Deterministic
impl Mul<f64> for TokenMultiplier {
    type Output = U256;
    fn mul(self, other: f64) -> U256 {
        let amount = (self.0 as f64 * other).round() as u64;
        if amount == 0 {
            U256::from(1) // This is the smallest a U256 can be
        } else {
            U256::from(amount)
        }
    }
}

/// An Enum describing the different states a deal can be in
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DealStatus {
    /// The deal does not exist
    Non = 0,
    /// The deal has been submitted to the chain, but not yet accepted
    DealCreated = 1,
    /// The deal has been accepted by the executor
    DealAccepted = 2,
    /// The deal is active
    DealActive = 3,
    /// The deal has been completed
    DealCompleted = 4,
    /// The deal has been finalized
    DealFinalized = 5,
    /// The deal was submitted to the chain, but not accepted
    DealTimedOut = 6,
    /// The deal was submitted to the chain, and then cancelled
    DealCancelled = 7,
}

/// Impl Tokenizable for DealStatus - this allows us to treat it like a Token with with Ethers Crate
impl Tokenizable for DealStatus {
    fn into_token(self) -> Token {
        Token::Uint(U256::from(self as u8))
    }
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::Uint(u) => match u.as_u64() {
                0 => Ok(DealStatus::Non),
                1 => Ok(DealStatus::DealCreated),
                2 => Ok(DealStatus::DealAccepted),
                3 => Ok(DealStatus::DealActive),
                4 => Ok(DealStatus::DealCompleted),
                5 => Ok(DealStatus::DealFinalized),
                6 => Ok(DealStatus::DealTimedOut),
                7 => Ok(DealStatus::DealCancelled),
                _ => Err(InvalidOutputType(format!(
                    "Expected `Token::Uint()`, got {:?}",
                    token
                ))),
            },
            other => Err(InvalidOutputType(format!(
                "Expected `Token::Uint()`, got {:?}",
                other
            ))),
        }
    }
}

/// DealProposal - What is submitted to the Ethereum contract to create a deal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DealProposal {
    /// The address of the party to propose the deal to
    pub executor_address: Address,
    /// The length of the deal in blocks
    pub deal_length_in_blocks: BlockNum,
    /// The frequency with which to submit proofs to chain
    pub proof_frequency_in_blocks: BlockNum,
    /// The amount of tokens to pay to the executor
    pub price: U256,
    /// The amount of collateral the executor must post
    pub collateral: U256,
    /// The token to use for payment
    pub erc20_token_denomination: Address,
    /// The File size of the data to be stored
    pub file_size: U256, // TODO: Change this to a U64
    /// The CID of the data to be stored
    pub ipfs_file_cid: CidToken,
    /// The blake3 hash of the data to be stored
    pub blake3_checksum: Blake3HashToken,
}

impl Display for DealProposal {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Executor: {}\n
            Deal Length: {}\n
            Proof Frequency: {}\n
            Bounty: {}\n
            Collateral: {}\n
            Token Denomination: {}\n
            File Size: {}\n
            IPFS CID: {}\n
            Blake3 Hash: {}\n",
            self.executor_address,
            self.deal_length_in_blocks.0,
            self.proof_frequency_in_blocks.0,
            self.price,
            self.collateral,
            self.erc20_token_denomination,
            self.file_size,
            self.ipfs_file_cid,
            self.blake3_checksum,
        )
    }
}

impl Tokenize for DealProposal {
    fn into_tokens(self) -> Vec<Token> {
        vec![
            self.executor_address.into_token(),
            self.deal_length_in_blocks.into_token(),
            self.proof_frequency_in_blocks.into_token(),
            self.price.into_token(),
            self.collateral.into_token(),
            self.erc20_token_denomination.into_token(),
            self.file_size.into_token(),
            self.ipfs_file_cid.into_token(),
            self.blake3_checksum.into_token(),
        ]
    }
}

// TODO: Re-incorporate DealStatus
/// OnChainDealInfo - Information about a deal that is stored on chain
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OnChainDealInfo {
    pub deal_id: DealID,
    pub deal_start_block: BlockNum,
    pub deal_length_in_blocks: BlockNum,
    pub proof_frequency_in_blocks: BlockNum,
    pub price: U256,
    pub collateral: U256,
    pub erc20_token_denomination: Address,
    pub ipfs_file_cid: CidToken,
    pub file_size: U256,
    pub blake3_checksum: Blake3HashToken,
    pub creator_address: Address,
    pub executor_address: Address,
    // pub deal_status: DealStatus,
}

impl Display for OnChainDealInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Deal ID: {}\n
            Deal Start Block: {}\n
            Deal Length: {}\n
            Proof Frequency: {}\n
            Bounty: {}\n
            Collateral: {}\n
            Token Denomination: {}\n
            File Size: {}\n
            CID: {}\n
            Blake3 Hash: {}\n
            Creator Address: {}\n
            Executor Address: {}\n",
            self.deal_id,
            self.deal_start_block,
            self.deal_length_in_blocks,
            self.proof_frequency_in_blocks,
            self.price,
            self.collateral,
            self.erc20_token_denomination,
            self.ipfs_file_cid,
            self.file_size,
            self.blake3_checksum,
            self.creator_address,
            self.executor_address,
        )
    }
}


/// Impl Tokenizable for onChainDealInfo - This allows us to treat the struct as a Token with ethers
impl Tokenizable for OnChainDealInfo {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::Tuple(tokens) => {
                let mut tokens = tokens.into_iter();
                Ok(OnChainDealInfo {
                    deal_id: DealID::from_token(tokens.next().unwrap())?,
                    deal_start_block: BlockNum::from_token(tokens.next().unwrap())?,
                    deal_length_in_blocks: BlockNum::from_token(tokens.next().unwrap())?,
                    proof_frequency_in_blocks: BlockNum::from_token(tokens.next().unwrap())?,
                    price: U256::from_token(tokens.next().unwrap())?,
                    collateral: U256::from_token(tokens.next().unwrap())?,
                    erc20_token_denomination: Address::from_token(tokens.next().unwrap())?,
                    ipfs_file_cid: CidToken::from_token(tokens.next().unwrap())?,
                    file_size: U256::from_token(tokens.next().unwrap())?,
                    blake3_checksum: Blake3HashToken::from_token(tokens.next().unwrap())?,
                    creator_address: Address::from_token(tokens.next().unwrap())?,
                    executor_address: Address::from_token(tokens.next().unwrap())?,
                    // deal_status: DealStatus::from_token(tokens.next().unwrap())?,
                })
            }
            other => Err(InvalidOutputType(format!(
                "Expected `Tuple`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::Tuple(vec![
            self.deal_id.into_token(),
            self.deal_start_block.into_token(),
            self.deal_length_in_blocks.into_token(),
            self.proof_frequency_in_blocks.into_token(),
            self.price.into_token(),
            self.collateral.into_token(),
            self.erc20_token_denomination.into_token(),
            self.ipfs_file_cid.into_token(),
            self.file_size.into_token(),
            self.blake3_checksum.into_token(),
            self.creator_address.into_token(),
            self.executor_address.into_token(),
            // self.deal_status.into_token(),
        ])
    }
}

impl OnChainDealInfo {
    pub fn get_final_block(&self) -> BlockNum {
        self.deal_start_block + self.deal_length_in_blocks
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof {
    pub block_number: BlockNum,
    pub bao_proof_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofBuddyMessageType {
    SubmitProof,
    Cancel,
    InitiateChainlinkFinalization,
    WithdrawEarnings,
}
