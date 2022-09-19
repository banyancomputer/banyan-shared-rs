# banyan shared repos

This Repo contains a bunch of Rust Modules we share between our projects.

# cutie little code cov badgie <3
[![codecov](https://codecov.io/gh/banyancomputer/banyan-shared-rs/branch/master/graph/badge.svg?token=BNIKTPUS3T)](https://codecov.io/gh/banyancomputer/banyan-shared-rs)

# cutie little code cov diagrammie <3 
![codecov diagram](https://codecov.io/gh/banyancomputer/banyan-shared-rs/branch/master/graphs/tree.svg?token=BNIKTPUS3T)

## Modules
- proofs - A library for creating and verifying proofs
- deals - A library for building deal proposals
- estuary - A library for interacting with the Estuary API
- eth - A library for interacting with the Ethereum blockchain
- ipfs - A library for working with IPFS and CIDs
- types - A library for defining common types used across our projects

# Testing
This repo requires a lot of configuration to run tests.
For now remember to set the following ENV variables before running tests:
- For eth.rs
    - `ETH_API_URL` - The URL of the Ethereum rpc you want to connect to
    - `ETH_API_KEY` - The API key for the Ethereum rpc you want to connect to
    - `ETH_CHAIN_ID` - The chain id of the Ethereum network you want to connect to
    - `ETH_PRIVATE_KEY` - The private key of the Ethereum account you want to use for testing. Required for signing transactions.
    - `ETH_CONTRACT_ADDRESS` - The address of the Banyan contract you want to use for testing.
- For estuary.rs
    - `ESTUARY_API_HOSTNAME` - The URL of the Estuary API you want to connect to
    - `ESTUARY_API_KEY` - The API key for the Estuary API you want to connect to
