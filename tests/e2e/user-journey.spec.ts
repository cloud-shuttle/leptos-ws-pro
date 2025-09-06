import { test, expect } from "@playwright/test";

test.describe("Complete User Journey Tests", () => {
  test("should complete full WebSocket communication flow", async ({
    page,
  }) => {
    await page.goto("/test-app.html");

    // Step 1: Initial state - disconnected
    await expect(page.locator("#status")).toHaveText("Disconnected");
    await expect(page.locator("#connect-btn")).toBeEnabled();
    await expect(page.locator("#disconnect-btn")).toBeDisabled();

    // Step 2: Configure server URL
    await page.fill("#server-url", "ws://localhost:8080");

    // Step 3: Connect to server
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");
    await expect(page.locator("#connect-btn")).toBeDisabled();
    await expect(page.locator("#disconnect-btn")).toBeEnabled();

    // Step 4: Send initial message
    await page.fill("#message-input", "Hello, Server!");
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Step 5: Send heartbeat
    await page.click("#send-heartbeat-btn");
    await expect(page.locator("#messages-sent")).toHaveText("2");

    // Step 6: Test RPC functionality
    await page.click("#rpc-echo-btn");
    await expect(page.locator("#messages-sent")).toHaveText("3");

    // Step 7: Verify metrics are updating
    await expect(page.locator("#connection-quality")).toHaveText("100%");
    await expect(page.locator("#reconnect-count")).toHaveText("0");

    // Step 8: Disconnect
    await page.click("#disconnect-btn");
    await expect(page.locator("#status")).toHaveText("Disconnected");
    await expect(page.locator("#connect-btn")).toBeEnabled();
    await expect(page.locator("#disconnect-btn")).toBeDisabled();
  });

  test("should handle connection failure and recovery", async ({ page }) => {
    await page.goto("/test-app.html");

    // Step 1: Try to connect to invalid server
    await page.fill("#server-url", "ws://invalid-server:9999");
    await page.click("#connect-btn");

    // Step 2: Should show connection failed
    await expect(page.locator("#status")).toHaveClass(/disconnected/);

    // Step 3: Switch to valid server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");

    // Step 4: Should connect successfully
    await expect(page.locator("#status")).toHaveText("Connected");

    // Step 5: Verify we can send messages
    await page.fill("#message-input", "Recovery test message");
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");
  });

  test("should handle reconnection scenario", async ({ page }) => {
    await page.goto("/test-app.html");

    // Step 1: Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Step 2: Send some messages
    await page.fill("#message-input", "Message before reconnect");
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Step 3: Disconnect
    await page.click("#disconnect-btn");
    await expect(page.locator("#status")).toHaveText("Disconnected");

    // Step 4: Reconnect
    await page.click("#reconnect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");
    await expect(page.locator("#reconnect-count")).toHaveText("1");

    // Step 5: Send message after reconnect
    await page.fill("#message-input", "Message after reconnect");
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("2");
  });

  test("should handle RPC request-response cycle", async ({ page }) => {
    await page.goto("/test-app.html");

    // Step 1: Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Step 2: Send RPC echo request
    await page.click("#rpc-echo-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Step 3: Wait for response (simulated)
    await page.waitForTimeout(1000);

    // Step 4: Send RPC broadcast request
    await page.click("#rpc-broadcast-btn");
    await expect(page.locator("#messages-sent")).toHaveText("2");

    // Step 5: Send RPC stats request
    await page.click("#rpc-stats-btn");
    await expect(page.locator("#messages-sent")).toHaveText("3");

    // Step 6: Verify all RPC requests were sent
    const messages = page.locator("#messages .message.sent");
    await expect(messages).toContainText("RPC Echo Request");
    await expect(messages).toContainText("RPC Broadcast Request");
    await expect(messages).toContainText("RPC Get Stats Request");
  });

  test("should handle message history and cleanup", async ({ page }) => {
    await page.goto("/test-app.html");

    // Step 1: Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Step 2: Send multiple messages
    const messages = ["Message 1", "Message 2", "Message 3"];
    for (const message of messages) {
      await page.fill("#message-input", message);
      await page.click("#send-btn");
    }

    // Step 3: Verify messages are in history
    await expect(page.locator("#messages .message.sent")).toHaveCount(3);

    // Step 4: Clear messages
    await page.click("#clear-messages-btn");
    await expect(page.locator("#messages .message")).toHaveCount(0);

    // Step 5: Send new message after clear
    await page.fill("#message-input", "Message after clear");
    await page.click("#send-btn");
    await expect(page.locator("#messages .message.sent")).toHaveCount(1);
  });

  test("should handle connection quality monitoring", async ({ page }) => {
    await page.goto("/test-app.html");

    // Step 1: Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Step 2: Verify initial connection quality
    await expect(page.locator("#connection-quality")).toHaveText("100%");

    // Step 3: Send messages to test quality monitoring
    for (let i = 0; i < 5; i++) {
      await page.fill("#message-input", `Quality test message ${i}`);
      await page.click("#send-btn");
      await page.waitForTimeout(200);
    }

    // Step 4: Verify metrics are tracking
    await expect(page.locator("#messages-sent")).toHaveText("5");
    await expect(page.locator("#connection-quality")).toHaveText("100%");
  });

  test("should handle error scenarios gracefully", async ({ page }) => {
    await page.goto("/test-app.html");

    // Step 1: Try to send message without connection
    await page.fill("#message-input", "Message without connection");
    await page.click("#send-btn");

    // Step 2: Should not send (button disabled)
    await expect(page.locator("#messages-sent")).toHaveText("0");

    // Step 3: Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Step 4: Send message with connection
    await page.fill("#message-input", "Message with connection");
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Step 5: Disconnect and try to send again
    await page.click("#disconnect-btn");
    await expect(page.locator("#status")).toHaveText("Disconnected");

    // Step 6: Should not be able to send (button disabled)
    await expect(page.locator("#send-btn")).toBeDisabled();
  });

  test("should handle rapid connect/disconnect cycles", async ({ page }) => {
    await page.goto("/test-app.html");

    // Step 1: Perform multiple connect/disconnect cycles
    for (let i = 0; i < 3; i++) {
      // Connect
      await page.fill("#server-url", "ws://localhost:8080");
      await page.click("#connect-btn");
      await expect(page.locator("#status")).toHaveText("Connected");

      // Send a message
      await page.fill("#message-input", `Cycle ${i} message`);
      await page.click("#send-btn");

      // Disconnect
      await page.click("#disconnect-btn");
      await expect(page.locator("#status")).toHaveText("Disconnected");

      await page.waitForTimeout(500);
    }

    // Step 2: Final connect and verify state
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Step 3: Verify we can still send messages
    await page.fill("#message-input", "Final message");
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");
  });

  test("should handle different message types in sequence", async ({
    page,
  }) => {
    await page.goto("/test-app.html");

    // Step 1: Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Step 2: Send regular message
    await page.fill("#message-input", "Regular message");
    await page.click("#send-btn");

    // Step 3: Send heartbeat
    await page.click("#send-heartbeat-btn");

    // Step 4: Send RPC request
    await page.click("#rpc-echo-btn");

    // Step 5: Send another regular message
    await page.fill("#message-input", "Another regular message");
    await page.click("#send-btn");

    // Step 6: Verify all message types were sent
    await expect(page.locator("#messages-sent")).toHaveText("4");

    // Step 7: Verify message history contains all types
    const messages = page.locator("#messages .message.sent");
    await expect(messages).toContainText("Regular message");
    await expect(messages).toContainText("Heartbeat");
    await expect(messages).toContainText("RPC Echo Request");
    await expect(messages).toContainText("Another regular message");
  });
});

test.describe("Cross-Platform User Journey Tests", () => {
  test("should work on desktop browsers", async ({ page, browserName }) => {
    await page.goto("/test-app.html");

    // Test basic functionality
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    await page.fill("#message-input", `Desktop ${browserName} test`);
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");
  });

  test("should work on mobile browsers", async ({ page, browserName }) => {
    test.skip(!browserName.includes("Mobile"));

    await page.goto("/test-app.html");

    // Test basic functionality on mobile
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    await page.fill("#message-input", `Mobile ${browserName} test`);
    await page.click("#send-btn");
    await expect(page.locator("#messages-sent")).toHaveText("1");
  });
});
