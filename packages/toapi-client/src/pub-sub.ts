type Subscription = (urls: Set<string>) => Promise<void>;

const DEFAULT_MIN_TTL = 5 * 1000;

interface Options {
  minTTL?: number;
}

export class PubSub {
  private subscriptions = new Set<Subscription>();
  private debounceTimeouts = new Map<string, NodeJS.Timeout>();
  private minTTL: number;

  constructor(options: Options) {
    this.minTTL = options.minTTL ?? DEFAULT_MIN_TTL;
  }

  subscribe(callback: Subscription) {
    this.subscriptions.add(callback);
    return () => {
      this.subscriptions.delete(callback);
    };
  }

  async publish(urls: Set<string>) {
    for (const url of urls) {
      if (this.debounceTimeouts.has(url)) {
        urls.delete(url);
      }
    }

    const timeout = setTimeout(() => {
      for (const url of urls) {
        this.debounceTimeouts.delete(url);
      }
      this.publish(urls);
    }, this.minTTL);

    for (const url of urls) {
      this.debounceTimeouts.set(url, timeout);
    }

    await Promise.all(
      this.subscriptions.values().map((callback) => callback(urls)),
    );
  }
}
