import { test, expect } from "@playwright/test";

test.describe("WebSocket Connection Tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/test-app.html");
  });

  test("should display disconnected status initially", async ({ page }) => {
    const status = page.locator("#status");
    await expect(status).toHaveText("Disconnected");
    await expect(status).toHaveClass(/disconnected/);
  });

  test("should connect to WebSocket server", async ({ page }) => {
    // Set server URL to our test server
    await page.fill("#server-url", "ws://localhost:8080");

    // Click connect button
    await page.click("#connect-btn");

    // Wait for connection status to change
    await expect(page.locator("#status")).toHaveText("Connected");
    await expect(page.locator("#status")).toHaveClass(/connected/);

    // Verify UI state changes
    await expect(page.locator("#connect-btn")).toBeDisabled();
    await expect(page.locator("#disconnect-btn")).toBeEnabled();
    await expect(page.locator("#message-input")).toBeEnabled();
  });

  test("should disconnect from WebSocket server", async ({ page }) => {
    // Connect first
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Disconnect
    await page.click("#disconnect-btn");
    await expect(page.locator("#status")).toHaveText("Disconnected");

    // Verify UI state changes
    await expect(page.locator("#connect-btn")).toBeEnabled();
    await expect(page.locator("#disconnect-btn")).toBeDisabled();
    await expect(page.locator("#message-input")).toBeDisabled();
  });

  test("should send and receive messages", async ({ page }) => {
    // Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Send a message
    await page.fill("#message-input", "Hello, Server!");
    await page.click("#send-btn");

    // Verify message was sent
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Check that message appears in messages list
    const messages = page.locator("#messages .message.sent");
    await expect(messages).toContainText("Hello, Server!");
  });

  test("should send heartbeat messages", async ({ page }) => {
    // Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Send heartbeat
    await page.click("#send-heartbeat-btn");

    // Verify heartbeat was sent
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Check that heartbeat appears in messages list
    const messages = page.locator("#messages .message.sent");
    await expect(messages).toContainText("Heartbeat");
  });

  test("should handle connection errors gracefully", async ({ page }) => {
    // Try to connect to invalid server
    await page.fill("#server-url", "ws://invalid-server:9999");
    await page.click("#connect-btn");

    // Should show error or disconnected status
    await expect(page.locator("#status")).toHaveClass(/disconnected/);
  });

  test("should update connection metrics", async ({ page }) => {
    // Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Send a message
    await page.fill("#message-input", "Test message");
    await page.click("#send-btn");

    // Verify metrics are updated
    await expect(page.locator("#messages-sent")).toHaveText("1");
    await expect(page.locator("#connection-quality")).toHaveText("100%");
  });

  test("should clear messages", async ({ page }) => {
    // Connect and send a message
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await page.fill("#message-input", "Test message");
    await page.click("#send-btn");

    // Verify message exists
    await expect(page.locator("#messages .message")).toHaveCount(1);

    // Clear messages
    await page.click("#clear-messages-btn");

    // Verify messages are cleared
    await expect(page.locator("#messages .message")).toHaveCount(0);
  });
});

test.describe("RPC Functionality Tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/test-app.html");
    // Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");
  });

  test("should send RPC echo request", async ({ page }) => {
    await page.click("#rpc-echo-btn");

    // Verify RPC request was sent
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Check that RPC request appears in messages
    const messages = page.locator("#messages .message.sent");
    await expect(messages).toContainText("RPC Echo Request");
  });

  test("should send RPC broadcast request", async ({ page }) => {
    await page.click("#rpc-broadcast-btn");

    // Verify RPC request was sent
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Check that RPC request appears in messages
    const messages = page.locator("#messages .message.sent");
    await expect(messages).toContainText("RPC Broadcast Request");
  });

  test("should send RPC get stats request", async ({ page }) => {
    await page.click("#rpc-stats-btn");

    // Verify RPC request was sent
    await expect(page.locator("#messages-sent")).toHaveText("1");

    // Check that RPC request appears in messages
    const messages = page.locator("#messages .message.sent");
    await expect(messages).toContainText("RPC Get Stats Request");
  });
});

test.describe("Connection Resilience Tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/test-app.html");
  });

  test("should handle reconnection", async ({ page }) => {
    // Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Disconnect
    await page.click("#disconnect-btn");
    await expect(page.locator("#status")).toHaveText("Disconnected");

    // Reconnect
    await page.click("#reconnect-btn");

    // Wait for reconnection
    await expect(page.locator("#status")).toHaveText("Connected");

    // Verify reconnect count increased
    await expect(page.locator("#reconnect-count")).toHaveText("1");
  });

  test("should handle multiple reconnections", async ({ page }) => {
    // Connect to server
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");

    // Perform multiple reconnections
    for (let i = 0; i < 3; i++) {
      await page.click("#reconnect-btn");
      await expect(page.locator("#status")).toHaveText("Connected");
    }

    // Verify reconnect count
    await expect(page.locator("#reconnect-count")).toHaveText("3");
  });
});

test.describe("Cross-Browser Compatibility", () => {
  test("should work in Chrome", async ({ page, browserName }) => {
    test.skip(browserName !== "chromium");

    await page.goto("/test-app.html");
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");
  });

  test("should work in Firefox", async ({ page, browserName }) => {
    test.skip(browserName !== "firefox");

    await page.goto("/test-app.html");
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");
  });

  test("should work in Safari", async ({ page, browserName }) => {
    test.skip(browserName !== "webkit");

    await page.goto("/test-app.html");
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");
  });
});

test.describe("Mobile Compatibility", () => {
  test("should work on mobile Chrome", async ({ page, browserName }) => {
    test.skip(browserName !== "Mobile Chrome");

    await page.goto("/test-app.html");
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");
  });

  test("should work on mobile Safari", async ({ page, browserName }) => {
    test.skip(browserName !== "Mobile Safari");

    await page.goto("/test-app.html");
    await page.fill("#server-url", "ws://localhost:8080");
    await page.click("#connect-btn");
    await expect(page.locator("#status")).toHaveText("Connected");
  });
});
