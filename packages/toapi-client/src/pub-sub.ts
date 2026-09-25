type Subscription = (urls: Set<string>) => Promise<void>;

interface Options {
  minTTL: number;
}

export class PubSub {
  private subscriptions = new Set<Subscription>();
  private debounceTimeouts = new Map<string, ReturnType<typeof setTimeout>>();
  private requestedUrls = new Set<string>();
  private minTTL: number;

  constructor(options: Options) {
    this.minTTL = options.minTTL;
  }

  subscribe(callback: Subscription) {
    this.subscriptions.add(callback);
    return () => {
      this.subscriptions.delete(callback);
    };
  }

  async publish(urls: Set<string>, force: boolean) {
    if (force) {
      for (const url of urls) {
        const timeout = this.debounceTimeouts.get(url);
        clearTimeout(timeout);
        this.debounceTimeouts.delete(url);
      }
      await Promise.all(
        this.subscriptions.values().map((callback) => callback(urls)),
      );
      return;
    }

    for (const url of urls) {
      if (this.debounceTimeouts.has(url)) {
        // debounced, mark as requested and ignore now
        this.requestedUrls.add(url);
        urls.delete(url);
      }
    }

    const timeout = setTimeout(() => {
      const pendingUrls = new Set<string>();
      for (const url of urls) {
        this.debounceTimeouts.delete(url);
        if (this.requestedUrls.delete(url)) {
          pendingUrls.add(url);
        }
      }
      // Consume pending invalidations and unlock the entire batch before replay.
      if (pendingUrls.size) return this.publish(pendingUrls, false);
    }, this.minTTL);

    for (const url of urls) {
      this.debounceTimeouts.set(url, timeout);
    }

    await Promise.all(
      this.subscriptions.values().map((callback) => callback(urls)),
    );
  }
}
