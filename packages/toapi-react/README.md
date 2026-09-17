# @toapi/react

React-Binding for [Toapi](https://www.npmjs.com/package/@toapi/client)

📚 [Docs](https://toapi-js.github.io/toapi/react/)

## Installation
```bash
npm install @toapi/react
yarn add @toapi/react
pnpm add @toapi/react
bun add @toapi/react
```

## Usage
```tsx
import { useQuery } from '@toapi/react';
import { client } from "/path/to/tapi-client";
import { use } from 'react';

function Page() {
  const booksPromise = useQuery(client.books.get())
  const books = use(booksPromise)

  return (
    <div>
      {books.map(item => (
        <div key={item.id}>{item.name}</div>
      ))}
    </div>
  );
}
```

## What does this do?
It subscribes Toapi's internal PubSub system to get updates on the data when it is invalidated through TApi's tag-based cache invalication mechanism or manually using the revalidate methods of the TApi client, e.G. `client.books.revalidate()`.

Note that `useQuery` actaully returns a `Promise` which can be passed to client components for granular control of loading states using `Suspense`-Boundaries.
