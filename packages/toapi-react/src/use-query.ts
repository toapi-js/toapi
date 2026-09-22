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
  const queryKey = observable.queryKey;
  const initialObservable = React.useMemo(() => observable, [queryKey]);

  const [state, setState] = React.useState<{
    promise: Promise<T>;
    queryKey: string;
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

  if (state.queryKey !== initialObservable.queryKey) {
    return React.use(initialObservable);
  }

  if (state.promise === initialObservable) {
    return React.use(initialObservable);
  }

  return state.value;
}
