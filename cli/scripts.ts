import { Program, web3 } from '@project-serum/anchor';
import * as anchor from '@project-serum/anchor';
import {
    Keypair,
    PublicKey,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    Transaction,
    TransactionInstruction,
    sendAndConfirmTransaction
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, AccountLayout, MintLayout, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";

import fs from 'fs';
import { CollectionPool, GlobalPool, RafflePool } from './types';
import { publicKey } from '@project-serum/anchor/dist/cjs/utils';
import { Raffle } from '../target/types/raffle';

const GLOBAL_AUTHORITY_SEED = "global-authority";
const TREASURY_WALLET = new PublicKey('Am9xhPPVCfDZFDabcGgmQ8GTMdsbqEt1qVXbyhTxybAp');
const PROGRAM_ID = "3qJm618bPvosqjFZqMjrUYgVXNKGDPBmTVRMqHar92DK";

const METAPLEX = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');

const RAFFLE_SIZE = 64168;
const COLLECTION_SIZE = 12816;
const DECIMALS = 1000000000;

anchor.setProvider(anchor.AnchorProvider.local(web3.clusterApiUrl('devnet')));
const solConnection = anchor.getProvider().connection;
const payer = anchor.AnchorProvider.local().wallet;
console.log(payer.publicKey.toBase58());

const idl = JSON.parse(
    fs.readFileSync(__dirname + "/raffle.json", "utf8")
);

let program: Program = null;

// Address of the deployed program.
const programId = new anchor.web3.PublicKey(PROGRAM_ID);

// Generate the program client from IDL.
program = new anchor.Program(idl, programId);
console.log('ProgramId: ', program.programId.toBase58());

const main = async () => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );
    console.log('GlobalAuthority: ', globalAuthority.toBase58());
    // console.log(await getCollectionState());

    await initProject();
    // await addCollection(payer.publicKey, new PublicKey('GYq1mi8dh18nRAHbtdDuWiVRu4oAuSNzxoy3qStqX4RA'));
    // await createRaffle(payer.publicKey, new PublicKey("FLuGogNV1UPns65SCz8ZLBnPx1P9EtcjVphvbyg2t6ix"), 1, 1654249100, 100);
    // await buyTicket(payer.publicKey, new PublicKey("FLuGogNV1UPns65SCz8ZLBnPx1P9EtcjVphvbyg2t6ix"), 5);
    // await revealWinner(payer.publicKey, new PublicKey("FLuGogNV1UPns65SCz8ZLBnPx1P9EtcjVphvbyg2t6ix"));
    // await claimReward(payer.publicKey, new PublicKey("FLuGogNV1UPns65SCz8ZLBnPx1P9EtcjVphvbyg2t6ix"));
    // await withdrawNft(payer.publicKey, new PublicKey("FLuGogNV1UPns65SCz8ZLBnPx1P9EtcjVphvbyg2t6ix"));

    // const pool = await getRaffleState(new PublicKey("FLuGogNV1UPns65SCz8ZLBnPx1P9EtcjVphvbyg2t6ix"));
    // console.log(pool);
}

/**
 * @dev Initialize the project - exactly the init account
 * @returns Init accounts for this project
 */
export const initProject = async () => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );
    let userAddress = payer.publicKey;

    let collection = await PublicKey.createWithSeed(
        userAddress,
        "collection-pool",
        program.programId,
    );
    console.log(userAddress.toBase58());

    let ix = SystemProgram.createAccountWithSeed({
        fromPubkey: userAddress,
        basePubkey: userAddress,
        seed: "collection-pool",
        newAccountPubkey: collection,
        lamports: await solConnection.getMinimumBalanceForRentExemption(COLLECTION_SIZE),
        space: COLLECTION_SIZE,
        programId: program.programId,
    });

    const tx = await program.rpc.initialize(
        bump, {
        accounts: {
            admin: payer.publicKey,
            globalAuthority,
            collection,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
        },
        instructions: [
            ix
        ],
        signers: [],
    });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);
    return true;
}

/**
 * @dev Add collection to the Program collection list
 * @param userAddress The caller of this function
 * @param collectionId The collection verified creator address to add
 */
export const addCollection = async (
    userAddress: PublicKey,
    collectionId: PublicKey
) => {
    let state: GlobalPool = await getGlobalState();
    let admin = state.superAdmin;
    let collection = await PublicKey.createWithSeed(
        admin,
        "collection-pool",
        program.programId,
    );
    const tx = await program.rpc.addCollection(
        {
            accounts: {
                admin: userAddress,
                collection,
                collectionId
            },
            instructions: [],
            signers: [],
        });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);
}

/**
 * @dev CreateRaffle function
 * @param userAddress The raffle creator's address
 * @param nft_mint The nft_mint address
 * @param ticketPriceSol The ticket price by SOL 
 * @param endTimestamp The raffle end timestamp
 * @param max The max entrants of this raffle
 */
export const createRaffle = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
    ticketPriceSol: number,
    endTimestamp: number,
    max: number
) => {

    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );
    let state: GlobalPool = await getGlobalState();
    let admin = state.superAdmin;
    let collection = await PublicKey.createWithSeed(
        admin,
        "collection-pool",
        program.programId,
    );

    let ownerNftAccount = await getAssociatedTokenAccount(userAddress, nft_mint);

    let ix0 = await getATokenAccountsNeedCreate(
        solConnection,
        userAddress,
        globalAuthority,
        [nft_mint]
    );
    console.log("Dest NFT Account = ", ix0.destinationAccounts[0].toBase58());

    let raffle;
    let i;

    for (i = 10; i > 0; i--) {
        raffle = await PublicKey.createWithSeed(
            userAddress,
            nft_mint.toBase58().slice(0, i),
            program.programId,
        );
        let state = await getStateByKey(raffle);
        if (state === null) {
            console.log(i);
            break;
        }
    }
    console.log(i);

    let ix = SystemProgram.createAccountWithSeed({
        fromPubkey: userAddress,
        basePubkey: userAddress,
        seed: nft_mint.toBase58().slice(0, i),
        newAccountPubkey: raffle,
        lamports: await solConnection.getMinimumBalanceForRentExemption(RAFFLE_SIZE),
        space: RAFFLE_SIZE,
        programId: program.programId,
    });

    const metadataAddr = await getMetadataAddr(nft_mint);

    const tx = await program.rpc.createRaffle(
        bump,
        new anchor.BN(ticketPriceSol * DECIMALS),
        new anchor.BN(endTimestamp),
        new anchor.BN(max),
        {
            accounts: {
                admin: payer.publicKey,
                globalAuthority,
                raffle,
                collection,
                ownerTempNftAccount: ownerNftAccount,
                destNftTokenAccount: ix0.destinationAccounts[0],
                nftMintAddress: nft_mint,
                mintMetadata: metadataAddr,
                tokenProgram: TOKEN_PROGRAM_ID,
                tokenMetadataProgram: METAPLEX
            },
            instructions: [
                ix,
                ...ix0.instructions,
            ],
            signers: [],
        });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);

}

/**
 * @dev BuyTicket function
 * @param userAddress The use's address
 * @param nft_mint The nft_mint address
 * @param amount The amount of ticket to buy
 */
export const buyTicket = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
    amount: number
) => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );

    const raffleKey = await getRaffleKey(nft_mint);
    let raffleState = await getStateByKey(raffleKey);

    const creator = raffleState.creator;

    const tx = await program.rpc.buyTickets(
        bump,
        new anchor.BN(amount),
        {
            accounts: {
                buyer: userAddress,
                raffle: raffleKey,
                globalAuthority,
                creator,
                treasuryWallet: TREASURY_WALLET,
                systemProgram: SystemProgram.programId,
            },
            instructions: [],
            signers: [],
        });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);

}

/**
 * @dev RevealWinner function
 * @param userAddress The user's address to call this function
 * @param raffleKey The raffleKey address
 */
export const revealWinner = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
) => {
    const raffleKey = await getRaffleKey(nft_mint);

    console.log(userAddress.toBase58());
    console.log(raffleKey.toBase58());
    const tx = await program.rpc.revealWinner(
        {
            accounts: {
                buyer: userAddress,
                raffle: raffleKey,
            },
            instructions: [],
            signers: [],
        });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);
}

/**
 * @dev ClaimReward function
 * @param userAddress The winner's address
 * @param nft_mint The nft_mint address
 */
export const claimReward = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
) => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );

    const raffleKey = await getRaffleKey(nft_mint);
    const srcNftTokenAccount = await getAssociatedTokenAccount(globalAuthority, nft_mint);

    let ix0 = await getATokenAccountsNeedCreate(
        solConnection,
        userAddress,
        userAddress,
        [nft_mint]
    );
    console.log("Claimer's NFT Account: ", ix0.destinationAccounts[0]);

    let tx = await program.rpc.claimReward(
        bump,
        {
            accounts: {
                claimer: userAddress,
                globalAuthority,
                raffle: raffleKey,
                claimerNftTokenAccount: ix0.destinationAccounts[0],
                srcNftTokenAccount,
                nftMintAddress: nft_mint,
                tokenProgram: TOKEN_PROGRAM_ID,
            },
            instructions: [
                ...ix0.instructions
            ],
            signers: [],
        });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);

}

/**
 * @dev WithdrawNFT function
 * @param userAddress The creator's address
 * @param nft_mint The nft_mint address
 */
export const withdrawNft = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
) => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );

    const raffleKey = await getRaffleKey(nft_mint);
    const srcNftTokenAccount = await getAssociatedTokenAccount(globalAuthority, nft_mint);

    let ix0 = await getATokenAccountsNeedCreate(
        solConnection,
        userAddress,
        userAddress,
        [nft_mint]
    );
    console.log("Creator's NFT Account: ", ix0.destinationAccounts[0].toBase58());
    console.log(raffleKey.toBase58());

    let tx = await program.rpc.withdrawNft(
        bump, {
        accounts: {
            claimer: userAddress,
            globalAuthority,
            raffle: raffleKey,
            claimerNftTokenAccount: ix0.destinationAccounts[0],
            srcNftTokenAccount,
            nftMintAddress: nft_mint,
            tokenProgram: TOKEN_PROGRAM_ID,
        },
        instructions: [
            ...ix0.instructions
        ],
        signers: [],
    });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);

}


export const getRaffleKey = async (
    nft_mint: PublicKey
): Promise<PublicKey | null> => {
    let poolAccounts = await solConnection.getParsedProgramAccounts(
        program.programId,
        {
            filters: [
                {
                    dataSize: RAFFLE_SIZE
                },
                {
                    memcmp: {
                        "offset": 40,
                        "bytes": nft_mint.toBase58()
                    }
                }
            ]
        }
    );

    if (poolAccounts.length !== 0) {
        let len = poolAccounts.length;
        let max = 0;
        let maxId = 0;
        for (let i = 0; i < len; i++) {
            let state = await getStateByKey(poolAccounts[i].pubkey);
            if (state.endTimestamp.toNumber() > max) {
                max = state.endTimestamp.toNumber();
                maxId = i;
            }
        }
        let raffleKey = poolAccounts[maxId].pubkey;
        return raffleKey;
    } else {
        return null;
    }
}

export const getRaffleState = async (
    nft_mint: PublicKey
): Promise<RafflePool | null> => {

    let poolAccounts = await solConnection.getParsedProgramAccounts(
        program.programId,
        {
            filters: [
                {
                    dataSize: RAFFLE_SIZE
                },
                {
                    memcmp: {
                        "offset": 40,
                        "bytes": nft_mint.toBase58()
                    }
                }
            ]
        }
    );
    if (poolAccounts.length !== 0) {
        let rentalKey = poolAccounts[0].pubkey;

        try {
            let rentalState = await program.account.rafflePool.fetch(rentalKey);
            return rentalState as unknown as RafflePool;
        } catch {
            return null;
        }
    } else {
        return null;
    }
}
export const getStateByKey = async (
    raffleKey: PublicKey
): Promise<RafflePool | null> => {
    try {
        let rentalState = await program.account.rafflePool.fetch(raffleKey);
        return rentalState as unknown as RafflePool;
    } catch {
        return null;
    }
}

export const getGlobalState = async (): Promise<GlobalPool | null> => {
    const [globalAuthority, _] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );
    try {
        let state = await program.account.globalPool.fetch(globalAuthority);
        return state as unknown as GlobalPool;
    } catch {
        return null;
    }
}

export const getCollectionState = async (): Promise<CollectionPool | null> => {
    let state: GlobalPool = await getGlobalState()
    let admin = state.superAdmin;
    let collection = await PublicKey.createWithSeed(
        admin,
        "collection-pool",
        program.programId,
    );

    try {
        let state = await program.account.collectionPool.fetch(collection);
        return state as unknown as CollectionPool;
    } catch {
        return null;
    }
}

const getAssociatedTokenAccount = async (ownerPubkey: PublicKey, mintPk: PublicKey): Promise<PublicKey> => {
    let associatedTokenAccountPubkey = (await PublicKey.findProgramAddress(
        [
            ownerPubkey.toBuffer(),
            TOKEN_PROGRAM_ID.toBuffer(),
            mintPk.toBuffer(), // mint address
        ],
        ASSOCIATED_TOKEN_PROGRAM_ID
    ))[0];
    return associatedTokenAccountPubkey;
}

export const getATokenAccountsNeedCreate = async (
    connection: anchor.web3.Connection,
    walletAddress: anchor.web3.PublicKey,
    owner: anchor.web3.PublicKey,
    nfts: anchor.web3.PublicKey[],
) => {
    let instructions = [], destinationAccounts = [];
    for (const mint of nfts) {
        const destinationPubkey = await getAssociatedTokenAccount(owner, mint);
        let response = await connection.getAccountInfo(destinationPubkey);
        if (!response) {
            const createATAIx = createAssociatedTokenAccountInstruction(
                destinationPubkey,
                walletAddress,
                owner,
                mint,
            );
            instructions.push(createATAIx);
        }
        destinationAccounts.push(destinationPubkey);
        if (walletAddress != owner) {
            const userAccount = await getAssociatedTokenAccount(walletAddress, mint);
            response = await connection.getAccountInfo(userAccount);
            if (!response) {
                const createATAIx = createAssociatedTokenAccountInstruction(
                    userAccount,
                    walletAddress,
                    walletAddress,
                    mint,
                );
                instructions.push(createATAIx);
            }
        }
    }
    return {
        instructions,
        destinationAccounts,
    };
}

export const createAssociatedTokenAccountInstruction = (
    associatedTokenAddress: anchor.web3.PublicKey,
    payer: anchor.web3.PublicKey,
    walletAddress: anchor.web3.PublicKey,
    splTokenMintAddress: anchor.web3.PublicKey
) => {
    const keys = [
        { pubkey: payer, isSigner: true, isWritable: true },
        { pubkey: associatedTokenAddress, isSigner: false, isWritable: true },
        { pubkey: walletAddress, isSigner: false, isWritable: false },
        { pubkey: splTokenMintAddress, isSigner: false, isWritable: false },
        {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        {
            pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
            isSigner: false,
            isWritable: false,
        },
    ];
    return new anchor.web3.TransactionInstruction({
        keys,
        programId: ASSOCIATED_TOKEN_PROGRAM_ID,
        data: Buffer.from([]),
    });
}
export const getMetadataAddr = async (mint: PublicKey): Promise<PublicKey> => {
    return (
        await PublicKey.findProgramAddress([Buffer.from('metadata'), METAPLEX.toBuffer(), mint.toBuffer()], METAPLEX)
    )[0];
};


main()