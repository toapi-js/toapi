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

  async publish(urls: Set<string>) {
    for (const url of urls) {
      if (this.debounceTimeouts.has(url)) {
        // debounced, mark as requested and ignore now
        this.requestedUrls.add(url);
        urls.delete(url);
      }
    }

    const timeout = setTimeout(
      () =>
        Promise.all(
          Array.from(urls).map((url) => {
            if (this.requestedUrls.has(url)) {
              this.debounceTimeouts.delete(url);
              // needs to be refetched
              return this.publish(urls);
            } else {
              // debounce timeout over
              this.debounceTimeouts.delete(url);
            }
          }),
        ),
      this.minTTL,
    );

    for (const url of urls) {
      this.debounceTimeouts.set(url, timeout);
    }

    await Promise.all(
      this.subscriptions.values().map((callback) => callback(urls)),
    );
  }
}
