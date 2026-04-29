/**
 * Convert a base58-encoded private key (Phantom/Solflare/pump.fun export)
 * into the Solana CLI JSON array format, then write it to
 * `keys/admin-wallet.json`.
 *
 * The deploy script (`scripts/deploy.sh`) expects that file to contain the
 * keypair for `6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP`. This converter
 * verifies the derived public key matches before writing.
 *
 * Usage:
 *   npx ts-node scripts/src/convert-private-key.ts
 *     → prompts for the base58 key on stdin (no shell history exposure)
 *   npx ts-node scripts/src/convert-private-key.ts <BASE58_PRIVATE_KEY>
 *     → one-shot (key visible in shell history; use the stdin form if you
 *       care about that)
 */
import { Keypair } from "@solana/web3.js";
// bs58 ships without TS types; use require to avoid needing @types/bs58.
// eslint-disable-next-line @typescript-eslint/no-var-requires
const bs58: { decode: (s: string) => Uint8Array } = require("bs58");
import * as fs from "fs";
import * as path from "path";
import * as readline from "readline";

const EXPECTED_PUBKEY = "6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP";
const OUTPUT_PATH = path.join(__dirname, "..", "..", "keys", "admin-wallet.json");

async function promptSecret(prompt: string): Promise<string> {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  // Very small "hide input" shim: mute echo while typing.
  const stdout = process.stdout;
  const origWrite = stdout.write.bind(stdout);
  (rl as any)._writeToOutput = (s: string) => {
    if (s.startsWith(prompt)) origWrite(prompt);
    else origWrite("*");
  };
  return new Promise((resolve) => {
    rl.question(prompt, (answer) => {
      rl.close();
      origWrite("\n");
      resolve(answer.trim());
    });
  });
}

async function main() {
  const argKey = process.argv[2];
  const base58Key = argKey && argKey.length > 0
    ? argKey
    : await promptSecret("Paste base58 private key (input hidden): ");

  if (!base58Key) {
    console.error("No key provided.");
    process.exit(1);
  }

  let secret: Uint8Array;
  try {
    secret = bs58.decode(base58Key);
  } catch (e: any) {
    console.error("Not valid base58:", e.message);
    process.exit(1);
  }

  if (secret.length !== 64) {
    console.error(
      `Decoded key is ${secret.length} bytes, expected 64. ` +
        `Make sure you exported the full secret key (not just the seed).`,
    );
    process.exit(1);
  }

  const kp = Keypair.fromSecretKey(secret);
  const pk = kp.publicKey.toBase58();
  console.log(`Derived public key: ${pk}`);

  if (pk !== EXPECTED_PUBKEY) {
    console.error(`\nERROR: public key mismatch.`);
    console.error(`  expected: ${EXPECTED_PUBKEY}`);
    console.error(`  got:      ${pk}`);
    console.error(`\nRefusing to write. Double-check that you exported the`);
    console.error(`admin wallet's private key (not a different wallet).`);
    process.exit(1);
  }

  const keysDir = path.dirname(OUTPUT_PATH);
  if (!fs.existsSync(keysDir)) fs.mkdirSync(keysDir, { recursive: true });

  const json = JSON.stringify(Array.from(secret));
  fs.writeFileSync(OUTPUT_PATH, json, { mode: 0o600 });

  console.log(`\nWrote Solana CLI keypair to: ${OUTPUT_PATH}`);
  console.log(`Permissions set to 0600 (owner read/write only).`);
  console.log(`\nVerify with:`);
  console.log(`  solana-keygen pubkey ${OUTPUT_PATH}`);
}

main().catch((e) => {
  console.error("error:", e.message ?? e);
  process.exit(1);
});
