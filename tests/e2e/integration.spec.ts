import { test, expect } from "@playwright/test";
import { spawn, ChildProcess } from "child_process";
import { promisify } from "util";

// Helper to start our test WebSocket server
async function startTestServer(): Promise<{
  server: ChildProcess;
  port: number;
}> {
  return new Promise((resolve, reject) => {
    // Start our Rust test server
    const server = spawn(
      "cargo",
      ["test", "--test", "server_integration_tests", "--", "--nocapture"],
      {
        stdio: "pipe",
        env: { ...process.env, RUST_LOG: "debug" },
      },
    );

    let port = 8080; // Default port

    server.stdout?.on("data", (data) => {
      const output = data.toString();
      console.log("Server output:", output);

      // Look for server URL in output
      const urlMatch = output.match(/ws:\/\/127\.0\.0\.1:(\d+)/);
      if (urlMatch) {
        port = parseInt(urlMatch[1]);
        resolve({ server, port });
      }
    });

    server.stderr?.on("data", (data) => {
      console.error("Server error:", data.toString());
    });

    server.on("error", (error) => {
      reject(error);
    });

    // Timeout after 10 seconds
    setTimeout(() => {
      reject(new Error("Server startup timeout"));
    }, 10000);
  });
}

test.describe("Real Server Integration Tests", () => {
  let testServer: ChildProcess | null = null;
  let serverPort = 8080;

  test.beforeAll(async () => {
    try {
      const { server, port } = await startTestServer();
      testServer = server;
      serverPort = port;

      // Give server time to fully start
      await new Promise((resolve) => setTimeout(resolve, 2000));
    } catch (error) {
      console.error("Failed to start test server:", error);
      // Fall back to default port
      serverPort = 8080;
    }
  });

  test.afterAll(async () => {
    if (testServer) {
      testServer.kill();
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  });

  test("should connect to real WebSocket server", async ({ page }) => {
    await page.goto("/test-app.html");

    // Connect to our real test server
    await page.fill("#server-url", `ws://localhost:${serverPort}`);
    await page.click("#connect-btn");

    // Wait for connection
    await expect(page.locator("#status")).toHaveText("Connected");

    // Verify we can send messages
    await page.fill("#message-input", "Hello from browser test!");
    await page.click("#send-btn");

    // Verify message was sent
    await expect(page.locator("#messages-sent")).toHaveText("1");
  });

  test("should receive server responses", async ({ page }) => {
    await page.goto("/test-app.html");

    // Connect to server
    await page.fill("#server-url", `ws://localhost:${serverPort}`);
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Send a message that should get echoed back
    await page.fill("#message-input", "Echo test message");
    await page.click("#send-btn");

    // Wait for response
    await page.waitForTimeout(1000);

    // Check that we received a response
    const receivedMessages = page.locator("#messages .message.received");
    await expect(receivedMessages).toHaveCount(1);
  });

  test("should handle RPC requests with real server", async ({ page }) => {
    await page.goto("/test-app.html");

    // Connect to server
    await page.fill("#server-url", `ws://localhost:${serverPort}`);
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Send RPC echo request
    await page.click("#rpc-echo-btn");

    // Wait for response
    await page.waitForTimeout(1000);

    // Check that we received an RPC response
    const receivedMessages = page.locator("#messages .message.received");
    await expect(receivedMessages).toContainText("RPC Response");
  });

  test("should handle connection drops and reconnects", async ({ page }) => {
    await page.goto("/test-app.html");

    // Connect to server
    await page.fill("#server-url", `ws://localhost:${serverPort}`);
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Disconnect
    await page.click("#disconnect-btn");
    await expect(page.locator("#status")).toHaveText("Disconnected");

    // Reconnect
    await page.click("#reconnect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Verify we can still send messages after reconnect
    await page.fill("#message-input", "Post-reconnect message");
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");
  });

  test("should handle multiple concurrent connections", async ({ browser }) => {
    // Create multiple browser contexts
    const context1 = await browser.newContext();
    const context2 = await browser.newContext();

    const page1 = await context1.newPage();
    const page2 = await context2.newPage();

    try {
      // Connect both pages to the server
      await page1.goto("/test-app.html");
      await page2.goto("/test-app.html");

      await page1.fill("#server-url", `ws://localhost:${serverPort}`);
      await page2.fill("#server-url", `ws://localhost:${serverPort}`);

      await page1.click("#connect-btn");
      await page2.click("#connect-btn");

      await expect(page1.locator("#status")).toHaveText("Connected");
      await expect(page2.locator("#status")).toHaveText("Connected");

      // Send messages from both clients
      await page1.fill("#message-input", "Message from client 1");
      await page1.click("#send-btn");

      await page2.fill("#message-input", "Message from client 2");
      await page2.click("#send-btn");

      // Verify both clients sent messages
      await expect(page1.locator("#messages-sent")).toHaveText("1");
      await expect(page2.locator("#messages-sent")).toHaveText("1");
    } finally {
      await context1.close();
      await context2.close();
    }
  });

  test("should handle server restart gracefully", async ({ page }) => {
    await page.goto("/test-app.html");

    // Connect to server
    await page.fill("#server-url", `ws://localhost:${serverPort}`);
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Kill the server
    if (testServer) {
      testServer.kill();
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    // Try to send a message (should fail gracefully)
    await page.fill("#message-input", "Message after server restart");
    await page.click("#send-btn");

    // Should show disconnected status
    await expect(page.locator("#status")).toHaveClass(/disconnected/);
  });
});

test.describe("Performance and Load Tests", () => {
  let testServer: ChildProcess | null = null;
  let serverPort = 8080;

  test.beforeAll(async () => {
    try {
      const { server, port } = await startTestServer();
      testServer = server;
      serverPort = port;
      await new Promise((resolve) => setTimeout(resolve, 2000));
    } catch (error) {
      console.error("Failed to start test server:", error);
      serverPort = 8080;
    }
  });

  test.afterAll(async () => {
    if (testServer) {
      testServer.kill();
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  });

  test("should handle rapid message sending", async ({ page }) => {
    await page.goto("/test-app.html");

    // Connect to server
    await page.fill("#server-url", `ws://localhost:${serverPort}`);
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Send multiple messages rapidly
    const messageCount = 10;
    for (let i = 0; i < messageCount; i++) {
      await page.fill("#message-input", `Rapid message ${i}`);
      await page.click("#send-btn");
      await page.waitForTimeout(100); // Small delay between messages
    }

    // Verify all messages were sent
    await expect(page.locator("#messages-sent")).toHaveText(
      messageCount.toString(),
    );
  });

  test("should handle large message payloads", async ({ page }) => {
    await page.goto("/test-app.html");

    // Connect to server
    await page.fill("#server-url", `ws://localhost:${serverPort}`);
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Create a large message
    const largeMessage = "A".repeat(10000); // 10KB message
    await page.fill("#message-input", largeMessage);
    await page.click("#send-btn");

    // Verify large message was sent
    await expect(page.locator("#messages-sent")).toHaveText("1");
  });

  test("should maintain connection under load", async ({ page }) => {
    await page.goto("/test-app.html");

    // Connect to server
    await page.fill("#server-url", `ws://localhost:${serverPort}`);
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Send messages continuously for 30 seconds
    const startTime = Date.now();
    const duration = 5000; // 5 seconds for test

    while (Date.now() - startTime < duration) {
      await page.fill("#message-input", `Load test message ${Date.now()}`);
      await page.click("#send-btn");
      await page.waitForTimeout(100);
    }

    // Verify connection is still active
    await expect(page.locator("#status")).toHaveText("Connected");

    // Verify we sent many messages
    const sentCount = await page.locator("#messages-sent").textContent();
    expect(parseInt(sentCount || "0")).toBeGreaterThan(10);
  });
});
