use cid::Cid;
use ethers::{
    abi::{
        AbiType, InvalidOutputType, ParamType, Token,
        Token::{Address as Ad, String as Str, Uint},
        Tokenizable, Tokenize,
    },
    prelude::Address,
    types::U256,
};
use ethers_contract_derive::EthAbiType;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sled::IVec;
use std::fmt::{Display, Formatter};
use std::ops::{Add, Mul, Sub};

/* Contract Primitives */

// CID
// TODO make a little macro for tokenizable
/// A Wrapper around the CID struct from the cid crate
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CidToken(pub Cid);

impl Display for CidToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CidToken {
    pub fn cid(&self) -> Cid {
        self.0
    }
}

/// Impl Tokenizable for CidToken - This allows us to use CidToken as a Token in the ethers crate
impl Tokenizable for CidToken {
    /// Convert a Token::String to a CidToken
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Str(s) => Ok(CidToken(Cid::try_from(s).unwrap())),
            other => Err(InvalidOutputType(format!(
                "Expected `String`, got {:?}",
                other
            ))),
        }
    }
    /// Convert a CidToken to a Token::String
    fn into_token(self) -> Token {
        Str(self.0.to_string())
    }
}

impl Serialize for CidToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
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

// Blake3 Hashes

/// A Wrapper around the Blake3 Hash struct from the blake3 crate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blake3HashToken(pub blake3::Hash);

impl Blake3HashToken {
    pub fn hash(&self) -> blake3::Hash {
        self.0
    }
    pub fn to_hex(&self) -> String {
        self.0.to_hex().to_string()
    }
}

impl Tokenizable for Blake3HashToken {
    fn into_token(self) -> Token {
        Str(self.to_hex())
    }
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            // TODO: THis was a hack for testing purposes, fix it
            Str(s) => Ok(Blake3HashToken(blake3::Hash::from_hex(s).unwrap())),
            other => Err(InvalidOutputType(format!(
                "Expected `String`, got {:?}",
                other
            ))),
        }
    }
}

impl AbiType for Blake3HashToken {
    fn param_type() -> ParamType {
        ParamType::String
    }
}

pub fn serialize_hash_token<S: Serializer>(
    hash: &Blake3HashToken,
    s: S,
) -> Result<S::Ok, S::Error> {
    let hash_bytes = hash.0.as_bytes();
    s.serialize_bytes(hash_bytes)
}

pub fn deserialize_hash_token<'de, D>(deserializer: D) -> Result<Blake3HashToken, D::Error>
where
    D: Deserializer<'de>,
{
    let hash_bytes = <[u8; 32]>::deserialize(deserializer)?;
    Ok(Blake3HashToken(blake3::Hash::from(hash_bytes)))
}

/// DealIds - The onChain ID of a deal submitted to Ethereum
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct DealID(pub u64);

impl Tokenizable for DealID {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Uint(u) => Ok(DealID(u.as_u64())),
            other => Err(InvalidOutputType(format!(
                "Expected `Uint`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Uint(self.0.into())
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

impl Tokenizable for BlockNum {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Uint(u) => Ok(BlockNum(u.as_u64())),
            other => Err(InvalidOutputType(format!(
                "Expected `Uint`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Uint(self.0.into())
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

// TODO: Talk to @c about this
/// Token Multiplier - a wrapper around u64 to specify a multiplier for a token
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord, EthAbiType,
)]
pub struct TokenMultiplier(pub u64);
// Our default multiplier is 1e9
impl Default for TokenMultiplier {
    fn default() -> Self {
        TokenMultiplier(1_000_000_000)
    }
}

/// Multiply a token amount as a float and return the result as TokenAmount
impl Mul<f64> for TokenMultiplier {
    type Output = TokenAmount;
    fn mul(self, other: f64) -> TokenAmount {
        let amount = (self.0 as f64 * other).round() as u64;
        if amount == 0 {
            TokenAmount(1)
        } else {
            TokenAmount(amount)
        }
    }
}

/// Token Amount
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TokenAmount(pub u64);

impl Tokenizable for TokenAmount {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Uint(u) => Ok(TokenAmount(u.as_u64())),
            other => Err(InvalidOutputType(format!(
                "Expected `Uint`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Uint(self.0.into())
    }
}

// TODO: Do we want to define this like this? There's already a type called Token in the ethers crate
/// Token - The identifier for a token contract
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenAddress(pub Address);

impl Tokenizable for TokenAddress {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Ad(a) => Ok(TokenAddress(a)),
            other => Err(InvalidOutputType(format!(
                "Expected `Address`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Ad(self.0)
    }
}

/// An Enum describing the different states a deal can be in
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EthAbiType)]
pub enum DealStatus {
    /// The deal does not exist
    Non,
    /// The deal has been submitted to the chain, but not yet accepted
    DealCreated,
    /// The deal has been accepted by the executor
    DealAccepted,
    /// The deal is active
    DealActive,
    /// The deal has been completed
    DealCompleted,
    /// The deal has been finalized
    DealFinalized,
    /// The deal was submitted to the chain, but not accepted
    DealTimedOut,
    /// The deal was submitted to the chain, and then cancelled
    DealCancelled,
}

/// DealProposal - A proposal for a deal
/// This is the data that is submitted to the Ethereum contract to create a deal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DealProposal {
    /// The address of the party to propose the deal to
    pub executor_address: Address,
    /// The length of the deal in blocks
    pub deal_length_in_blocks: BlockNum,
    /// The frequency with which to submit proofs to chain
    pub proof_frequency_in_blocks: BlockNum,
    /// The amount of tokens to pay to the executor
    pub price: TokenAmount,
    /// The amount of collateral the executor must post
    pub collateral: TokenAmount,
    /// The token to use for payment
    pub erc20_token_denomination: TokenAddress,
    /// The File size of the data to be stored
    pub file_size: U256,
    // TODO: Change these to the correct types
    /// The CID of the data to be stored
    pub ipfs_file_cid: CidToken,
    #[serde(
        serialize_with = "serialize_hash_token",
        deserialize_with = "deserialize_hash_token"
    )]
    /// The blake3 hash of the data to be stored
    pub blake3_checksum: Blake3HashToken,
}

// TODO: Figure out how to derive this using EthAbiType
// TODO: Cleanup Token types so that they implement Tokenizable
impl Tokenize for DealProposal {
    fn into_tokens(self) -> Vec<ethers::abi::Token> {
        vec![
            Ad(self.executor_address),
            Uint(U256::from(self.deal_length_in_blocks.0)),
            Uint(U256::from(self.proof_frequency_in_blocks.0)),
            Uint(U256::from(self.price.0)),
            Uint(U256::from(self.collateral.0)),
            Ad(self.erc20_token_denomination.0),
            self.file_size.into_token(),
            self.ipfs_file_cid.into_token(),
            self.blake3_checksum.into_token(),
        ]
    }
}

// TODO: Re-incorporate DealStatus
/// OnChainDealInfo - Information about a deal that is stored on chain
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct OnChainDealInfo {
    pub deal_id: DealID,
    pub deal_start_block: BlockNum,
    pub deal_length_in_blocks: BlockNum,
    pub proof_frequency_in_blocks: BlockNum,
    pub price: TokenAmount,
    pub collateral: TokenAmount,
    pub erc20_token_denomination: TokenAddress,
    pub ipfs_file_cid: CidToken,
    pub file_size: U256,
    #[serde(
        serialize_with = "serialize_hash_token",
        deserialize_with = "deserialize_hash_token"
    )]
    pub blake3_checksum: Blake3HashToken,
    pub creator_address: Address,
    pub executor_address: Address,
    // pub deal_status: DealStatus,
}

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
                    price: TokenAmount::from_token(tokens.next().unwrap())?,
                    collateral: TokenAmount::from_token(tokens.next().unwrap())?,
                    erc20_token_denomination: TokenAddress::from_token(tokens.next().unwrap())?,
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
        Token::Tuple(self.into_tokens())
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
