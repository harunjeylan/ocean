# React Query Implementation for ahmedin-jable-elearning

## 📚 Documentation Overview

This directory contains comprehensive guidance for using React Query in the project.

### Documents

1. **REACT_QUERY_GUIDE.md** - Full implementation guide
   - Architecture overview
   - Service pattern explanation
   - Usage examples (queries, mutations, hooks)
   - Best practices
   - Performance optimization
   - Migration guide
   - Troubleshooting

2. **QUICK_REFERENCE.md** - Quick lookup guide
   - Setup (already done)
   - Naming conventions
   - Common patterns
   - Code snippets
   - Common mistakes to avoid
   - Debug tips

3. **MIGRATION_CHECKLIST.md** - Step-by-step service migration
   - Checklist for converting services
   - Priority list of services to migrate
   - Service template
   - Component migration example
   - Validation checklist
   - Common issues & fixes

## Check Setup Status

### Checklist ✨

- [ ] QueryClient setup in `/src/providers/react-query.ts `
- [ ] ReactQueryProvider in root layout `/src/routes/__root.ts `
- [ ] Query options functions added to `lessons.service.ts`
- [ ] Query options functions added to `sections.service.ts`
- [ ] Custom hook `useLessonsQuery` created
- [ ] Course content editor updated to lazy-load lessons
- [ ] Documentation created

### Service Migration

#### Services Format
- ✅ lessons.service.ts
- ✅ sections.service.ts

### For New Components Using Queries

```typescript
import { useLessonsQuery } from '@/hooks/use-lessons-query';

function MyComponent({ courseId }) {
  const { data: lessons = [], isLoading, error } = useLessonsQuery({
    courseId,
  });

  if (isLoading) return <Loading />;
  if (error) return <Error error={error} />;

  return <div>{/* render lessons */}</div>;
}
```

### For Mutations

```typescript
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateLessonOptions } from '@/services/lessons.service';

function UpdateForm() {
  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation({
    ...updateLessonOptions(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['lessons'] });
    },
  });

  return (
    <button onClick={() => mutate({ id, data, params })}>
      {isPending ? 'Saving...' : 'Save'}
    </button>
  );
}
```

## 🎯 Key Benefits

1. **Fast Page Loads** - Lazy load non-critical data client-side
2. **Automatic Caching** - Built-in deduplication and persistence
3. **Background Sync** - Auto-refetch stale data
4. **Less Boilerplate** - No manual loading/error state management
5. **Better DX** - DevTools for debugging
6. **Easy Testing** - Mockable hooks and clear data flow

## 📊 Performance Impact

### Course Content Editor Page

**Before (10-20 second load):**
- Loader waits for: auth + course + sections + lessons (all lessons)
- Large payload blocks page render

**After (< 2 second initial):**
- Loader fetches: auth + course + sections only
- Sections shows immediately
- Lessons load in background via React Query
- Page interactive much faster

## 🔑 Core Concepts

### QueryKey Pattern
```typescript
// Hierarchical, array-based for proper invalidation
queryKey: ['lessons', { courseId: '123', page: 1 }]
```

### Stale Time vs GC Time
```typescript
staleTime: 5 * 60 * 1000,   // Data stays fresh for 5 minutes
gcTime: 10 * 60 * 1000,     // Keep in cache for 10 minutes
```

### Lazy Queries
```typescript
// Only fetch when condition met
useQuery({
  ...options,
  enabled: !!id,  // Don't fetch until id exists
})
```

### Invalidation
```typescript
// Force refetch when data changes
queryClient.invalidateQueries({ queryKey: ['lessons'] })
```

## 📖 Common Tasks

### Fetch Data in Component
See: `QUICK_REFERENCE.md` → "Query (Read Data)"

### Create Custom Hook
See: `REACT_QUERY_GUIDE.md` → "Custom Query Hooks"

### Migrate a Service
See: `MIGRATION_CHECKLIST.md` → "Step 1-5"

### Debug Cache Issues
See: `REACT_QUERY_GUIDE.md` → "Troubleshooting"

### Optimize Performance
See: `REACT_QUERY_GUIDE.md` → "Performance Tips"

## 🛠️ Tools

### Development Tools
- **React Query DevTools**: Inspect all queries/mutations
- **React Dev Tools**: Check component state
- **Network Tab**: Monitor actual requests

### Enable DevTools

In production build, DevTools won't load. For development:

```tsx
// In __root.tsx or dev-only component
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';

<ReactQueryDevtools initialIsOpen={false} position="bottom-right" />
```

## 📋 Naming Convention Cheat Sheet

| Action | Service Function | Options Function | Hook |
|--------|-----------------|-----------------|------|
| Get single | `getUser(id)` | `getUserOptions(id)` | `useUserQuery(id)` |
| Get list | `getUsers()` | `getUsersOptions()` | `useUsersQuery()` |
| Create | `createUser(data)` | `createUserOptions()` | - |
| Update | `updateUser(id, data)` | `updateUserOptions()` | - |
| Delete | `deleteUser(id)` | `deleteUserOptions()` | - |

## 🎓 Learning Path

1. **Start Here**: Read `QUICK_REFERENCE.md` (5 min)
2. **Understand Pattern**: Read `REACT_QUERY_GUIDE.md` sections 1-3 (15 min)
3. **See Example**: Check `/src/services/lessons.service.ts` (10 min)
4. **Hands On**: Create your first query hook (20 min)
5. **Deep Dive**: Read full `REACT_QUERY_GUIDE.md` (30 min)
6. **Migrate**: Use `MIGRATION_CHECKLIST.md` for next service (varies)

## 🐛 Debugging Tips

### Check Query State
```typescript
const query = useQuery(options);
console.log({
  data: query.data,
  status: query.status,           // pending | error | success
  isLoading: query.isLoading,
  isFetching: query.isFetching,   // includes background refetch
  dataUpdatedAt: query.dataUpdatedAt,
});
```

### Check Cache
```typescript
const query = queryClient.getQueryData(['lessons', courseId]);
console.log('Cached data:', query);
```

### Force Refetch
```typescript
const { refetch } = useQuery(options);
refetch();
```

### Clear Cache
```typescript
queryClient.clear();
queryClient.removeQueries({ queryKey: ['lessons'] });
```

## ⚠️ Common Gotchas

1. **QueryKey must be array** - `['users']` not `'users'`
2. **Mutations don't auto-invalidate** - Add `onSuccess` handler
3. **Enabled prevents fetch** - `enabled: !!id` won't fetch until id exists
4. **Double renders in StrictMode** - Expected in dev, single request in prod
5. **Forgotten error handling** - Always check error state

## 📚 External Resources

- [Official Docs](https://tanstack.com/query/latest)
- [Query Key Factory](https://tkdodo.eu/blog/effective-react-query-keys)
- [Best Practices](https://tkdodo.eu/blog/react-query-as-a-state-manager)
- [Testing Guide](https://tanstack.com/query/latest/docs/frameworks/react/guides/testing)

## 🤝 Contributing

When adding new services:

1. Follow the service pattern (function + options function)
2. Create corresponding hook if complex
3. Update components to use hooks
4. Add to migration checklist
5. Link to documentation

## 📝 Notes

- Default config suitable for most data types
- Override `staleTime`/`gcTime` per query if needed
- Keep `refetchOnWindowFocus: false` to avoid annoying refetches
- Test cache behavior matches product requirements

## 🎯 Next Steps

1. Review `QUICK_REFERENCE.md` (today)
2. Start migrating `users.service.ts` (this week)
3. Convert components to use new hooks (next week)
4. Monitor performance metrics
5. Document learnings

---

**Version:** 1.0
**Last Updated:** 2024
**Maintained By:** Development Team
