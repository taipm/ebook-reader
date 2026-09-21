import path from "node:path";

// Debug binary from `cargo build` (respects CARGO_TARGET_DIR / shared target); override with TAURI_APP_BIN.
const application =
  process.env.TAURI_APP_BIN ??
  path.join(process.env.CARGO_TARGET_DIR ?? path.resolve(__dirname, "../src-tauri/target"), "debug/ebook-reader");

// tauri-driver has no macOS support — run this suite on Linux/Windows only.
export const config = {
  runner: "local",
  specs: ["./tests/**/*.spec.ts"],
  maxInstances: 1,
  capabilities: [{ "tauri:options": { application } }],
  hostname: "localhost",
  port: 4444,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { ui: "bdd", timeout: 60000 },
  logLevel: "info",
};
