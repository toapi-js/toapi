import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { PubSub } from "./pub-sub.js";

describe("PubSub invalidation debounce", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  test.each([
    { initial: ["/a"], pending: ["/a"] },
    { initial: ["/a", "/b"], pending: ["/a"] },
    { initial: ["/a", "/b"], pending: ["/a", "/b"] },
  ])("replays pending URLs once for $initial / $pending", async ({ initial, pending }) => {
    const pubSub = new PubSub({ minTTL: 2000 });
    const notifications: string[][] = [];
    pubSub.subscribe(async (urls) => {
      if (urls.size) notifications.push([...urls]);
    });

    await pubSub.publish(new Set(initial));
    await vi.advanceTimersByTimeAsync(400);
    await pubSub.publish(new Set(pending));
    await pubSub.publish(new Set(pending));
    expect(notifications).toEqual([initial]);

    await vi.advanceTimersByTimeAsync(1600);
    expect(notifications).toEqual([initial, pending]);
    await vi.advanceTimersByTimeAsync(20000);
    expect(notifications).toEqual([initial, pending]);
    expect(vi.getTimerCount()).toBe(0);

    // A later invalidation batch must still be delivered and debounced.
    await pubSub.publish(new Set(initial));
    await pubSub.publish(new Set(pending));
    await vi.advanceTimersByTimeAsync(4000);
    expect(notifications).toEqual([initial, pending, initial, pending]);
    expect(vi.getTimerCount()).toBe(0);
  });
});
