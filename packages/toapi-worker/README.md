# @toapi/worker

Service-worker request handling for Toapi: offline-capable caching and remote tag revalidation, working alongside [@toapi/server](https://www.npmjs.com/package/@toapi/server)'s caching and [@toapi/client](https://www.npmjs.com/package/@toapi/client).

📚 [Docs](https://toapi-js.github.io/toapi/worker/)

## Installation

```bash
npm install @toapi/worker
```

## Quick start

```ts
// service-worker.ts
import { setupToapiWorker } from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope;

setupToapiWorker();
```
