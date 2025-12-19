import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";

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
  const MINT = new PublicKey("6gESBt8umWdDmwGBR1tpYwjhdmRKVS1QK23f9SoKTSnp");

  // Your market program (owns market PDAs)
  const MARKET_PROGRAM_ID = new PublicKey(
    "deprZ6k7MU6w3REU6hJ2yCfnkbDvzUZaKE4Z4BuZBhU"
  );

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
  // 1️⃣ Initialize config (safe to re-run once)
  // ------------------------------------------------------------

  console.log("\nInitializing config...");

  try {
    const tx = await program.methods
      .initializeConfig(MARKET_PROGRAM_ID)
      .accounts({
        payer,
        config: configPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Config initialized:", tx);
  } catch (e) {
    console.warn(
      "Config init failed (maybe already initialized):",
      e.toString()
    );
  }

  console.log("\nUpdating config...");
  try {
    const tx = await program.methods
      .updateConfig(MARKET_PROGRAM_ID)
      .accounts({
        payer,
        config: configPda,
      })
      .rpc();

    console.log("Config updated successfully:", tx);
  } catch (e) {
    console.error("Config update failed:", e);
    process.exit(1);
  }

  // ------------------------------------------------------------
  // 2️⃣ Initialize ExtraAccountMetaList (per mint)
  // ------------------------------------------------------------

  console.log("\nInitializing ExtraAccountMetaList...");

  try {
    const tx = await program.methods
      .initializeExtraAccountMetaList()
      .accounts({
        payer,
        mint: MINT,
        extraAccountMetaList: extraAccountMetaListPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("ExtraAccountMetaList initialized:", tx);
  } catch (e) {
    console.error("ExtraAccountMetaList init failed:", e);
    process.exit(1);
  }

  console.log("\n✅ Hook on-chain state initialized successfully");
})();
