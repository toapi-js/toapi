import type { Observable } from "@toapi/common";
import * as React from "react";

type ObservablePromise<T> = Promise<T> & Observable<T>;

interface Options {
  startTransition?: typeof React.startTransition;
}

export function useQuery<T>(
  query: ObservablePromise<T> | (() => ObservablePromise<T>),
  { startTransition = React.startTransition }: Options = {},
) {
  const observable = React.useMemo(
    typeof query === "function" ? query : () => query,
    [query],
  );
  // The client keeps queryKey stable for the lifetime of a cached query,
  // while each refresh returns a new promise. Comparing promises would mistake
  // an inline factory's next render for a query switch and suspend on refresh.
  // Custom observables without a key retain their existing identity semantics.
  const source = observable.queryKey ?? observable;
  const [state, setState] = React.useState<{
    source: unknown;
    value: T;
  } | null>(null);

  React.useEffect(() => {
    let active = true;
    const unsubscribe = observable.subscribe((next) => {
      startTransition(async () => {
        const value = await next;
        // A late update from a subscription we have already left behind must
        // not overwrite the current one.
        if (active) setState({ source, value });
      });
    });
    return () => {
      active = false;
      unsubscribe();
    };
  }, [source, startTransition]);

  return state !== null && state.source === source
    ? state.value
    : React.use(observable);
}
