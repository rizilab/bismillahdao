# Web Service

This service provides authentication via Keycloak and Multi-Party Computation (MPC) wallet functionality for secure key generation and transaction signing. It combines the authentication backend, MPC and MPCC services into a unified authentication and wallet management system.

## Overview

The Web service has two primary functions:
1. User authentication using Keycloak
2. MPC wallet creation and management for Solana

It implements threshold signatures using FROST (Flexible Round-Optimized Schnorr Threshold) signatures with Paillier encryption for secure computation. This approach allows for secure wallet creation and transaction signing without any single point of failure.

## Architecture

### Key Components

1. **Authentication System**: Keycloak integration for user management
2. **FROST Implementation**: Uses Schnorr signatures with a threshold approach
3. **Paillier Cryptosystem**: Enables secure computation on encrypted values
4. **Solana Integration**: Provides wallet functionality specific to Solana
5. **Frontend**: User interface for authentication and wallet management

### User Flow

1. User authenticates via Keycloak
2. After authentication, user can create an MPC wallet
3. Wallet shares are distributed securely
4. User can sign transactions using the MPC wallet

### Key Generation

For a 2-of-3 MPC wallet:
- 1 share stored on user's device (in keychain)
- 1 share stored in user's cloud backup
- 1 share stored on our server

This ensures that even if our server is compromised, the attacker cannot access user funds since they would need at least 2 shares.

### Transaction Signing

Transaction signing follows a two-round process:
1. **Round 1**: Each participant generates nonces and commitments
2. **Round 2**: Each participant creates signature shares
3. **Aggregation**: Shares are combined into a final signature

## Technical Q&A

### What's the difference between Schnorr signatures and Paillier cryptosystem?

**Schnorr Signatures:**
- A digital signature scheme
- Used to prove ownership without revealing secret keys
- FROST is a threshold version allowing multiple parties to create a signature together

**Paillier Cryptosystem:**
- A homomorphic encryption system
- Allows computation on encrypted numbers without decryption
- Used for secure computation between parties in MPC

### How do they work together?

Schnorr handles the actual signing of transactions, while Paillier helps with the secure computation needed during the MPC process:

1. Paillier is used to securely generate and compute values needed for signing
2. FROST/Schnorr is used to create the actual signature that's valid on-chain

### What are the benefits of two-round signing?

Two-round signing provides important security guarantees:

1. **Prevents Rogue-Key Attacks**: By committing to nonces before seeing others'
2. **Prevents Replay Attacks**: By incorporating all commitments in the signature

### How is wallet generation secured?

Key generation happens in the frontend, with shares distributed to:
- User's device keychain
- User's cloud backup
- Our server

This distribution ensures:
- Server breach protection (server only has 1 of 3 shares)
- Device loss protection (recovery possible with server + cloud shares)
- User control (server can never act alone)

## Usage Flow

1. **Authentication**:
   - User logs in via Keycloak
   - Auth service validates credentials and issues tokens
   - R4GMI Tauri app receives authentication tokens

2. **Wallet Creation**:
   - Generate key shares on frontend after authentication
   - Store one share in user's device
   - Store backup share in user's cloud storage
   - Send one share to our server

3. **Transaction Signing**:
   - User initiates transaction in R4GMI app
   - Frontend retrieves device share
   - Frontend and server participate in two-round signing
   - Final signature is created and submitted on-chain

4. **Wallet Recovery**:
   - If device is lost, user authenticates to our service
   - Server share + cloud backup share are used to recover wallet
   - New device share is generated and stored

## Implementation Details

The service uses the [FROST-ed25519](https://docs.rs/frost-ed25519/latest/frost_ed25519/) crate for Schnorr threshold signatures, which is compatible with Solana's Ed25519 signature scheme.

For secure frontend-backend communication during signing, we implement a client-server protocol that follows the FROST two-round signing process.

### Integration with R4GMI Project

The R4GMI Tauri application interacts with this Auth service for:
1. User authentication via Keycloak
2. Initial wallet creation during onboarding
3. Transaction signing and submission
4. Wallet recovery if needed

## Dependencies

- `keycloak`: Authentication and user management
- `frost-ed25519`: FROST implementation for Ed25519
- `paillier`: Implementation of the Paillier cryptosystem
- `@solana/web3.js`: Solana web3 library for transaction creation and submission

## Configuration

The service can be configured using environment variables or a configuration file. Key configuration parameters include:

- `KEYCLOAK_URL`: URL of the Keycloak server
- `KEYCLOAK_REALM`: Realm name in Keycloak
- `KEYCLOAK_CLIENT_ID`: Client ID for the application
- `DATABASE_URL`: Connection string for the database
- `PORT`: Port for the server to listen on
- `SOLANA_RPC_URL`: URL of the Solana RPC node,