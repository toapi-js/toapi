import { HttpError, type Observable } from "@toapi/common";
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
