import type { MaybePromise } from "@toapi/common";
import type { Logger } from "@toapi/common";
import type { Path as BasePath, StrictParams } from "@toapi/common";
import type { Route } from "@toapi/common";
import { type Cache, PubSub } from "./cache.js";
import type { RevalidationStreamConfig } from "./revalidation-stream.js";

export interface OasInfo {
  title: string;
  version: string;
}

interface Options {
  cache?: Cache;
  oas?: OasInfo;
  logger?: Logger;
  revalidationStream?: RevalidationStreamConfig;
}

export function defineApi(options: Options = {}) {
  return new ApiDefinition(
    {},
    options?.cache ?? new PubSub(),
    options?.oas,
    options?.logger,
  );
}

export class ApiDefinition<Routes extends Record<BasePath, unknown>> {
  constructor(
    public routes: Routes,
    public cache: Cache,
    public oas?: OasInfo,
    public logger?: Logger,
    public revalidationStreamConfig?: RevalidationStreamConfig,
  ) {}

  async invalidate(tags: string[]) {
    await this.cache.delete(tags);
  }

  route<
    Path extends BasePath,
    GetResponse = never,
    GetQuery extends Record<string, unknown> = never,
    PostResponse = never,
    PostQuery extends Record<string, unknown> = never,
    PostBody = never,
    DeleteResponse = never,
    DeleteQuery extends Record<string, unknown> = never,
    PutResponse = never,
    PutQuery extends Record<string, unknown> = never,
    PutBody = never,
    PatchResponse = never,
    PatchQuery extends Record<string, unknown> = never,
    PatchBody = never,
  >(
    path: Path,
    route: MaybePromise<
      Route<
        StrictParams<Path>,
        GetResponse,
        GetQuery,
        PostResponse,
        PostQuery,
        PostBody,
        DeleteResponse,
        DeleteQuery,
        PutResponse,
        PutQuery,
        PutBody,
        PatchResponse,
        PatchQuery,
        PatchBody
      >
    >,
  ) {
    (this.routes[path] as any) = route;
    return this as unknown as ApiDefinition<
      Routes & {
        [path in Path]: MaybePromise<
          Route<
            StrictParams<Path>,
            GetResponse,
            GetQuery,
            PostResponse,
            PostQuery,
            PostBody,
            DeleteResponse,
            DeleteQuery,
            PutResponse,
            PutQuery,
            PutBody,
            PatchResponse,
            PatchQuery,
            PatchBody
          >
        >;
      }
    >;
  }
}
