import * as anchor from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';

export interface GlobalPool {
    superAdmin: PublicKey,
}

export interface CollectionPool {
    count: anchor.BN,
    collections: PublicKey[],
}

export interface RafflePool {
    creator: PublicKey,
    nftMint: PublicKey,
    count: anchor.BN,
    noRepeat: anchor.BN,
    maxEntrants: anchor.BN,
    startTimestamp: anchor.BN,
    endTimestamp: anchor.BN,
    ticketPriceSol: anchor.BN,
    claimed: anchor.BN,
    winnerIndex: anchor.BN,
    winner: PublicKey,
    entrants: PublicKey[],
}