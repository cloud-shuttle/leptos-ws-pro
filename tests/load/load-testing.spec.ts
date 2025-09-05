import { test, expect } from '@playwright/test';

test.describe('Load Testing and Performance', () => {
  test('should handle high message throughput', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Send many messages rapidly
    const messageCount = 50;
    const startTime = Date.now();
    
    for (let i = 0; i < messageCount; i++) {
      await page.fill('#message-input', `Load test message ${i}`);
      await page.click('#send-btn');
      
      // Small delay to prevent overwhelming the UI
      if (i % 10 === 0) {
        await page.waitForTimeout(10);
      }
    }
    
    const endTime = Date.now();
    const duration = endTime - startTime;
    
    // Verify all messages were sent
    await expect(page.locator('#messages-sent')).toHaveText(messageCount.toString());
    
    // Verify connection is still stable
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Log performance metrics
    console.log(`Sent ${messageCount} messages in ${duration}ms (${(messageCount / duration * 1000).toFixed(2)} msg/s)`);
  });

  test('should handle large message payloads', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Test different message sizes
    const messageSizes = [1000, 10000, 50000]; // 1KB, 10KB, 50KB
    
    for (const size of messageSizes) {
      const largeMessage = 'A'.repeat(size);
      await page.fill('#message-input', largeMessage);
      await page.click('#send-btn');
      
      // Verify message was sent
      await expect(page.locator('#messages-sent')).toHaveText('1');
      
      // Clear for next test
      await page.click('#clear-messages-btn');
    }
    
    // Verify connection is still stable
    await expect(page.locator('#status')).toHaveText('Connected');
  });

  test('should handle concurrent message sending', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Send messages in rapid succession
    const promises = [];
    const messageCount = 20;
    
    for (let i = 0; i < messageCount; i++) {
      promises.push(
        page.fill('#message-input', `Concurrent message ${i}`).then(() =>
          page.click('#send-btn')
        )
      );
    }
    
    // Wait for all messages to be sent
    await Promise.all(promises);
    
    // Verify all messages were sent
    await expect(page.locator('#messages-sent')).toHaveText(messageCount.toString());
    
    // Verify connection is still stable
    await expect(page.locator('#status')).toHaveText('Connected');
  });

  test('should handle RPC load testing', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Send multiple RPC requests
    const rpcCount = 10;
    const startTime = Date.now();
    
    for (let i = 0; i < rpcCount; i++) {
      await page.click('#rpc-echo-btn');
      await page.waitForTimeout(50); // Small delay between requests
    }
    
    const endTime = Date.now();
    const duration = endTime - startTime;
    
    // Verify all RPC requests were sent
    await expect(page.locator('#messages-sent')).toHaveText(rpcCount.toString());
    
    // Verify connection is still stable
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Log performance metrics
    console.log(`Sent ${rpcCount} RPC requests in ${duration}ms (${(rpcCount / duration * 1000).toFixed(2)} req/s)`);
  });

  test('should handle heartbeat stress testing', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Send many heartbeats rapidly
    const heartbeatCount = 30;
    const startTime = Date.now();
    
    for (let i = 0; i < heartbeatCount; i++) {
      await page.click('#send-heartbeat-btn');
      await page.waitForTimeout(100); // 100ms between heartbeats
    }
    
    const endTime = Date.now();
    const duration = endTime - startTime;
    
    // Verify all heartbeats were sent
    await expect(page.locator('#messages-sent')).toHaveText(heartbeatCount.toString());
    
    // Verify connection is still stable
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Log performance metrics
    console.log(`Sent ${heartbeatCount} heartbeats in ${duration}ms (${(heartbeatCount / duration * 1000).toFixed(2)} hb/s)`);
  });

  test('should handle connection quality degradation', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Monitor connection quality over time
    const initialQuality = await page.locator('#connection-quality').textContent();
    expect(initialQuality).toBe('100%');
    
    // Send messages continuously to simulate load
    for (let i = 0; i < 20; i++) {
      await page.fill('#message-input', `Quality test message ${i}`);
      await page.click('#send-btn');
      await page.waitForTimeout(200);
      
      // Check quality periodically
      if (i % 5 === 0) {
        const quality = await page.locator('#connection-quality').textContent();
        console.log(`Connection quality at message ${i}: ${quality}`);
      }
    }
    
    // Verify connection is still stable
    await expect(page.locator('#status')).toHaveText('Connected');
  });

  test('should handle memory usage under load', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Send many messages to test memory usage
    const messageCount = 100;
    
    for (let i = 0; i < messageCount; i++) {
      await page.fill('#message-input', `Memory test message ${i} - ${'A'.repeat(100)}`);
      await page.click('#send-btn');
      
      // Clear messages periodically to test memory management
      if (i % 20 === 0 && i > 0) {
        await page.click('#clear-messages-btn');
      }
    }
    
    // Verify connection is still stable
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Verify we can still send messages
    await page.fill('#message-input', 'Post-load test message');
    await page.click('#send-btn');
    await expect(page.locator('#messages-sent')).toHaveText('1');
  });

  test('should handle network interruption simulation', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Send some messages
    await page.fill('#message-input', 'Message before interruption');
    await page.click('#send-btn');
    await expect(page.locator('#messages-sent')).toHaveText('1');
    
    // Simulate network interruption by disconnecting
    await page.click('#disconnect-btn');
    await expect(page.locator('#status')).toHaveText('Disconnected');
    
    // Try to send message during interruption (should fail gracefully)
    await page.fill('#message-input', 'Message during interruption');
    await page.click('#send-btn');
    
    // Reconnect
    await page.click('#reconnect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    await expect(page.locator('#reconnect-count')).toHaveText('1');
    
    // Send message after reconnection
    await page.fill('#message-input', 'Message after reconnection');
    await page.click('#send-btn');
    await expect(page.locator('#messages-sent')).toHaveText('2');
  });

  test('should handle rapid connect/disconnect cycles under load', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Perform multiple connect/disconnect cycles
    const cycleCount = 5;
    
    for (let cycle = 0; cycle < cycleCount; cycle++) {
      // Connect
      await page.fill('#server-url', 'ws://localhost:8080');
      await page.click('#connect-btn');
      await expect(page.locator('#status')).toHaveText('Connected');
      
      // Send messages during connection
      for (let i = 0; i < 5; i++) {
        await page.fill('#message-input', `Cycle ${cycle} message ${i}`);
        await page.click('#send-btn');
      }
      
      // Disconnect
      await page.click('#disconnect-btn');
      await expect(page.locator('#status')).toHaveText('Disconnected');
      
      await page.waitForTimeout(100);
    }
    
    // Final connect and verify state
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Verify we can still send messages
    await page.fill('#message-input', 'Final load test message');
    await page.click('#send-btn');
    await expect(page.locator('#messages-sent')).toHaveText('1');
  });
});

test.describe('Performance Monitoring', () => {
  test('should track message throughput metrics', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Send messages and track timing
    const messageCount = 25;
    const startTime = Date.now();
    
    for (let i = 0; i < messageCount; i++) {
      await page.fill('#message-input', `Throughput test ${i}`);
      await page.click('#send-btn');
    }
    
    const endTime = Date.now();
    const totalTime = endTime - startTime;
    const throughput = (messageCount / totalTime) * 1000; // messages per second
    
    // Verify all messages were sent
    await expect(page.locator('#messages-sent')).toHaveText(messageCount.toString());
    
    // Log performance metrics
    console.log(`Message throughput: ${throughput.toFixed(2)} messages/second`);
    console.log(`Total time: ${totalTime}ms for ${messageCount} messages`);
    
    // Verify connection is still stable
    await expect(page.locator('#status')).toHaveText('Connected');
  });

  test('should monitor connection stability over time', async ({ page }) => {
    await page.goto('/test-app.html');
    
    // Connect to server
    await page.fill('#server-url', 'ws://localhost:8080');
    await page.click('#connect-btn');
    await expect(page.locator('#status')).toHaveText('Connected');
    
    // Monitor connection for extended period
    const monitoringDuration = 10000; // 10 seconds
    const startTime = Date.now();
    let messageCount = 0;
    
    while (Date.now() - startTime < monitoringDuration) {
      await page.fill('#message-input', `Stability test ${messageCount}`);
      await page.click('#send-btn');
      messageCount++;
      
      // Check connection status periodically
      const status = await page.locator('#status').textContent();
      expect(status).toBe('Connected');
      
      await page.waitForTimeout(500);
    }
    
    // Verify connection remained stable
    await expect(page.locator('#status')).toHaveText('Connected');
    await expect(page.locator('#messages-sent')).toHaveText(messageCount.toString());
    
    console.log(`Connection remained stable for ${monitoringDuration}ms, sent ${messageCount} messages`);
  });
});
