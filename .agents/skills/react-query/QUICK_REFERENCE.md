# React Query Quick Reference

## Setup ✅ (Already Done)

- QueryClient configured in `/src/providers/react-query.tsx`
- ReactQueryProvider wraps app in `/src/routes/__root.tsx`
- Global defaults: 5min staleTime, 10min gcTime

## Pattern: Add React Query to a Service

### 1. Service File (`/src/services/example.service.ts`)

```typescript
import type { UseQueryOptions, UseMutationOptions } from '@tanstack/react-query';
import { api } from '@/lib/api';

// Function
export async function getExample(id: string) {
  const res = await api.get('/example/' + id);
  return res;
}

// Options (right below function)
export function getExampleOptions(id: string): UseQueryOptions<any, Error> {
  return {
    queryKey: ['example', id],
    queryFn: async () => {
      const res = await getExample(id);
      if (!res.data) throw new Error(res.error?.message || 'Failed to fetch');
      return res.data;
    },
    staleTime: 5 * 60 * 1000,
  };
}
```

### 2. Custom Hook (`/src/hooks/use-example-query.ts`)

```typescript
import { useQuery } from '@tanstack/react-query';
import { getExampleOptions } from '@/services/example.service';

export function useExampleQuery(id: string) {
  return useQuery({
    ...getExampleOptions(id),
    enabled: !!id,
  });
}
```

### 3. Use in Component

```typescript
import { useExampleQuery } from '@/hooks/use-example-query';

function MyComponent({ id }: { id: string }) {
  const { data, isLoading, error } = useExampleQuery(id);
  
  if (isLoading) return <Loading />;
  if (error) return <Error error={error} />;
  
  return <div>{data?.name}</div>;
}
```

## Common Patterns

### Query (Read Data)

```typescript
const { data, isLoading, error, refetch } = useQuery(queryOptions);
```

### Mutation (Write Data)

```typescript
const { mutate, isPending } = useMutation({
  ...mutationOptions,
  onSuccess: (data) => {
    queryClient.invalidateQueries({ queryKey: ['items'] });
  },
  onError: (error) => {
    console.error(error);
  },
});

mutate({ id: '123', name: 'New Name' });
```

### Prefetch (Warm Cache)

```typescript
const queryClient = useQueryClient();

queryClient.prefetchQuery({
  ...getExampleOptions(id),
});
```

### Invalidate (Force Refetch)

```typescript
const queryClient = useQueryClient();

// Invalidate all
queryClient.invalidateQueries({ queryKey: ['example'] });

// Invalidate specific
queryClient.invalidateQueries({ queryKey: ['example', id] });
```

### Set Cache Directly

```typescript
const queryClient = useQueryClient();

queryClient.setQueryData(['example', id], newData);
```

## Naming Convention

| Action | Service | Options | Hook |
|--------|---------|---------|------|
| Get One | `getUser(id)` | `getUserOptions(id)` | `useUserQuery(id)` |
| Get Many | `getUsers()` | `getUsersOptions()` | `useUsersQuery()` |
| Create | `createUser(data)` | `createUserOptions()` | N/A (use mutation) |
| Update | `updateUser(id, data)` | `updateUserOptions()` | N/A (use mutation) |
| Delete | `deleteUser(id)` | `deleteUserOptions()` | N/A (use mutation) |

## State States

```typescript
const { 
  data,              // The fetched data
  isLoading,         // true while fetching
  isError,           // true if error occurred
  error,             // Error object if isError
  status,            // 'pending' | 'error' | 'success'
  isFetching,        // true while any fetch in progress
  dataUpdatedAt,     // Timestamp of last update
  refetch,           // Manual refetch function
} = useQuery(options);
```

## Mutation States

```typescript
const { 
  mutate,            // Function to trigger mutation
  isPending,         // true while mutation in progress
  isError,           // true if mutation failed
  error,             // Error object if isError
  data,              // Last successful result
} = useMutation(options);
```

## Tips

- **Stale Time**: Data stays "fresh" without refetch
- **GC Time**: Data stays in cache after last use
- **Enabled**: Conditional queries (enable when data available)
- **QueryKey**: Must be deterministic array (use objects, not functions)
- **Retry**: Automatic retry on network errors (1 by default)
- **Deduplication**: Duplicate requests deduplicated automatically

## Examples in Codebase

- Query: `/src/hooks/use-lessons-query.ts`
- Service: `/src/services/lessons.service.ts`
- Component: `/src/features/content/editor/course-content-editor.tsx`

## Common Mistakes ❌

```typescript
// ❌ String-based queryKey (not reactive)
useQuery({ queryKey: 'users' })

// ✅ Array-based queryKey (reactive)
useQuery({ queryKey: ['users'] })

// ❌ Throwing errors without message
if (!res.data) throw new Error();

// ✅ Providing context
if (!res.data) throw new Error(res.error?.message || 'Failed to fetch');

// ❌ Not setting enabled
useQuery(options) // Runs even if data invalid

// ✅ Conditional query
useQuery({ ...options, enabled: !!id })

// ❌ Forgetting to invalidate after mutation
useMutation(mutationOptions) // Cache stale

// ✅ Invalidate in onSuccess
useMutation({
  ...options,
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ['users'] })
})
```

## When to Use

| Need | Use |
|------|-----|
| Fetch & cache data | `useQuery` + hook |
| Handle loading state | Query states |
| Sync server & client | Query invalidation |
| Auto-refetch stale | staleTime config |
| One-off API calls | `useMutation` |
| Manual cache control | `queryClient.set/removeQueries` |
| Background sync | staleTime + refetchInterval |

## Debug

```typescript
// Log all queries
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
<ReactQueryDevtools initialIsOpen={false} />

// Check query state
const queryCache = queryClient.getQueryData(['lessons', courseId]);

// Manually refetch
const { refetch } = useQuery(options);
refetch();
```

---

**See full guide**: `REACT_QUERY_GUIDE.md`
