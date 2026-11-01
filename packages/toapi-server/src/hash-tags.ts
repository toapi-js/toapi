import { TAGS_HEADER, TResponse, type MaybePromise } from "@toapi/common";

export async function hashTag(te: TextEncoder, secret: string, tag: string) {
  const digest = await crypto.subtle.digest("SHA-256", te.encode(secret + tag));
  const dv = new DataView(digest);
  const num = dv.getBigUint64(0);
  return num.toString(36);
}

type Encode = (tags: string[]) => MaybePromise<Uint8Array>;

export function hashEncoder(secret: string): Encode {
  const te = new TextEncoder();
  return async (tags) => {
    if (!tags.length) return te.encode("\n");
    const hashedTags = await Promise.all(
      tags.map((tag) => hashTag(te, secret, tag)),
    );
    const tagsString = hashedTags.join(" ");
    return te.encode(tagsString + "\n");
  };
}

export function textEncoder(): Encode {
  const te = new TextEncoder();
  return (tags) => {
    if (!tags.length) return te.encode("\n");
    const tagsString = tags.join(" ");
    return te.encode(tagsString + "\n");
  };
}

export async function encodeTags(
  secret: string | undefined,
  res: Response & { cache?: TResponse<unknown>["cache"] },
) {
  if (!secret) return;
  const tags =
    res.cache?.tags ?? res.headers.get(TAGS_HEADER)?.split(" ") ?? [];

  const te = new TextEncoder();
  const hashedTags = await Promise.all(
    tags.map((tag) => hashTag(te, secret, tag)),
  );

  res.headers.set(TAGS_HEADER, hashedTags.join(" "));
}
