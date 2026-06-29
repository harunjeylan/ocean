# React Query Migration Checklist

## For Service Owners: Converting Services to React Query

Use this checklist to convert any service to follow the React Query pattern.

### Step 1: Add Query Options Functions

**In `/src/services/your-service.service.ts`:**

- [ ] Import types: `UseQueryOptions`, `UseMutationOptions` from `@tanstack/react-query`
- [ ] For each query function (`get*`), add an options function below it
- [ ] For each mutation function (`create*`, `update*`, `delete*`), add an options function
- [ ] Options function returns proper type:
  - [ ] Queries: `UseQueryOptions<DataType, Error>`
  - [ ] Mutations: `UseMutationOptions<DataType, Error, PayloadType>`
- [ ] Include `queryKey` or `mutationFn` in options
- [ ] Set `staleTime` and `gcTime` for queries
- [ ] Error handling: throw with message

### Step 2: Create Custom Hooks (if needed)

**In `/src/hooks/use-your-service-query.ts`:**

- [ ] Import service options function
- [ ] Create hook with `useQuery` or `useMutation`
- [ ] Spread service options: `...getYourDataOptions(...)`
- [ ] Add conditional logic (`enabled`, `onSuccess`, etc.)
- [ ] Export hook with clear name

### Step 3: Update Components

**In your React components:**

- [ ] Remove direct service function calls from `useEffect`
- [ ] Replace with custom hook or `useQuery`/`useMutation`
- [ ] Remove manual loading state (use hook state)
- [ ] Remove manual error state (use hook state)
- [ ] Add proper error handling
- [ ] Add loading indicators

### Step 4: Update Route Loaders

**In `/src/routes/your-route.tsx`:**

- [ ] Remove queries that can be lazy-loaded
- [ ] Keep only essential queries in loader
- [ ] Comments: "X will be fetched client-side using React Query"
- [ ] Pass only critical data to component

### Step 5: Testing

- [ ] Test loading state
- [ ] Test success state
- [ ] Test error state
- [ ] Test cache behavior (data persists between routes)
- [ ] Test invalidation/refetch behavior

## Priority Services to Migrate

These services have high usage and would benefit most:

### High Priority 🔴
- [ ] `users.service.ts` - Many components use user queries
- [ ] `courses.service.ts` - Course editor needs fast initial load
- [ ] `batches.service.ts` - Batch management page
- [ ] `classes.service.ts` - Classes listing

### Medium Priority 🟡
- [ ] `assignments.service.ts`
- [ ] `exams.service.ts`
- [ ] `materials.service.ts`
- [ ] `announcements.service.ts`

### Low Priority 🟢
- [ ] One-off services with minimal usage
- [ ] Services with single POST endpoints
- [ ] Legacy services

## Service Template

Copy this template for new services:

```typescript
// File: /src/services/example.service.ts

import type { UseQueryOptions, UseMutationOptions } from '@tanstack/react-query';
import type { TExample, TExampleFilter, TCreateExample, TUpdateExample } from '@repo/common';
import { api } from '@/lib/api';

// ===== QUERIES =====

export async function getExample(id: string) {
  const res = await api.get<TExample>(`/example/${id}`);
  return res;
}

export function getExampleOptions(id: string): UseQueryOptions<TExample, Error> {
  return {
    queryKey: ['example', id],
    queryFn: async () => {
      const res = await getExample(id);
      if (!res.data) throw new Error(res.error?.message || 'Failed to fetch example');
      return res.data;
    },
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  };
}

export async function getManyExamples(filters?: TExampleFilter) {
  const res = await api.get<TExample[]>('/example', { params: filters });
  return res;
}

export function getManyExamplesOptions(filters?: TExampleFilter): UseQueryOptions<TExample[], Error> {
  return {
    queryKey: ['examples', filters],
    queryFn: async () => {
      const res = await getManyExamples(filters);
      if (!res.data) throw new Error(res.error?.message || 'Failed to fetch examples');
      return res.data;
    },
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  };
}

// ===== MUTATIONS =====

export async function createExample(data: TCreateExample) {
  const res = await api.post<TExample>('/example', data);
  return res;
}

export function createExampleOptions(): UseMutationOptions<TExample, Error, TCreateExample> {
  return {
    mutationFn: async (data) => {
      const res = await createExample(data);
      if (!res.data) throw new Error(res.error?.message || 'Failed to create example');
      return res.data;
    },
  };
}

export async function updateExample(id: string, data: TUpdateExample) {
  const res = await api.put<TExample>(`/example/${id}`, data);
  return res;
}

export function updateExampleOptions(): UseMutationOptions<
  TExample,
  Error,
  { id: string; data: TUpdateExample }
> {
  return {
    mutationFn: async ({ id, data }) => {
      const res = await updateExample(id, data);
      if (!res.data) throw new Error(res.error?.message || 'Failed to update example');
      return res.data;
    },
  };
}

export async function deleteExample(id: string) {
  const res = await api.delete<boolean>(`/example/${id}`);
  return res;
}

export function deleteExampleOptions(): UseMutationOptions<boolean, Error, string> {
  return {
    mutationFn: async (id) => {
      const res = await deleteExample(id);
      if (!res.data) throw new Error(res.error?.message || 'Failed to delete example');
      return res.data;
    },
  };
}
```

## Hook Template

```typescript
// File: /src/hooks/use-example-query.ts

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { 
  getExampleOptions,
  createExampleOptions,
  updateExampleOptions,
  deleteExampleOptions,
} from '@/services/example.service';

// Query Hook
export function useExampleQuery(id?: string) {
  return useQuery({
    ...getExampleOptions(id || ''),
    enabled: !!id,
  });
}

// Mutation Hooks
export function useCreateExampleMutation() {
  const queryClient = useQueryClient();
  
  return useMutation({
    ...createExampleOptions(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['examples'] });
    },
  });
}

export function useUpdateExampleMutation() {
  const queryClient = useQueryClient();
  
  return useMutation({
    ...updateExampleOptions(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['examples'] });
    },
  });
}

export function useDeleteExampleMutation() {
  const queryClient = useQueryClient();
  
  return useMutation({
    ...deleteExampleOptions(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['examples'] });
    },
  });
}
```

## Component Migration Example

### Before (Without React Query)

```typescript
function ExampleComponent() {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  useEffect(() => {
    const fetch = async () => {
      setLoading(true);
      try {
        const res = await getExample('123');
        setData(res.data);
      } catch (err) {
        setError(err);
      } finally {
        setLoading(false);
      }
    };
    fetch();
  }, []);

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return <div>{data?.name}</div>;
}
```

### After (With React Query)

```typescript
function ExampleComponent() {
  const { data, isLoading, error } = useExampleQuery('123');

  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return <div>{data?.name}</div>;
}
```

**Benefits:**
- ✅ 70% less boilerplate code
- ✅ Automatic caching & background refetch
- ✅ Built-in deduplication
- ✅ Better error handling
- ✅ Easier to test

## Validation Checklist

After migrating a service, verify:

- [ ] All query functions have options functions
- [ ] All mutation functions have options functions
- [ ] Options functions throw errors with messages
- [ ] QueryKey format is array-based: `['resource', id]`
- [ ] Hooks have conditional logic for dependent queries
- [ ] Route loaders don't fetch unnecessary data
- [ ] Components use hooks instead of direct API calls
- [ ] Error boundaries/handling in place
- [ ] DevTools can see all queries
- [ ] Cache behavior matches requirements
- [ ] No duplicate requests in DevTools
- [ ] Stale/GC times appropriate for data type

## Common Issues & Fixes

### Issue: "Too many network requests"
**Cause:** Multiple hooks calling same query function
**Fix:** Check queryKey matches exactly
```typescript
// These should be identical
queryKey: ['example', id]
queryKey: ['example', id]  // ✅ Same

queryKey: ['example', { id }]
queryKey: ['example', id]   // ❌ Different
```

### Issue: "Data not updating after mutation"
**Cause:** Forgot to invalidate
**Fix:** Invalidate in `onSuccess`
```typescript
useMutation({
  ...mutationOptions,
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['examples'] });
  },
})
```

### Issue: "Hook runs on mount even with enabled: false"
**Cause:** enabled prop changed after mount
**Fix:** Check enabled value is stable
```typescript
// ❌ Bad: enabled recalculates
enabled: id ? true : false

// ✅ Good: clear boolean
enabled: !!id
```

## Success Metrics

After migration, track:

- ⏱️ Page load time (should decrease)
- 📊 Number of network requests (should stay same or decrease)
- 🔄 Re-render count (should decrease due to batching)
- 💾 Memory usage (should improve with cleanup)
- ✅ Error tracking (easier to debug)
- 😊 Developer experience (less code to write)

## Support

- Review full guide: `REACT_QUERY_GUIDE.md`
- Quick reference: `QUICK_REFERENCE.md`
- Example implementations: `/src/services/lessons.service.ts`
- React Query docs: https://tanstack.com/query/latest

---

**Last Updated:** 2024
**Status:** Implementation in progress
**Next Service to Migrate:** TBD
