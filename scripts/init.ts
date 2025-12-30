import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

(async () => {
  // ------------------------------------------------------------
  // Provider & program
  // ------------------------------------------------------------
  // Use devnet by default, can be overridden via ANCHOR_PROVIDER_URL
  const rpcUrl = "https://api.devnet.solana.com";
  const connection = new Connection(rpcUrl, "confirmed");

  // Load wallet from default location
  const walletPath = path.join(os.homedir(), ".config", "solana", "id.json");
  const walletKeypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf-8")))
  );

  const wallet = new anchor.Wallet(walletKeypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  // Load IDL from target/idl directory (relative to project root)
  const idlPath = path.join(
    __dirname,
    "..",
    "target",
    "idl",
    "market_transfer_hook.json"
  );
  const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));

  // Create program instance using Program.at() or direct instantiation
  const program = new anchor.Program(
    idl,
    provider
  ) as anchor.Program<anchor.Idl>;

  const payer = provider.wallet.publicKey;

  // ------------------------------------------------------------
  // CONSTANTS — FILL THESE CAREFULLY
  // ------------------------------------------------------------

  // Your already-created Token-2022 mint
  const MINT = new PublicKey("6uMMEsXVim3QvuxA2Xy5NYguyvPspE1fG8S7NC9zENWj");

  // Bouncer program ID (whitelist program)
  const BOUNCER_PROGRAM_ID = new PublicKey("4qn7TjxgnALkV5wjqSjeedSPx8XbacSYNKH4Gv54QEQC");

  // Bouncer list PDA (the whitelist)
  const BOUNCER_LIST = new PublicKey("7h7qtpFwNNgYPK68b9abbomcUoBcTVvmWC21TQWsQVn9");

  // ------------------------------------------------------------
  // PDA derivations
  // ------------------------------------------------------------

  // Config PDA (global, one per hook program)
  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );

  // ExtraAccountMetaList PDA (one per mint)
  const [extraAccountMetaListPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("extra-account-metas"), MINT.toBuffer()],
    program.programId
  );

  console.log("Payer:", payer.toBase58());
  console.log("Hook program:", program.programId.toBase58());
  console.log("Config PDA:", configPda.toBase58());
  console.log("ExtraAccountMetaList PDA:", extraAccountMetaListPda.toBase58());

  // ------------------------------------------------------------
  // 1️⃣ Initialize or Update config
  // ------------------------------------------------------------

  console.log("\nChecking config state...");
  console.log("Bouncer Program ID:", BOUNCER_PROGRAM_ID.toBase58());
  console.log("Bouncer List:", BOUNCER_LIST.toBase58());

  // Check if config already exists
  const configAccountInfo = await connection.getAccountInfo(configPda);
  
  if (configAccountInfo) {
    console.log("⚠️  Config account already exists.");
    console.log("⚠️  Attempting to update with new values...");
    
    try {
      const tx = await program.methods
        .updateConfig(BOUNCER_PROGRAM_ID, BOUNCER_LIST)
        .accounts({
          payer,
          config: configPda,
        })
        .rpc();

      console.log("✅ Config updated successfully:", tx);
    } catch (e: any) {
      console.error("❌ Config update failed:", e.toString());
      console.error("⚠️  Config exists but update failed. This is a new program ID, so config should not exist yet.");
      console.error("⚠️  If you see this, there might be a conflict. Exiting...");
      process.exit(1);
    }
  } else {
    // Config doesn't exist, initialize it (fresh start with new program ID)
    console.log("\nInitializing new config...");
    try {
      const tx = await program.methods
        .initializeConfig(BOUNCER_PROGRAM_ID, BOUNCER_LIST)
        .accounts({
          payer,
          config: configPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("✅ Config initialized:", tx);
    } catch (e: any) {
      console.error("❌ Config init failed:", e.toString());
      process.exit(1);
    }
  }

  // ------------------------------------------------------------
  // 2️⃣ Initialize ExtraAccountMetaList (per mint)
  // ------------------------------------------------------------

  console.log("\nInitializing ExtraAccountMetaList...");
  console.log("This will set up the extra accounts needed for transfer hook execution.");

  try {
    const tx = await program.methods
      .initializeExtraAccountMetaList()
      .accountsPartial({
        payer,
        mint: MINT,
        extraAccountMetaList: extraAccountMetaListPda,
        config: configPda,
        bouncerProgram: BOUNCER_PROGRAM_ID,
        bouncerList: BOUNCER_LIST,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("✅ ExtraAccountMetaList initialized:", tx);
  } catch (e: any) {
    if (e.toString().includes("already in use") || e.toString().includes("AccountDiscriminatorAlreadySet")) {
      console.warn("⚠️  ExtraAccountMetaList already initialized. Skipping...");
    } else {
      console.error("❌ ExtraAccountMetaList init failed:", e);
      process.exit(1);
    }
  }

  console.log("\n✅ Hook on-chain state initialized successfully");
})();
