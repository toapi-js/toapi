import { type Observable } from "@toapi/common";
import * as React from "react";

type ObservablePromise<T> = Promise<T> & Observable<T>;

interface Options {
  startTransition?: typeof React.startTransition;
}

export function useQuery<T>(
  // TODO remove the useless function-form in next major release
  query: ObservablePromise<T> | (() => ObservablePromise<T>),
  { startTransition = React.startTransition }: Options = {},
) {
  const observable = typeof query === "function" ? query() : query;

  const queryKey = observable.queryKey ?? null;
  const [state, setState] = React.useState<{
    promise: Promise<T>;
    queryKey: string | null;
    value: T;
  } | null>(null);

  React.useEffect(() => {
    let active = true;
    const unsubscribe = observable.subscribe((next) => {
      startTransition(async () => {
        try {
          const value = await next;
          // A late update from a subscription we have already left behind must
          // not overwrite the current one.
          if (active) setState({ promise: next, queryKey, value });
        } catch {}
      });
    });
    return () => {
      active = false;
      unsubscribe();
    };
  }, [queryKey, startTransition]);

  if (!state) {
    return React.use(observable);
  }

  if (state.queryKey !== observable.queryKey) {
    return React.use(observable);
  }

  if (state.promise === observable) {
    return React.use(observable);
  }

  return state.value;
}
