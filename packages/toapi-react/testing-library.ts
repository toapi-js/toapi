import { afterEach, expect } from "vitest";
import { cleanup } from "@testing-library/react";
import * as matchers from "@testing-library/jest-dom/matchers";

expect.extend(matchers);

// @ts-ignore no idea why this is necessary
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

// Optional: cleans up `render` after each test
afterEach(() => {
  cleanup();
});
