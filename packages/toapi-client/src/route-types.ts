import type { Observable } from "@toapi/common";

export type GetRoute<R, Q = undefined> = Q extends undefined
  ? (query?: {}, req?: RequestInit) => Promise<R> & Observable<R>
  : (query: Q, req?: RequestInit) => Promise<R> & Observable<R>;

type T = GetRoute<string>;

export type PostRoute<R, Q = unknown, B = unknown> = (
  query: Q | FormData,
  body: B | FormData,
  req?: RequestInit,
) => Promise<R>;
