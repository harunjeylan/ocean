# React Query Implementation Guide

## Overview

This document provides guidelines for using React Query (@tanstack/react-query v5) in the ahmedin-jable-elearning project. React Query is used for server state management, data fetching, caching, and synchronization.

## Architecture

### Core Setup

React Query is configured at the root level in `/src/providers/react-query.tsx`:

```tsx
// Default configuration for all queries and mutations
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,      // 5 minutes
      gcTime: 10 * 60 * 1000,        // 10 minutes (formerly cacheTime)
      retry: 1,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: 1,
    },
  },
});
```

The `ReactQueryProvider` wraps the entire application in `/src/routes/__root.tsx`.

## Service Pattern

Each service function has a corresponding **Options function** that defines React Query behavior.

### Naming Convention

```
Function:      getManyLessons()
Options:       getManyLessonsOptions()

Function:      getOneLesson()
Options:       getOneLessonOptions()

Function:      createLesson()
Options:       createLessonOptions()

Function:      updateLesson()
Options:       updateLessonOptions()

Function:      deleteLesson()
Options:       deleteLessonOptions()
```

### Example: Lessons Service

**File:** `/src/services/lessons.service.ts`

```typescript
import type { UseQueryOptions, UseMutationOptions } from '@tanstack/react-query';
import type {
  TCreateLesson,
  TLesson,
  TLessonDetail,
  TLessonQueryFilter,
  TUpdateLesson,
  TUpdateLessonOrder,
} from '@repo/common';
import { api } from '@/lib/api';

// === QUERY FUNCTIONS ===

export async function getManyLessons(params: TLessonQueryFilter) {
  const res = await api.get<TLesson[] | TLessonDetail[]>('/lessons', { params });
  return res;
}

export function getManyLessonsOptions(
  params: TLessonQueryFilter
): UseQueryOptions<TLesson[] | TLessonDetail[], Error> {
  return {
    queryKey: ['lessons', params],
    queryFn: async () => {
      const res = await getManyLessons(params);
      if (!res.data) {
        throw new Error(res.error?.message || 'Failed to fetch lessons');
      }
      return res.data;
    },
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  };
}

export async function getOneLesson(id: string, params: TLessonQueryFilter) {
  const res = await api.get<TLessonDetail>(`/lessons/${id}`, { params });
  return res;
}

export function getOneLessonOptions(
  id: string,
  params: TLessonQueryFilter
): UseQueryOptions<TLessonDetail, Error> {
  return {
    queryKey: ['lessons', id, params],
    queryFn: async () => {
      const res = await getOneLesson(id, params);
      if (!res.data) {
        throw new Error(res.error?.message || 'Failed to fetch lesson');
      }
      return res.data;
    },
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  };
}

// === MUTATION FUNCTIONS ===

export async function createLesson(data: TCreateLesson, params: TLessonQueryFilter) {
  const res = await api.post<TLesson>('/lessons', data, { params });
  return res;
}

export function createLessonOptions(): UseMutationOptions<
  TLesson,
  Error,
  { data: TCreateLesson; params: TLessonQueryFilter }
> {
  return {
    mutationFn: async ({ data, params }) => {
      const res = await createLesson(data, params);
      if (!res.data) {
        throw new Error(res.error?.message || 'Failed to create lesson');
      }
      return res.data;
    },
  };
}

export async function updateLesson(id: string, data: TUpdateLesson, params: TLessonQueryFilter) {
  const res = await api.put<TLesson>(`/lessons/${id}`, data, { params });
  return res;
}

export function updateLessonOptions(): UseMutationOptions<
  TLesson,
  Error,
  { id: string; data: TUpdateLesson; params: TLessonQueryFilter }
> {
  return {
    mutationFn: async ({ id, data, params }) => {
      const res = await updateLesson(id, data, params);
      if (!res.data) {
        throw new Error(res.error?.message || 'Failed to update lesson');
      }
      return res.data;
    },
  };
}

export async function deleteLesson(id: string, params: TLessonQueryFilter) {
  const res = await api.delete<boolean>(`/lessons/${id}`, { params });
  return res;
}

export function deleteLessonOptions(): UseMutationOptions<
  boolean,
  Error,
  { id: string; params: TLessonQueryFilter }
> {
  return {
    mutationFn: async ({ id, params }) => {
      const res = await deleteLesson(id, params);
      if (!res.data) {
        throw new Error(res.error?.message || 'Failed to delete lesson');
      }
      return res.data;
    },
  };
}
```

## Using React Query

### 1. Custom Query Hooks

Create a custom hook in `/src/hooks/use-lessons-query.ts`:

```typescript
import type { TLessonBasic } from '@repo/common';
import { useQuery } from '@tanstack/react-query';
import { getManyLessonsOptions } from '@/services/lessons.service';

interface UseLessonsQueryOptions {
  courseId: string;
  enabled?: boolean;
}

export function useLessonsQuery({ courseId, enabled = true }: UseLessonsQueryOptions) {
  return useQuery({
    ...getManyLessonsOptions({ courseId }),
    enabled: enabled && !!courseId,
  });
}
```

### 2. Using Query Hook in Components

```typescript
import { useLessonsQuery } from '@/hooks/use-lessons-query';

function CourseContentEditor({ course }: { course: TCourseDetail }) {
  // Fetch lessons with automatic caching and background refetch
  const { data: lessons = [], isLoading, error } = useLessonsQuery({
    courseId: course._id,
    enabled: !!course._id,
  });

  if (isLoading) return <div>Loading lessons...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      {lessons.map((lesson) => (
        <div key={lesson._id}>{lesson.name}</div>
      ))}
    </div>
  );
}
```

### 3. Using Mutations

```typescript
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateLessonOptions } from '@/services/lessons.service';

function UpdateLessonForm() {
  const queryClient = useQueryClient();

  const { mutate: updateLesson, isPending } = useMutation({
    ...updateLessonOptions(),
    onSuccess: (updatedLesson) => {
      // Invalidate related queries to trigger refetch
      queryClient.invalidateQueries({ queryKey: ['lessons'] });

      // Or update cache directly
      queryClient.setQueryData(['lessons', updatedLesson._id], updatedLesson);

      toast.success('Lesson updated successfully');
    },
    onError: (error) => {
      toast.error(error.message);
    },
  });

  return (
    <button
      onClick={() =>
        updateLesson({
          id: lessonId,
          data: { name: 'Updated Name' },
          params: { courseId },
        })
      }
      disabled={isPending}
    >
      {isPending ? 'Saving...' : 'Save'}
    </button>
  );
}
```

## Best Practices

### 1. Query Key Design

Use hierarchical, array-based query keys:

```typescript
// ✅ Good: Hierarchical and specific
queryKey: ['lessons', { courseId: '123', limit: 50 }]
queryKey: ['lessons', lessonId, { detail: true }]

// ❌ Bad: String-based, not hierarchical
queryKey: 'lessons-123'
queryKey: 'lesson-detail'
```

### 2. Lazy Loading for Large Datasets

Don't fetch everything in the loader. Use React Query for client-side data:

```typescript
// In route loader: Fast initial load
loader: async ({ params }) => {
  const { data: course } = await getOneCourse(params.courseId);
  const { data: sections } = await getManySections({ courseId: params.courseId });
  return { course, sections };
};

// In component: Lazy load lessons
function CourseEditor({ course, sections }) {
  const { data: lessons } = useLessonsQuery({ courseId: course._id });
  // Page loads fast, lessons load in background
}
```

### 3. Pagination

```typescript
interface UsePaginatedLessonsOptions {
  courseId: string;
  page: number;
  limit: number;
}

export function usePaginatedLessonsQuery({ courseId, page, limit }: UsePaginatedLessonsOptions) {
  return useQuery({
    ...getManyLessonsOptions({ courseId, page, limit }),
    enabled: !!courseId && page > 0 && limit > 0,
  });
}
```

### 4. Dependent Queries

```typescript
export function useLessonWithDetails(lessonId?: string) {
  return useQuery({
    ...getOneLessonOptions(lessonId || '', { courseId: '' }),
    enabled: !!lessonId, // Only fetch when lessonId is available
  });
}
```

### 5. Invalidation Strategy

```typescript
const { mutate: createLesson } = useMutation({
  ...createLessonOptions(),
  onSuccess: (newLesson) => {
    // Invalidate all lessons queries
    queryClient.invalidateQueries({ queryKey: ['lessons'] });

    // Or be specific
    queryClient.invalidateQueries({
      queryKey: ['lessons', { courseId: newLesson.courseId }],
    });
  },
});
```

### 6. Error Handling

```typescript
const { data, error, isError } = useQuery({
  ...getManyLessonsOptions({ courseId }),
});

if (isError) {
  return <ErrorBoundary error={error} />;
}

// Always check error type
if (error instanceof Error) {
  console.error('Query failed:', error.message);
}
```

## Performance Tips

### 1. Stale Time vs GC Time

```typescript
// Default (from global config)
staleTime: 5 * 60 * 1000,   // Data is fresh for 5 minutes
gcTime: 10 * 60 * 1000,     // Keep in cache for 10 minutes after last use

// Override for frequently accessed data
queryOptions: {
  staleTime: 30 * 60 * 1000, // 30 minutes
  gcTime: 60 * 60 * 1000,    // 1 hour
}

// Override for rarely accessed data
queryOptions: {
  staleTime: 30 * 1000,      // 30 seconds
  gcTime: 2 * 60 * 1000,     // 2 minutes
}
```

### 2. Disable Refetch on Window Focus

```typescript
// Already disabled globally, but can enable per query
useQuery({
  ...queryOptions,
  refetchOnWindowFocus: true, // Override global
})
```

### 3. Prefetching

```typescript
const queryClient = useQueryClient();

// Prefetch data on hover
<button
  onMouseEnter={() => {
    queryClient.prefetchQuery({
      ...getLessonOptions(lessonId, params),
    });
  }}
>
  View Lesson
</button>
```

### 4. Parallel Queries

```typescript
// Instead of sequential Promise.all, use multiple hooks
function CourseDashboard({ courseId }) {
  const lessonsQuery = useLessonsQuery({ courseId });
  const sectionsQuery = useSectionsQuery({ courseId });

  // Both queries run in parallel, cached independently
  const isLoading = lessonsQuery.isLoading || sectionsQuery.isLoading;
}
```

## Migration Guide

### From Server-Side to Client-Side

**Before:**
```tsx
export const Route = createFileRoute('/course/$courseId')({
  loader: async ({ params }) => {
    const course = await getOneCourse(params.courseId);
    const lessons = await getManyLessons({ courseId: params.courseId });
    const sections = await getManySections({ courseId: params.courseId });
    
    return { course, lessons, sections }; // Large payload
  },
  component: CoursePage,
});

function CoursePage() {
  const { course, lessons, sections } = Route.useLoaderData();
  // Page blocked until all data loaded
}
```

**After:**
```tsx
export const Route = createFileRoute('/course/$courseId')({
  loader: async ({ params }) => {
    // Only fetch essential data
    const course = await getOneCourse(params.courseId);
    const sections = await getManySections({ courseId: params.courseId });
    
    return { course, sections }; // Small, fast payload
  },
  component: CoursePage,
});

function CoursePage() {
  const { course, sections } = Route.useLoaderData();
  
  // Load lessons client-side with React Query
  const { data: lessons = [] } = useLessonsQuery({ courseId: course._id });
  
  // Page shows structure immediately, lessons load in background
}
```

## Troubleshooting

### Issue: Queries Not Refetching

```typescript
// Check if query is stale
const { dataUpdatedAt } = useQuery(queryOptions);

// Manually refetch
const { refetch } = useQuery(queryOptions);
refetch();

// Or invalidate
queryClient.invalidateQueries({ queryKey: ['lessons'] });
```

### Issue: Duplicate Requests

```typescript
// React 18 StrictMode causes double renders in dev
// Queries deduplicate automatically - this is expected behavior

// In production, you'll see single request per query
```

### Issue: Cached Data Stale

```typescript
// Increase staleTime
getManyLessonsOptions({ courseId }): {
  ...defaults,
  staleTime: 30 * 60 * 1000, // 30 minutes instead of 5
}

// Or disable caching for sensitive data
gcTime: 0, // Immediately discard
```

## DevTools

Enable React Query DevTools (in development):

```bash
npm install @tanstack/react-query-devtools --save-dev
```

Add to root layout:

```tsx
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';

function RootDocument() {
  return (
    <>
      {/* ... app content ... */}
      <ReactQueryDevtools initialIsOpen={false} />
    </>
  );
}
```

## Testing

```typescript
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render } from '@testing-library/react';

// Create a fresh QueryClient for each test
const createTestQueryClient = () => new QueryClient();

function renderWithQueryClient(component: React.ReactElement) {
  const queryClient = createTestQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      {component}
    </QueryClientProvider>
  );
}

// In tests:
test('displays lessons', async () => {
  const { getByText } = renderWithQueryClient(
    <CourseEditor courseId="123" />
  );
  
  expect(await getByText('Loading...')).toBeInTheDocument();
  expect(await getByText('Lesson 1')).toBeInTheDocument();
});
```

## Resources

- [React Query Documentation](https://tanstack.com/query/latest)
- [Query Key Factory Pattern](https://tkdodo.eu/blog/effective-react-query-keys)
- [React Query Best Practices](https://tkdodo.eu/blog/react-query-as-a-state-manager)
- [Caching Strategies](https://tkdodo.eu/blog/the-important-parts-of-useless-code)

## Summary

1. **Service Pattern**: Every service function has an options function
2. **Custom Hooks**: Create hooks for complex queries
3. **Lazy Loading**: Load large datasets client-side with React Query
4. **Mutations**: Use mutation hooks with proper invalidation
5. **Caching**: Leverage automatic caching and deduplication
6. **Performance**: Control stale time and garbage collection

This approach ensures:
- ✅ Fast page loads
- ✅ Automatic data synchronization
- ✅ Built-in error handling
- ✅ Efficient caching
- ✅ Developer experience with DevTools
