exports.config = {
  runner: "local",
  specs: ["./tests/**/*.spec.ts"],
  maxInstances: 1,
  capabilities: [
    {
      "tauri:options": {
        // Built app path (after `npm run tauri build`) — dùng dev binary thay vì production
        application: "/Users/taipm/.cargo/shared-target/debug/ebook-reader",
      },
    },
  ],
  // Tauri driver chạy ở port 4444
  hostname: "localhost",
  port: 4444,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
  // Log level
  logLevel: "info",
};
