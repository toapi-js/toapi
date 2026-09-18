import React from "react";
import type { ObservablePromise } from "../observable-promise";

interface Options {
  startTransition?: typeof React.startTransition;
}

type Page<T> = { items: T[]; observable: ObservablePromise<T> };

export function useCursorQuery<T extends { cursor: string }>(
  query: (cursor: string | null) => ObservablePromise<T[]>,
  { startTransition = React.startTransition }: Options = [],
) {
  const [pages, setPages] = React.useState<Page<T>[]>([]);

  return React.useMemo(() => [pages.flatMap((page) => page.items)], [pages]);
}
