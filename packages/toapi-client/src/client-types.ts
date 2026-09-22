import type {
  Handler as BaseHandler,
  Path as BasePath,
  BaseRoute,
  MaybePromise,
  Observable,
} from "@toapi/common";

type Segment<Path> = Path extends `/${infer Segment}/${string}`
  ? Segment
  : Path extends `/${infer Segment}`
    ? Segment
    : never;

type Rest<
  Path,
  Segment extends string,
> = Path extends `/${Segment}/${infer Rest}` ? `/${Rest}` : never;

export type Client<Routes extends Record<BasePath, MaybePromise<BaseRoute>>> = {
  [
    segment in Segment<keyof Routes> as segment extends `:${string}`
      ? string | number
      : segment extends `*${string}`
        ? string | number
        : segment
  ]: (Extract<keyof Routes, `/${segment}`> extends never
    ? {}
    : ClientRoute<Routes[`/${segment}`]>) &
    (Exclude<keyof Routes, `/${segment}`> extends never
      ? {}
      : Client<{
          [rest in Rest<keyof Routes, segment>]: Routes[`/${segment}${rest}`];
        }>);
};

type ClientRoute<Route extends MaybePromise<BaseRoute>> = {
  get: QueryWithoutBody<Awaited<Route>["GET"]>;
  post: MutationWithBody<Awaited<Route>["POST"]>;
  delete: MutationWithoutBody<Awaited<Route>["DELETE"]>;
  put: MutationWithBody<Awaited<Route>["PUT"]>;
  patch: MutationWithBody<Awaited<Route>["PATCH"]>;
  revalidate: () => Promise<void>;
};

export type QueryWithoutBody<
  Handler extends BaseHandler<any, any, any, undefined> | undefined,
> = Handler extends undefined
  ? never
  : RequiredKeys<QueryType<Handler>> extends never
    ? (
        query?: Partial<QueryType<Handler>>,
        req?: RequestInit,
      ) => Promise<ResponseType<Handler>> & Observable<ResponseType<Handler>>
    : (
        query: OptionalizeUndefined<QueryType<Handler>>,
        req?: RequestInit,
      ) => Promise<ResponseType<Handler>> & Observable<ResponseType<Handler>>;

export type Revalidating<T> = {
  revalidated: Promise<T>;
};

export type MutationWithoutBody<
  Handler extends BaseHandler<any, any, any, undefined> | undefined,
> = Handler extends undefined
  ? never
  : RequiredKeys<QueryType<Handler>> extends never
    ? (
        query?: Partial<QueryType<Handler>>,
        req?: RequestInit,
      ) => Promise<ResponseType<Handler>> & Revalidating<ResponseType<Handler>>
    : (
        query: OptionalizeUndefined<QueryType<Handler>>,
        req?: RequestInit,
      ) => Promise<ResponseType<Handler>> & Revalidating<ResponseType<Handler>>;

export type MutationWithBody<
  Handler extends BaseHandler<any, any, any, unknown> | undefined,
> = Handler extends undefined
  ? never
  : (
      body?: BodyType<Handler> | FormData,
      req?: RequestInit & { query?: OptionalizeUndefined<QueryType<Handler>> },
    ) => Promise<ResponseType<Handler>> & Revalidating<ResponseType<Handler>>;

type QueryType<Handler extends { schema: { __q?: any } } | undefined> =
  NonNullable<NonNullable<Handler>["schema"]["__q"]>;

type RequiredKeys<T> = {
  [K in keyof T]-?: undefined extends T[K] ? never : K;
}[keyof T];

type OptionalizeUndefined<T> = {
  [K in keyof T as undefined extends T[K] ? never : K]: T[K];
} & {
  [K in keyof T as undefined extends T[K] ? K : never]?: T[K];
};

type BodyType<Handler extends { schema: { __b?: any } } | undefined> =
  NonNullable<NonNullable<Handler>["schema"]["__b"]>;

type ResponseType<Handler extends { schema: { __r?: any } } | undefined> =
  NonNullable<NonNullable<Handler>["schema"]["__r"]>;
