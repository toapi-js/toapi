import { EXPIRES_AT_HEADER, TAGS_HEADER } from "@toapi/common";

export interface Metadata {
  tags: Set<string>;
  expiresAt?: number;
}

export function extractMetadata(response: Response): Metadata {
  const expiresAtHeader = response.headers.get(EXPIRES_AT_HEADER);
  return {
    tags: new Set(response.headers.get(TAGS_HEADER)?.split(" ") ?? []),
    expiresAt: expiresAtHeader ? parseInt(expiresAtHeader, 10) : undefined,
  };
}
