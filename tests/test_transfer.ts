import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { 
  PublicKey, 
  Keypair, 
  Transaction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import * as fs from "fs";
import * as path from "path";
import {
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddress,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getMint,
  getAccount,
  getTransferHook,
  createTransferCheckedWithTransferHookInstruction,
} from "@solana/spl-token";
// Note: addExtraAccountMetasForExecute may need to be imported differently
// For now, we'll use the transfer hook interface helpers directly
import { Bouncer } from "../target/types/bouncer";
import { MarketTransferHook } from "../target/types/market_transfer_hook";

// Constants
const TOKEN_MINT = new PublicKey("6uMMEsXVim3QvuxA2Xy5NYguyvPspE1fG8S7NC9zENWj");
const BOUNCER_LIST = new PublicKey("7h7qtpFwNNgYPK68b9abbomcUoBcTVvmWC21TQWsQVn9");
const WHITELISTED_PDA = new PublicKey("J8ridcz8pgJ4g9E5sk7xmjDjWD3AR5rMqXUxkrc8Zp9L");
const BOUNCER_PROGRAM_ID = new PublicKey("4qn7TjxgnALkV5wjqSjeedSPx8XbacSYNKH4Gv54QEQC");
const TRANSFER_HOOK_PROGRAM_ID = new PublicKey("5HXB2HCrvizDb87ZHkmgLtNtXgojqCm5owd3L1yfGHuH");

describe("Transfer Hook Test", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const connection = provider.connection;

  const bouncerProgram = anchor.workspace.bouncer as Program<Bouncer>;
  const transferHookProgram = anchor.workspace.marketTransferHook as Program<MarketTransferHook>;

  let testKeypair: Keypair;
  let mintAuthority: Keypair; // Local keypair with mint authority
  let testTokenAccount: PublicKey;
  let destinationTokenAccount: PublicKey;

  // Helper function to load keypair from file
  function loadKeypair(keypairPath: string): Keypair {
    const fullPath = path.resolve(keypairPath.replace("~", process.env.HOME || ""));
    if (!fs.existsSync(fullPath)) {
      throw new Error(`Keypair file not found: ${fullPath}`);
    }
    const keypairData = JSON.parse(fs.readFileSync(fullPath, "utf-8"));
    return Keypair.fromSecretKey(Uint8Array.from(keypairData));
  }

  // Helper function to load or create keypair
  function loadOrCreateKeypair(keypairPath: string): Keypair {
    const fullPath = path.join(__dirname, "..", keypairPath);
    
    if (fs.existsSync(fullPath)) {
      console.log(`Loading existing keypair from ${keypairPath}`);
      const keypairData = JSON.parse(fs.readFileSync(fullPath, "utf-8"));
      return Keypair.fromSecretKey(Uint8Array.from(keypairData));
    } else {
      console.log(`Creating new keypair and saving to ${keypairPath}`);
      const keypair = Keypair.generate();
      // Ensure directory exists
      const dir = path.dirname(fullPath);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }
      fs.writeFileSync(fullPath, JSON.stringify(Array.from(keypair.secretKey)));
      return keypair;
    }
  }

  before(async () => {
    // Load local keypair (mint authority)
    try {
      mintAuthority = loadKeypair("~/.config/solana/id.json");
      console.log("\n=== Test Setup ===");
      console.log("Mint authority:", mintAuthority.publicKey.toBase58());
    } catch (error: any) {
      throw new Error(`Failed to load mint authority keypair: ${error.message}\nPlease ensure ~/.config/solana/id.json exists`);
    }

    // Load or create persistent test keypair
    testKeypair = loadOrCreateKeypair("keys/test-keypair.json");
    console.log("Test keypair:", testKeypair.publicKey.toBase58());

    // Check SOL balance for test keypair
    const balance = await connection.getBalance(testKeypair.publicKey);
    console.log("Test keypair SOL balance:", balance / LAMPORTS_PER_SOL, "SOL");
    
    if (balance < 0.1 * LAMPORTS_PER_SOL) {
      throw new Error(
        `Insufficient SOL balance. Please fund the keypair: ${testKeypair.publicKey.toBase58()}\n` +
        `Current balance: ${balance / LAMPORTS_PER_SOL} SOL\n` +
        `Minimum required: 0.1 SOL`
      );
    }

    // Get or create test token account (on-curve, can be created)
    // Note: getOrCreateAssociatedTokenAccount doesn't support TOKEN_2022_PROGRAM_ID directly
    // So we'll get the address and create manually if needed
    testTokenAccount = await getAssociatedTokenAddress(
      TOKEN_MINT,
      testKeypair.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );
    console.log("Test token account:", testTokenAccount.toBase58());
    
    // Check if account exists, create if not
    try {
      await getAccount(connection, testTokenAccount, undefined, TOKEN_2022_PROGRAM_ID);
      console.log("Test token account already exists");
    } catch (error: any) {
      if (error.name === "TokenAccountNotFoundError" || error.code === "TokenAccountNotFoundError" || error.message?.includes("TokenAccountNotFoundError")) {
        console.log("Creating test token account...");
        const createIx = createAssociatedTokenAccountInstruction(
          testKeypair.publicKey,
          testTokenAccount,
          testKeypair.publicKey,
          TOKEN_MINT,
          TOKEN_2022_PROGRAM_ID
        );
        const tx = new Transaction().add(createIx);
        await sendAndConfirmTransaction(connection, tx, [testKeypair]);
        console.log("✓ Test token account created");
      } else {
        throw error;
      }
    }

    // Get destination token account address (PDA - cannot create it ourselves)
    // The PDA's program must create it, or it will be created on first transfer
    destinationTokenAccount = getAssociatedTokenAddressSync(
      TOKEN_MINT,
      WHITELISTED_PDA,
      true, // allowOwnerOffCurve = true for PDA
      TOKEN_2022_PROGRAM_ID
    );
    console.log("Destination token account (PDA):", destinationTokenAccount.toBase58());
  });

  it("Reads bouncer list information", async () => {
    console.log("\n=== Reading Bouncer List ===");
    console.log("List PDA:", BOUNCER_LIST.toBase58());

    try {
      const listAccount = await bouncerProgram.account.list.fetch(BOUNCER_LIST);
      console.log("\nList Account Info:");
      console.log("  Version:", listAccount.version);
      console.log("  Authority:", listAccount.authority.toBase58());
      console.log("  Creator:", listAccount.creator.toBase58());
      console.log("  List ID:", listAccount.listId.toString());
      console.log("  Policy:", listAccount.policy === 0 ? "Allowlist" : "Blocklist");
      console.log("  Storage Kind:", listAccount.storageKind === 0 ? "Direct PDA" : "Merkle Root");
      console.log("  Entry Count:", listAccount.entryCount.toString());
      console.log("  Flags:", listAccount.flags.toString());
      console.log("  Frozen:", (listAccount.flags & 1) !== 0);
    } catch (error) {
      console.error("Error reading list:", error);
      throw error;
    }
  });

  it("Checks specific entry in whitelist", async () => {
    console.log("\n=== Checking Entry for Whitelisted PDA ===");
    console.log("Subject PDA:", WHITELISTED_PDA.toBase58());

    // Derive entry PDA
    const [entryPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("entry"),
        BOUNCER_LIST.toBuffer(),
        WHITELISTED_PDA.toBuffer(),
      ],
      BOUNCER_PROGRAM_ID
    );

    console.log("Entry PDA:", entryPda.toBase58());

    try {
      const entryAccount = await bouncerProgram.account.entry.fetch(entryPda);
      console.log("\nEntry Account Info:");
      console.log("  Version:", entryAccount.version);
      console.log("  List:", entryAccount.list.toBase58());
      console.log("  Subject:", entryAccount.subject.toBase58());
      console.log("  Status:", 
        entryAccount.status === 1 ? "ALLOW" : 
        entryAccount.status === 2 ? "BLOCK" : "UNSET"
      );
      console.log("  ✓ Entry found and status is:", 
        entryAccount.status === 1 ? "ALLOWED" : "NOT ALLOWED"
      );
    } catch (error: any) {
      if (error.code === "AccountNotFoundError") {
        console.log("  ✗ Entry not found - PDA is not in whitelist");
        throw new Error("Entry not found in whitelist");
      } else {
        console.error("Error reading entry:", error);
        throw error;
      }
    }
  });

  it("Mints tokens to test keypair (if needed)", async () => {
    console.log("\n=== Checking Token Account ===");
    console.log("Token Mint:", TOKEN_MINT.toBase58());
    console.log("Recipient:", testKeypair.publicKey.toBase58());

    // Check if test token account exists and has balance
    let testTokenAccountInfo;
    let needsMinting = false;
    let accountExists = false;
    
    try {
      testTokenAccountInfo = await getAccount(connection, testTokenAccount, undefined, TOKEN_2022_PROGRAM_ID);
      accountExists = true;
      const balance = testTokenAccountInfo.amount;
      console.log("Test token account exists:", testTokenAccount.toBase58());
      console.log("Current balance:", balance.toString());
      
      if (balance === BigInt(0)) {
        console.log("⚠ Token account exists but has zero balance. Will attempt to mint.");
        needsMinting = true;
      } else {
        console.log("✓ Token account has sufficient balance. Skipping mint.");
        return; // Exit early if we have tokens
      }
    } catch (error: any) {
      if (error.name === "TokenAccountNotFoundError" || error.code === "TokenAccountNotFoundError" || error.message?.includes("TokenAccountNotFoundError")) {
        console.log("Token account does not exist. Creating account...");
        accountExists = false;
        needsMinting = true;
      } else {
        console.error("Unexpected error checking token account:", error);
        throw error;
      }
    }

    // Create account if it doesn't exist
    if (!accountExists) {
      try {
        const createIx = createAssociatedTokenAccountInstruction(
          testKeypair.publicKey,
          testTokenAccount,
          testKeypair.publicKey,
          TOKEN_MINT,
          TOKEN_2022_PROGRAM_ID
        );
        const tx = new Transaction().add(createIx);
        await sendAndConfirmTransaction(connection, tx, [testKeypair]);
        console.log("✓ Test token account created");
      } catch (createError: any) {
        console.error("Failed to create token account:", createError.message);
        throw createError;
      }
    }

    // Only proceed with minting if needed
    if (!needsMinting) {
      return;
    }

    console.log("\n=== Attempting to Mint Tokens ===");
    
    // Get mint info to find mint authority
    const mintInfo = await getMint(connection, TOKEN_MINT, undefined, TOKEN_2022_PROGRAM_ID);
    console.log("Mint decimals:", mintInfo.decimals);
    console.log("Mint authority:", mintInfo.mintAuthority?.toBase58() || "None");

    // Check if we can mint (need mint authority)
    if (!mintInfo.mintAuthority) {
      console.log("⚠ Mint has no authority - cannot mint.");
      throw new Error("No tokens available and cannot mint. Please fund the token account manually.");
    }

    // Check if mint authority matches our loaded keypair
    if (mintInfo.mintAuthority && !mintInfo.mintAuthority.equals(mintAuthority.publicKey)) {
      console.log("⚠ Loaded keypair is not the mint authority.");
      console.log("⚠ Expected:", mintInfo.mintAuthority?.toBase58() || "None");
      console.log("⚠ Got:", mintAuthority.publicKey.toBase58());
      throw new Error("Loaded keypair is not mint authority. Please check ~/.config/solana/id.json");
    }

    // Mint tokens using mint authority keypair
    const mintAmount = BigInt(10000 * Math.pow(10, mintInfo.decimals)); // Mint 10,000 tokens
    console.log("Minting", mintAmount.toString(), "raw units to test keypair...");
    
    try {
      const mintIx = createMintToInstruction(
        TOKEN_MINT,
        testTokenAccount,
        mintAuthority.publicKey, // Use mint authority, not test keypair
        mintAmount,
        [],
        TOKEN_2022_PROGRAM_ID
      );
      const tx = new Transaction().add(mintIx);
      const sig = await sendAndConfirmTransaction(connection, tx, [mintAuthority]); // Sign with mint authority
      console.log("✓ Tokens minted successfully!");
      console.log("Transaction signature:", sig);
    } catch (error: any) {
      console.error("✗ Failed to mint tokens:", error.message);
      throw new Error(`Minting failed: ${error.message}. Please check mint authority and try again.`);
    }
  });

  it("Checks destination token account for PDA", async () => {
    console.log("\n=== Checking Destination Token Account ===");
    console.log("Destination PDA:", WHITELISTED_PDA.toBase58());
    console.log("Destination token account:", destinationTokenAccount.toBase58());

    try {
      const destAccount = await getAccount(connection, destinationTokenAccount, undefined, TOKEN_2022_PROGRAM_ID);
      console.log("✓ Destination token account exists");
      console.log("Current balance:", destAccount.amount.toString());
    } catch (error: any) {
      if (error.name === "TokenAccountNotFoundError" || error.code === "TokenAccountNotFoundError" || error.message?.includes("TokenAccountNotFoundError")) {
        console.log("⚠ Destination token account does not exist yet");
        console.log("⚠ Note: Creating ATA for PDA requires the PDA's program to sign");
        console.log("⚠ The account will be created automatically on first transfer if the program supports it");
        // Don't throw - this is expected for PDAs
      } else {
        console.error("Unexpected error checking destination account:", error);
        throw error;
      }
    }
  });

  it("Attempts transfer from user to whitelisted PDA", async () => {
    console.log("\n=== Testing Transfer ===");
    console.log("From:", testKeypair.publicKey.toBase58());
    console.log("To PDA:", WHITELISTED_PDA.toBase58());

    // Check test account balance
    let testBalance;
    try {
      const testAccount = await getAccount(connection, testTokenAccount, undefined, TOKEN_2022_PROGRAM_ID);
      testBalance = testAccount.amount;
      console.log("Test account balance:", testBalance.toString());
    } catch (error: any) {
      if (error.name === "TokenAccountNotFoundError" || error.code === "TokenAccountNotFoundError" || error.message?.includes("TokenAccountNotFoundError")) {
        throw new Error("Test token account does not exist. Please run the minting test first.");
      }
      throw error;
    }

    if (testBalance === BigInt(0)) {
      throw new Error("Test account has zero balance. Please run the minting test first.");
    }

    // Get mint info to check decimals and transfer hook
    const mintInfo = await getMint(connection, TOKEN_MINT, undefined, TOKEN_2022_PROGRAM_ID);
    const decimals = mintInfo.decimals;
    const transferHook = getTransferHook(mintInfo);
    
    if (!transferHook) {
      throw new Error("Token does not have a transfer hook configured");
    }
    
    console.log("Transfer hook program:", transferHook.programId.toBase58());

    // Get config PDA
    const [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("config")],
      TRANSFER_HOOK_PROGRAM_ID
    );
    console.log("Config PDA:", configPda.toBase58());

    // Fetch config to verify it exists
    try {
      const config = await transferHookProgram.account.config.fetch(configPda);
      console.log("Config found", config);
      // Note: Config structure may vary - adjust based on actual structure
    } catch (error) {
      console.error("Error fetching config:", error);
      throw new Error("Config not initialized. Please initialize config first.");
    }

    // Build transfer instruction with extra accounts
    const transferAmount = BigInt(1000 * Math.pow(10, decimals)); // Transfer 1000 tokens
    console.log("\nAttempting transfer of", transferAmount.toString(), "raw units (", 1000, "tokens)...");

    // Create base transfer instruction
    const transferIx = await createTransferCheckedWithTransferHookInstruction(
      connection,
      testTokenAccount,
      TOKEN_MINT,
      destinationTokenAccount,
      testKeypair.publicKey,
      transferAmount,
      decimals,
      [],
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );

    const tx = new Transaction().add(transferIx);

    try {
      const signature = await sendAndConfirmTransaction(
        connection,
        tx,
        [testKeypair],
        { commitment: "confirmed", skipPreflight: true }
      );
      console.log("✓ Transfer successful!");
      console.log("Transaction signature:", signature);

      // Verify balances
      const testAccountAfter = await getAccount(connection, testTokenAccount, undefined, TOKEN_2022_PROGRAM_ID);
      console.log("\nFinal balances:");
      console.log("  Test account:", testAccountAfter.amount.toString());
      
      // Check destination account (might not exist if PDA program doesn't create it)
      try {
        const destAccountAfter = await getAccount(connection, destinationTokenAccount, undefined, TOKEN_2022_PROGRAM_ID);
        console.log("  Destination account:", destAccountAfter.amount.toString());
      } catch (error: any) {
        if (error.name === "TokenAccountNotFoundError" || error.code === "TokenAccountNotFoundError") {
          console.log("  Destination account: Not created yet (PDA program may create it on first transfer)");
        } else {
          throw error;
        }
      }
    } catch (error: any) {
      console.error("✗ Transfer failed:", error.message);
      if (error.logs) {
        console.error("Program logs:");
        error.logs.forEach((log: string) => console.error("  ", log));
      }
      throw error;
    }
  });

  it("Attempts transfer from user to user (should fail)", async () => {
    console.log("\n=== Testing User-to-User Transfer (Should Fail) ===");
    
    // Create another test keypair
    const recipientKeypair = Keypair.generate();
    console.log("Recipient:", recipientKeypair.publicKey.toBase58());

    const recipientTokenAccount = await getAssociatedTokenAddress(
      TOKEN_MINT,
      recipientKeypair.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );

    // Check if recipient account exists, create if not
    let recipientAccountExists = false;
    try {
      await getAccount(connection, recipientTokenAccount, undefined, TOKEN_2022_PROGRAM_ID);
      recipientAccountExists = true;
      console.log("Recipient token account already exists");
    } catch (error: any) {
      if (error.name === "TokenAccountNotFoundError" || error.code === "TokenAccountNotFoundError" || error.message?.includes("TokenAccountNotFoundError")) {
        console.log("Creating recipient token account...");
        try {
          const createIx = createAssociatedTokenAccountInstruction(
            testKeypair.publicKey,
            recipientTokenAccount,
            recipientKeypair.publicKey,
            TOKEN_MINT,
            TOKEN_2022_PROGRAM_ID
          );
          const tx = new Transaction().add(createIx);
          await sendAndConfirmTransaction(connection, tx, [testKeypair]);
          recipientAccountExists = true;
          console.log("✓ Recipient token account created");
        } catch (createError: any) {
          console.error("Failed to create recipient account:", createError.message);
          throw createError;
        }
      } else {
        throw error;
      }
    }

    // Get mint decimals
    const mintInfo2 = await getMint(connection, TOKEN_MINT, undefined, TOKEN_2022_PROGRAM_ID);
    const decimals2 = mintInfo2.decimals;
    const transferHook2 = getTransferHook(mintInfo2);
    
    const transferAmount2 = BigInt(100 * Math.pow(10, decimals2));
    console.log("Attempting transfer of", transferAmount2.toString(), "raw units (", 100, "tokens)...");

    const transferIx2 = await createTransferCheckedWithTransferHookInstruction(
      connection,
      testTokenAccount,
      TOKEN_MINT,
      recipientTokenAccount,
      testKeypair.publicKey,
      transferAmount2,
      decimals2,
      [],
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );


    const tx2 = new Transaction().add(transferIx2);

    try {
      const signature2 = await sendAndConfirmTransaction(
        connection,
        tx2,
        [testKeypair],
        { commitment: "confirmed" , skipPreflight: true }
      );
      console.log("✗ Transfer should have failed but succeeded!");
      console.log("Transaction signature:", signature2);
      throw new Error("Transfer should have been blocked");
    } catch (error: any) {
      console.log("Error:", error);
      if (error.message.includes("Transfer not allowed") || 
          error.message.includes("custom program error") ||
          error.logs?.some((log: string) => log.includes("Transfer not allowed"))) {
        console.log("✓ Transfer correctly blocked by transfer hook");
        if (error.logs) {
          console.log("Program logs:");
          error.logs.forEach((log: string) => console.log("  ", log));
        }
      } else {
        console.error("✗ Unexpected error:", error.message);
        throw error;
      }
    }
  });
});

