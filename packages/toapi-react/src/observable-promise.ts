import type { Observable } from "@toapi/common";

export type ObservablePromise<T> = Promise<T> & Observable<T>;
