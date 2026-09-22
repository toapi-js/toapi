export type Observable<T> = {
  readonly queryKey: string;
  subscribe(callback: (value: Promise<T>) => void): () => void;
};
