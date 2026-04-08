import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TransferHook } from "../target/types/transfer_hook";
import {
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
  Keypair,
} from "@solana/web3.js";
import {
  ExtensionType,
  TOKEN_2022_PROGRAM_ID,
  getMintLen,
  createInitializeMintInstruction,
  createInitializeTransferHookInstruction,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  createTransferCheckedWithTransferHookInstruction,
  getExtraAccountMetas
} from "@solana/spl-token";

describe("transfer-hook", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TransferHook as Program<TransferHook>;

  const wallet = provider.wallet as anchor.Wallet;
  
  const connection = provider.connection;

  // Generate keypair to use as address for the transfer-hook enabled mint
  const mint = new Keypair();
  const decimals = 5;

  // Sender token account address
  const sourceTokenAccount = getAssociatedTokenAddressSync(
    mint.publicKey,
    wallet.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  // Recipient token account address
  const recipient = Keypair.generate();
  const destinationTokenAccount = getAssociatedTokenAddressSync(
    mint.publicKey,
    recipient.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  // ExtraAccountMetaList address
  // Store extra accounts required by the custom transfer hook instruction
  const [extraAccountMetaListPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("extra-account-metas"), mint.publicKey.toBuffer()],
    program.programId
  );


  
  it("Create Mint Account with Transfer Hook Extension", async () => {
    const extensions = [ExtensionType.TransferHook];
    const mintLen = getMintLen(extensions);
    const lamports =
      await provider.connection.getMinimumBalanceForRentExemption(mintLen);

    const transaction = new Transaction()
    .add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: mint.publicKey,
        space: mintLen,
        lamports: lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),

      createInitializeTransferHookInstruction(
        mint.publicKey,
        wallet.publicKey,
        program.programId, // Transfer Hook Program ID
        TOKEN_2022_PROGRAM_ID
      ),
      createInitializeMintInstruction(
        mint.publicKey,
        decimals,
        wallet.publicKey,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );

    const txSig = await sendAndConfirmTransaction(
      provider.connection,
      transaction,
      [wallet.payer, mint],
      { skipPreflight: true, commitment: "confirmed"}
    );

    console.log(`Transaction Signature: ${txSig}`);
  });

  // Create the two token accounts for the transfer-hook enabled mint
  // Fund the sender token account with 100 tokens
  it("Create Token Accounts and Mint Tokens", async () => {
    // 100 tokens
    const amount = 100 * 10 ** decimals;

    const transaction = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      ),
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        destinationTokenAccount,
        recipient.publicKey,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      ),
      createMintToInstruction(
        mint.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        amount,
        [],
        TOKEN_2022_PROGRAM_ID
      )
    );

    const txSig = await sendAndConfirmTransaction(
      connection,
      transaction,
      [wallet.payer],
      { skipPreflight: true, commitment: "confirmed"}
    );

    console.log(`Transaction Signature: ${txSig}`);
  });


  const [source_reward_pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("rewardpda"), mint.publicKey.toBuffer(),sourceTokenAccount.toBuffer()],
    program.programId
  );

  const [destinaton_reward_pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("rewardpda"), mint.publicKey.toBuffer(),destinationTokenAccount.toBuffer()],
    program.programId
  );


  it("create the reward_pda for token accounts", async() => {

    //  console.log("source_reward_pda meta: " + source_reward_pda);

         await connection.requestAirdrop(recipient.publicKey, 1e9); // 1 SOL

await new Promise(resolve => setTimeout(resolve, 1000)); // wait for confirmation

      const initializeRewardPda = await program.methods.initailize()
                                  .accounts({
                                    payer: wallet.publicKey,
                                    mint: mint.publicKey,
                                    tokenProgram:TOKEN_2022_PROGRAM_ID
                                  }).instruction() ; 

       const initializeRewardPda2 = await program.methods.initailize()
                                  .accounts({
                                    payer: recipient.publicKey,
                                    mint: mint.publicKey,
                                    tokenProgram:TOKEN_2022_PROGRAM_ID
                                  }).instruction() ;

      const transaction = new Transaction().add(
            initializeRewardPda,
              
      );

       const transaction2 = new Transaction().add(
        
            initializeRewardPda2        
      );

      const txSig = await sendAndConfirmTransaction(
                          provider.connection,
                          transaction,
                          [wallet.payer],
                          { skipPreflight: false, commitment: "confirmed"}
      );

      const txSig2 = await sendAndConfirmTransaction(
                          provider.connection,
                          transaction2,
                          [recipient],
                          { skipPreflight: false, commitment: "confirmed"}
      );

      console.log("Transaction Signature:", txSig);
      const rewardData = await program.account.rewardPda.fetch(source_reward_pda);
      console.log("Decoded Reward PDA:", rewardData);

      console.log("Transaction Signature:", txSig2);
      const rewardData2 = await program.account.rewardPda.fetch(destinaton_reward_pda);
      console.log("Decoded Reward PDA:", rewardData2);
  });

  const [dividend_per_token_account] = PublicKey.findProgramAddressSync(
    [Buffer.from("dividend_per_token"), mint.publicKey.toBuffer()],
    program.programId
  );

  it("Create dividend_per_token_account",async() => {


     
      const initializeDiv = await program.methods.initailizediv()
                                  .accounts({
                                    payer: wallet.publicKey,
                                    mint: mint.publicKey,
                                    tokenProgram:TOKEN_2022_PROGRAM_ID
                                  }).instruction();
                                  
        const transaction = new Transaction().add(
            initializeDiv,
              
      );   
      
       const txSig = await sendAndConfirmTransaction(
                          provider.connection,
                          transaction,
                          [wallet.payer],
                          { skipPreflight: false, commitment: "confirmed"}
      );

       console.log("Transaction Signature:", txSig);
      const rewardData = await program.account.dividendPerToken.fetch(dividend_per_token_account);
      console.log("Decoded Reward PDA:", rewardData.dividendPerToken.toNumber());

  })




//   // Account to store extra accounts required by the transfer hook instruction
  it("Create ExtraAccountMetaList Account", async () => {
    const extraAccountMetasInfo = await connection.getAccountInfo(extraAccountMetaListPDA);
    
    console.log("Extra accounts meta: " + extraAccountMetasInfo);

    if (extraAccountMetasInfo === null) {
      const initializeExtraAccountMetaListInstruction = await program.methods
      .initializeExtraAccountMetaList()
      .accounts({
        mint: mint.publicKey,
        tokenProgram:TOKEN_2022_PROGRAM_ID
      })
      .instruction();

      const transaction = new Transaction().add(
        initializeExtraAccountMetaListInstruction
      );

      const txSig = await sendAndConfirmTransaction(
        provider.connection,
        transaction,
        [wallet.payer],
        { skipPreflight: false, commitment: "confirmed"}
      );
      console.log("Transaction Signature:", txSig);
    }

  });

  it("Transfer Hook with Extra Account Meta", async () => {
    // 1 tokens
    const amount = 1.5 * 10 ** decimals;
    const amountBigInt = BigInt(amount);
for (let i = 0; i < 2; i++) {

  console.log(`\n========= TRANSFER ${i + 1} =========`);

  const ix = await createTransferCheckedWithTransferHookInstruction(
    connection,
    sourceTokenAccount,
    mint.publicKey,
    destinationTokenAccount,
    wallet.publicKey,
    amountBigInt,
    decimals,
    [],
    "confirmed",
    TOKEN_2022_PROGRAM_ID,
  );

  const tx = new Transaction().add(ix);

  const latestBlockhash = await connection.getLatestBlockhash();

  tx.recentBlockhash = latestBlockhash.blockhash;
  tx.feePayer = wallet.publicKey;

  tx.sign(wallet.payer);

  const sig = await connection.sendRawTransaction(tx.serialize());

  await connection.confirmTransaction({
    signature: sig,
    blockhash: latestBlockhash.blockhash,
    lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
  });

  console.log("Transaction Signature:", sig);

  // ✅ Fetch updated data after EACH transfer
  const rewardData = await program.account.rewardPda.fetch(source_reward_pda);
  const rewardData2 = await program.account.rewardPda.fetch(destinaton_reward_pda);

  const sourceBalance = await connection.getTokenAccountBalance(sourceTokenAccount);
  const destinationBalance = await connection.getTokenAccountBalance(destinationTokenAccount);

  console.log("Source :: ==> ");
  console.log("Source balance:", sourceBalance.value.uiAmount);
  console.log("Source Reward PDA:", rewardData.rewardDebt.toNumber());
  console.log("Source PendingReward PDA:", rewardData.pendingReward.toNumber());

  console.log("Destination :: ==> ");
  console.log("Destination balance:", destinationBalance.value.uiAmount);
  console.log("Destination Reward PDA:", rewardData2.rewardDebt.toNumber());
  console.log("Destination PendingReward PDA:", rewardData2.pendingReward.toNumber());

  // small delay (optional)
  await new Promise(res => setTimeout(res, 300));
}
}
  );
});

