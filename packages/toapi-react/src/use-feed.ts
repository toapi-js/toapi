import { type Observable } from "@toapi/common";
import * as React from "react";
import { runTransition, type TransitionProp } from "./transition";

type ObservablePromise<T> = Promise<T> & Observable<T>;

interface Options {
  transition?: TransitionProp;
  onStatusChange?(status: "loading" | "idle"): void;
}

type Cursor = string | number | Date;
type WithCursor = { cursor: Cursor };

function useObservable<T>(observable: ObservablePromise<T>) {
  return React.useMemo(() => observable, [observable.queryKey]);
}

type QueueItem<T> = { position: "start" | "end"; promise: Promise<T[]> };

export function useFeed<T extends WithCursor>(
  query: (cursor?: Cursor) => ObservablePromise<T[]>,
  { transition = false, onStatusChange }: Options = {},
) {
  const topPage = useObservable(query());
  const [queue, setQueue] = React.useState<QueueItem<T>[]>([]);
  const [entries, setEntries] = React.useState<T[]>([]);

  React.useEffect(() => {
    const unsubscribe = topPage.subscribe(async (next) => {
      setQueue((queue) => [...queue, { position: "end", promise: next }]);
    });
    return () => unsubscribe();
  }, [topPage]);

  const nextItem = queue[0];

  React.useEffect(() => {
    if (!nextItem) {
      onStatusChange?.("idle");
      return;
    }
    onStatusChange?.("loading");
    nextItem.promise.then((page) => {
      if (nextItem.position === "end") {
        
      }
    });
    setQueue((queue) => queue.slice(1));
  }, [nextItem, transition]);

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
