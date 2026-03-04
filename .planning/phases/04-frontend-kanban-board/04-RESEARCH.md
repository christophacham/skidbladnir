# Phase 4: Frontend Kanban Board - Research

**Researched:** 2026-03-04
**Domain:** SvelteKit 5 SPA frontend, REST API integration, kanban board UI
**Confidence:** HIGH

## Summary

Phase 4 creates the first SvelteKit frontend for AGTX -- a single-page application that connects to the existing agtxd daemon REST API and presents a 5-column kanban board. The backend API surface is fully implemented (tasks CRUD, projects listing, health check) and returns direct JSON responses with no envelope. The frontend is a new `web/` directory with SvelteKit 5 configured as an SPA via adapter-static.

Svelte 5 introduces "runes" ($state, $derived, $effect) as the reactive primitive system, replacing Svelte 4's compiler-based reactivity. For shared state across components, the pattern is `.svelte.ts` modules exporting reactive objects or class instances. The SPA connects to `localhost:3742` (default daemon port) with Vite's dev proxy eliminating CORS issues during development. For production, the daemon needs a CORS layer via tower-http (currently only has `trace` and `timeout` features enabled).

The phase scope is deliberately limited: board layout, task cards, CRUD modals, text search with dimming, project sidebar, and command palette. No live output streaming (Phase 5), no workflow side effects (Phase 6), no drag-and-drop (out of scope per REQUIREMENTS.md).

**Primary recommendation:** Use `npx sv create web --types ts --no-add-ons` to scaffold, add Tailwind CSS via `sv add tailwind`, configure adapter-static with fallback for SPA mode, and use Vite server proxy to forward `/api` and `/health` requests to `localhost:3742` during development. Build all state management with Svelte 5 runes in `.svelte.ts` modules.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- 5-column layout with equal-width columns by default
- Columns are collapsible -- user can collapse Done/Backlog to icons to give more room to active work
- Collapsed state persisted in localStorage
- Minimal cards: title + agent badge + phase status colored dot
- Description and details shown in the detail panel (Phase 5), not on cards
- Agent badge: small colored pill with agent name (e.g., "claude", "codex")
- Phase status: colored dot -- green pulsing (Working), yellow (Idle), checkmark (Ready), gray (Exited)
- Modal dialog triggered from multiple entry points: 'o' keyboard shortcut, '+' button in Backlog column header, and command palette
- Modal fields: title (required), agent dropdown (required, pre-filled with project default), description textarea (optional)
- All three fields shown upfront
- Confirmation dialog: press 'x' or click delete -> modal asks "Delete 'task-name'?" with Cancel/Delete buttons
- Project sidebar: Hidden by default, toggled with 'e' key, ~200px on the left when open, flat list of projects with task count per project
- Command palette: Ctrl+K opens fuzzy search over actions, central action hub
- Top navigation bar: Project name on the left, search bar (always visible) in the center, '+' create button on the right
- Search: Always-visible search bar, '/' to focus, Escape to clear, live filtering as you type across title + description
- Non-matching cards dimmed/semi-transparent (not hidden) to maintain spatial context
- Text search only for Phase 4 -- agent/status filter chips deferred to Phase 10
- Client-side filtering -- all tasks fetched on load, filtered in browser

### Claude's Discretion
- SvelteKit project structure and component organization
- CSS approach (Tailwind vs scoped CSS vs CSS modules)
- State management pattern (stores, context, etc.)
- HTTP client for REST API calls
- Animation/transition details for collapsible columns
- Exact color values for dark theme (must use CSS custom properties per PROJECT.md)
- Responsive breakpoint handling within 1280px-2560px+ range

### Deferred Ideas (OUT OF SCOPE)
- Agent/status filter chips alongside search -- Phase 10 (UX Polish)
- Drag-and-drop task reordering -- explicitly out of scope per REQUIREMENTS.md
- Light theme option -- v2 (THEME-01)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BOARD-01 | User sees 5-column kanban layout (Backlog/Planning/Running/Review/Done) | SvelteKit SPA with CSS grid layout; TaskStatus enum maps directly to columns; GET /api/v1/tasks provides all task data |
| BOARD-02 | Task cards display title, agent badge, and phase status indicator | Task model includes title, agent, status fields; PhaseStatus enum (Working/Idle/Ready/Exited) for status dots |
| BOARD-03 | User can create tasks with title and description | POST /api/v1/tasks accepts {title, agent, project_id, description?}; modal dialog pattern with Svelte 5 |
| BOARD-04 | User can delete tasks with confirmation dialog | DELETE /api/v1/tasks/{id} returns 204; confirmation modal before API call |
| BOARD-05 | User can search/filter tasks across title and description | Client-side filtering with $derived rune; fuse.js for fuzzy matching in command palette |
| BOARD-06 | User can switch between projects via multi-project sidebar | GET /api/v1/projects returns project list; sidebar component with localStorage persistence |
| BOARD-07 | User can invoke command palette (Ctrl+K) for fuzzy action search | Custom component with fuse.js; registers actions for create, search, switch project, toggle sidebar |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| svelte | 5.53.x | UI framework | Locked decision from PROJECT.md; runes provide fine-grained reactivity |
| @sveltejs/kit | 2.53.x | App framework | SPA mode via adapter-static; file-based routing; $lib alias |
| @sveltejs/adapter-static | latest | Build adapter | Produces static SPA with fallback page for deployment |
| typescript | 5.x | Type system | Type safety for API contracts, component props |
| vite | 6.x | Build tool | Ships with SvelteKit; dev server with proxy |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tailwindcss | 4.x | Utility CSS | All styling -- rapid dark theme, responsive layout, consistent spacing |
| fuse.js | 7.x | Fuzzy search | Command palette action search; ~5KB gzipped, zero deps |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Tailwind CSS | Svelte scoped CSS | Scoped CSS is zero-dep but slower for consistent dark theme + responsive; Tailwind integrates via `sv add` |
| fuse.js | microfuzz (2KB) | Smaller but less battle-tested; fuse.js has better Svelte bindings and typo tolerance |
| Native fetch | axios | fetch is built-in, sufficient for REST JSON; no need for axios's extras |

**Installation:**
```bash
# Create project
npx sv create web --types ts --no-add-ons --no-install

# Add Tailwind CSS
cd web && npx sv add tailwind

# Install dependencies
npm install

# Add runtime deps
npm install fuse.js
```

## Architecture Patterns

### Recommended Project Structure
```
web/
├── src/
│   ├── lib/
│   │   ├── api/
│   │   │   ├── client.ts          # fetch wrapper, base URL, error handling
│   │   │   ├── tasks.ts           # Task CRUD API functions
│   │   │   └── projects.ts        # Project list API functions
│   │   ├── stores/
│   │   │   ├── tasks.svelte.ts    # Task state: list, selected, search filter
│   │   │   ├── projects.svelte.ts # Project state: list, active project
│   │   │   ├── ui.svelte.ts       # UI state: sidebar open, collapsed columns, modals
│   │   │   └── commands.svelte.ts # Command palette actions registry
│   │   ├── components/
│   │   │   ├── Board.svelte       # 5-column kanban grid layout
│   │   │   ├── Column.svelte      # Single column with header + task list
│   │   │   ├── TaskCard.svelte    # Minimal card: title + agent badge + status dot
│   │   │   ├── CreateTaskModal.svelte  # Task creation modal dialog
│   │   │   ├── DeleteConfirmModal.svelte # Delete confirmation dialog
│   │   │   ├── CommandPalette.svelte    # Ctrl+K fuzzy action search
│   │   │   ├── Sidebar.svelte     # Project list sidebar
│   │   │   ├── NavBar.svelte      # Top bar: project name, search, create button
│   │   │   └── SearchBar.svelte   # Always-visible search with '/' focus
│   │   ├── types/
│   │   │   └── index.ts           # TypeScript interfaces matching API models
│   │   └── utils/
│   │       └── keyboard.ts        # Keyboard shortcut registration
│   ├── routes/
│   │   ├── +layout.svelte         # Root layout: sidebar + nav + board slot
│   │   ├── +layout.ts             # export const ssr = false (SPA mode)
│   │   └── +page.svelte           # Board page (single route for SPA)
│   ├── app.html                   # HTML shell
│   └── app.css                    # Tailwind directives + CSS custom properties
├── static/
│   └── favicon.ico
├── svelte.config.js               # adapter-static with fallback: '200.html'
├── vite.config.ts                 # Dev proxy to localhost:3742
├── tailwind.config.ts             # Dark theme, custom colors
├── tsconfig.json
└── package.json
```

### Pattern 1: Svelte 5 Runes for Shared State (.svelte.ts modules)
**What:** Reactive state in `.svelte.ts` files using $state, exported as objects/classes
**When to use:** All shared application state (tasks, projects, UI state)
**Example:**
```typescript
// src/lib/stores/tasks.svelte.ts
// Source: https://svelte.dev/docs/svelte/$state + https://mainmatter.com/blog/2025/03/11/global-state-in-svelte-5/

import type { Task } from '$lib/types';
import { fetchTasks, createTask as apiCreate, deleteTask as apiDelete } from '$lib/api/tasks';

class TaskStore {
  list = $state<Task[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);
  searchQuery = $state('');

  // Derived: tasks grouped by status column
  byStatus = $derived.by(() => {
    const grouped: Record<string, Task[]> = {
      backlog: [], planning: [], running: [], review: [], done: []
    };
    for (const task of this.list) {
      grouped[task.status]?.push(task);
    }
    return grouped;
  });

  // Derived: filtered tasks based on search query
  filtered = $derived.by(() => {
    if (!this.searchQuery.trim()) return this.list;
    const q = this.searchQuery.toLowerCase();
    return this.list.filter(t =>
      t.title.toLowerCase().includes(q) ||
      (t.description?.toLowerCase().includes(q) ?? false)
    );
  });

  // Derived: set of matching task IDs for dimming non-matches
  matchingIds = $derived(new Set(this.filtered.map(t => t.id)));

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.list = await fetchTasks();
    } catch (e) {
      this.error = e instanceof Error ? e.message : 'Failed to load tasks';
    } finally {
      this.loading = false;
    }
  }

  async create(title: string, agent: string, projectId: string, description?: string) {
    const task = await apiCreate({ title, agent, project_id: projectId, description });
    this.list = [...this.list, task];
    return task;
  }

  async delete(id: string) {
    await apiDelete(id);
    this.list = this.list.filter(t => t.id !== id);
  }
}

export const taskStore = new TaskStore();
```

### Pattern 2: API Client with fetch
**What:** Thin fetch wrapper with base URL, JSON parsing, and error normalization
**When to use:** All REST API communication
**Example:**
```typescript
// src/lib/api/client.ts

const BASE_URL = '/api/v1'; // Proxied to daemon in dev, same-origin in prod

class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

export async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    headers: { 'Content-Type': 'application/json', ...options?.headers },
    ...options,
  });

  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new ApiError(res.status, body.error || res.statusText);
  }

  // 204 No Content
  if (res.status === 204) return undefined as T;

  return res.json();
}
```

### Pattern 3: CSS Custom Properties for Theme
**What:** Dark theme colors as CSS variables, populated from daemon's ThemeConfig defaults
**When to use:** All color references in the app
**Example:**
```css
/* src/app.css */
@import 'tailwindcss';

:root {
  --color-selected: #ead49a;    /* Yellow - from ThemeConfig defaults */
  --color-normal: #5cfff7;      /* Cyan */
  --color-dimmed: #9C9991;      /* Dark Gray */
  --color-text: #f2ece6;        /* Light Rose */
  --color-accent: #5cfff7;      /* Cyan */
  --color-description: #C4B0AC; /* Dimmed Rose */
  --color-column-header: #a0d2fa; /* Light Blue Gray */
  --color-popup-border: #9ffcf8;  /* Light Cyan */
  --color-popup-header: #69fae7;  /* Light Cyan */
  --color-bg: #1a1a2e;         /* Dark background */
  --color-surface: #16213e;    /* Card/panel surface */
  --color-surface-hover: #1e2d4d; /* Hover state */
  --color-border: #2a2a4a;     /* Subtle borders */
}
```

### Pattern 4: SPA Configuration
**What:** SvelteKit configured as pure client-side SPA
**When to use:** The entire frontend -- no SSR needed since this is a dashboard connecting to a local daemon
**Example:**
```javascript
// svelte.config.js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: '200.html'
    })
  }
};
```

```typescript
// vite.config.ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': 'http://localhost:3742',
      '/health': 'http://localhost:3742'
    }
  }
});
```

```typescript
// src/routes/+layout.ts
export const ssr = false;
```

### Anti-Patterns to Avoid
- **Exporting $state variables directly:** `export let count = $state(0)` breaks reactivity across modules. Export objects or class instances instead.
- **Destructuring reactive proxies:** `const { title } = task` creates a static copy. Access `task.title` directly.
- **Using $effect for derived state:** Use `$derived` instead of `$effect(() => { doubled = count * 2 })`.
- **Wrapping API calls in $effect without cleanup:** Race conditions occur when dependencies change mid-fetch. Track request identity.
- **Storing all state in a single monolithic store:** Separate concerns (tasks, projects, UI) into distinct `.svelte.ts` modules.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fuzzy search | Custom string matching | fuse.js | Handles typo tolerance, weighted fields, and scoring |
| CSS utility system | Custom CSS class naming | Tailwind CSS | Dark theme, responsive, consistent spacing out of the box |
| SPA routing | Custom hash router | SvelteKit adapter-static | Framework-native SPA mode with build optimization |
| Dev proxy | Manual CORS handling | Vite server.proxy | Zero-config proxy to daemon, no CORS needed in dev |
| Keyboard event handling | Raw addEventListener | Svelte `on:keydown` + `<svelte:window>` | Component lifecycle handles cleanup automatically |

**Key insight:** This phase is a standard CRUD SPA connecting to an existing API. The primary complexity is in the layout (collapsible columns, dimming search, command palette) not in data management. Use framework primitives and avoid inventing patterns.

## Common Pitfalls

### Pitfall 1: CORS in Production
**What goes wrong:** Frontend served from different origin than API; browsers block requests.
**Why it happens:** Dev mode uses Vite proxy (same-origin), but production static files are served separately.
**How to avoid:** Either (a) serve static files from the daemon itself via tower-http's ServeDir, or (b) add CorsLayer to the daemon. Option (a) is simpler -- add `tower-http` feature `fs` and serve the `web/build/` directory. This also means no CORS configuration needed.
**Warning signs:** 403 or CORS errors in browser console.

### Pitfall 2: Svelte 5 Reactivity with Destructuring
**What goes wrong:** Destructuring a reactive $state proxy freezes the value at destructure time.
**Why it happens:** Svelte's reactivity tracks property access on the proxy object; destructured copies lose the proxy.
**How to avoid:** Always access properties through the reactive object: `task.title` not `const { title } = task`.
**Warning signs:** UI not updating when state changes.

### Pitfall 3: Search Dimming vs Hiding
**What goes wrong:** Implementing search as a filter that removes non-matching cards changes spatial layout, disorienting users.
**Why it happens:** Natural instinct is `{#if matchesSearch}` but decision says dimming, not hiding.
**How to avoid:** Render ALL cards always. Apply CSS `opacity: 0.3` to non-matching cards via a `matchingIds` derived set.
**Warning signs:** Cards jumping around when typing in search.

### Pitfall 4: Collapsed Column State
**What goes wrong:** Collapsed columns lose task count or become inaccessible.
**Why it happens:** Collapsing to icons requires maintaining the data even when not rendering task cards.
**How to avoid:** Collapsed columns show an icon + task count badge. The data is always present; only the rendering changes. Store collapsed state in localStorage keyed by column name.
**Warning signs:** Collapsed column shows no information.

### Pitfall 5: fetch Calls in $effect Without Cleanup
**What goes wrong:** Race conditions when project switches rapidly, stale data renders.
**Why it happens:** $effect fires on dependency change; if project_id changes twice quickly, the first fetch resolves after the second starts.
**How to avoid:** Use an AbortController or track the current request identity and discard stale responses.
**Warning signs:** Flash of wrong project's tasks.

### Pitfall 6: Command Palette Keyboard Conflict
**What goes wrong:** Ctrl+K intercepted by browser (e.g., Chrome's address bar focus on some platforms).
**Why it happens:** Browser has its own keyboard shortcuts.
**How to avoid:** Call `event.preventDefault()` in the handler on `<svelte:window>`. Test in target browsers.
**Warning signs:** Command palette not opening on Ctrl+K.

## Code Examples

Verified patterns from official sources:

### Board Layout with CSS Grid and Collapsible Columns
```svelte
<!-- Board.svelte -->
<script lang="ts">
  import Column from './Column.svelte';
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';

  const columns = ['backlog', 'planning', 'running', 'review', 'done'] as const;
</script>

<div
  class="grid h-full gap-2 p-2"
  style="grid-template-columns: {columns.map(c =>
    uiStore.collapsedColumns.has(c) ? '48px' : '1fr'
  ).join(' ')};"
>
  {#each columns as status}
    <Column
      {status}
      tasks={taskStore.byStatus[status]}
      collapsed={uiStore.collapsedColumns.has(status)}
      matchingIds={taskStore.matchingIds}
      searchActive={taskStore.searchQuery.trim().length > 0}
      ontoggle={() => uiStore.toggleColumn(status)}
    />
  {/each}
</div>
```

### Task Card with Agent Badge and Status Dot
```svelte
<!-- TaskCard.svelte -->
<script lang="ts">
  import type { Task } from '$lib/types';

  interface Props {
    task: Task;
    dimmed?: boolean;
  }

  let { task, dimmed = false }: Props = $props();

  const agentColors: Record<string, string> = {
    claude: 'bg-purple-500/20 text-purple-300',
    codex: 'bg-green-500/20 text-green-300',
    gemini: 'bg-blue-500/20 text-blue-300',
    copilot: 'bg-gray-500/20 text-gray-300',
    opencode: 'bg-amber-500/20 text-amber-300',
  };

  // Phase status dot comes from session polling (Phase 5 integration point)
  // For Phase 4, default to 'exited' gray dot
</script>

<div
  class="rounded-md border border-[var(--color-border)] bg-[var(--color-surface)]
         p-3 cursor-pointer hover:bg-[var(--color-surface-hover)] transition-opacity"
  class:opacity-30={dimmed}
>
  <div class="flex items-center justify-between gap-2">
    <span class="text-sm font-medium text-[var(--color-text)] truncate">{task.title}</span>
    <span class="w-2 h-2 rounded-full bg-gray-500 shrink-0" title="Exited"></span>
  </div>
  <div class="mt-1.5">
    <span class="text-xs px-1.5 py-0.5 rounded {agentColors[task.agent] ?? 'bg-gray-500/20 text-gray-300'}">
      {task.agent}
    </span>
  </div>
</div>
```

### Keyboard Shortcuts on Window
```svelte
<!-- +layout.svelte -->
<!-- Source: https://svelte.dev/docs/svelte/$effect -->
<script lang="ts">
  import { uiStore } from '$lib/stores/ui.svelte';

  function handleKeydown(e: KeyboardEvent) {
    // Skip if user is typing in an input/textarea
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

    if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      uiStore.toggleCommandPalette();
    } else if (e.key === 'o' && !e.metaKey && !e.ctrlKey) {
      uiStore.openCreateModal();
    } else if (e.key === 'e' && !e.metaKey && !e.ctrlKey) {
      uiStore.toggleSidebar();
    } else if (e.key === '/' && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      uiStore.focusSearch();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />
```

### TypeScript Interfaces Matching API Models
```typescript
// src/lib/types/index.ts
// Source: crates/agtx-core/src/db/models.rs

export type TaskStatus = 'backlog' | 'planning' | 'running' | 'review' | 'done';

export interface Task {
  id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  agent: string;
  project_id: string;
  session_name: string | null;
  worktree_path: string | null;
  branch_name: string | null;
  pr_number: number | null;
  pr_url: string | null;
  plugin: string | null;
  cycle: number;
  created_at: string; // RFC3339
  updated_at: string; // RFC3339
}

export interface Project {
  id: string;
  name: string;
  path: string;
  github_url: string | null;
  default_agent: string | null;
  last_opened: string; // RFC3339
}

export type PhaseStatus = 'working' | 'idle' | 'ready' | 'exited';
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Svelte stores (writable/readable) | Svelte 5 runes ($state, $derived) | Oct 2024 (Svelte 5.0) | Stores still work but runes are the recommended pattern |
| create-svelte CLI | sv create CLI | 2024 | New CLI with `sv add` for addons |
| $: reactive labels | $derived() rune | Oct 2024 | Explicit over implicit; works outside .svelte files |
| export let (props) | $props() rune | Oct 2024 | Destructured props via $props() |
| on:click directive | onclick attribute | Oct 2024 | Standard DOM event attributes |

**Deprecated/outdated:**
- `create-svelte`: Replaced by `sv create`
- `$:` reactive labels: Replaced by `$derived` rune
- `export let` for props: Replaced by `$props()` rune
- Svelte stores (writable/readable): Still supported but runes preferred for new code
- `on:directive` syntax: Replaced by standard event attributes (onclick, onkeydown)

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | vitest + @testing-library/svelte |
| Config file | `web/vitest.config.ts` -- Wave 0 |
| Quick run command | `cd web && npx vitest run --reporter=verbose` |
| Full suite command | `cd web && npx vitest run` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BOARD-01 | 5 columns render with correct headers | unit | `cd web && npx vitest run src/lib/components/Board.test.ts -t "renders five columns"` | Wave 0 |
| BOARD-02 | Task card shows title, agent badge, status dot | unit | `cd web && npx vitest run src/lib/components/TaskCard.test.ts` | Wave 0 |
| BOARD-03 | Create task modal submits to API | unit | `cd web && npx vitest run src/lib/components/CreateTaskModal.test.ts` | Wave 0 |
| BOARD-04 | Delete confirm dialog calls API on confirm | unit | `cd web && npx vitest run src/lib/components/DeleteConfirmModal.test.ts` | Wave 0 |
| BOARD-05 | Search dims non-matching cards | unit | `cd web && npx vitest run src/lib/stores/tasks.svelte.test.ts -t "search filtering"` | Wave 0 |
| BOARD-06 | Sidebar lists projects, switches active | unit | `cd web && npx vitest run src/lib/components/Sidebar.test.ts` | Wave 0 |
| BOARD-07 | Command palette opens on Ctrl+K, filters actions | unit | `cd web && npx vitest run src/lib/components/CommandPalette.test.ts` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cd web && npx vitest run --reporter=verbose`
- **Per wave merge:** `cd web && npx vitest run`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `web/` directory -- entire SvelteKit project scaffold
- [ ] `web/vitest.config.ts` -- Vitest configuration with jsdom
- [ ] `web/src/lib/components/*.test.ts` -- component tests for all BOARD-* requirements
- [ ] `web/src/lib/stores/*.svelte.test.ts` -- store logic tests
- [ ] Framework install: `npm install -D vitest @testing-library/svelte jsdom @testing-library/jest-dom`

## Open Questions

1. **Static file serving in production**
   - What we know: Dev uses Vite proxy; production needs either daemon-served static files or separate HTTP server with CORS
   - What's unclear: Whether to serve from daemon (tower-http ServeDir) or separate nginx/caddy (Phase 9 has Caddy)
   - Recommendation: For Phase 4, plan for Vite proxy in dev. Production serving is Phase 9's concern (Caddy reverse proxy). Add `cors` feature to tower-http now so manual testing against built files works. The daemon should add a CorsLayer with permissive settings (localhost only) behind a feature flag or config option.

2. **Phase status dots without WebSocket**
   - What we know: Phase 4 has no WebSocket integration (that's Phase 5). The phase status (Working/Idle/Ready/Exited) requires live session data.
   - What's unclear: How to show meaningful status indicators without real-time data
   - Recommendation: For Phase 4, default all task status dots to gray ("exited" / no session). Leave a clear integration point where Phase 5 will inject live status. This keeps the UI working without fake data.

3. **Task list scoped to active project**
   - What we know: GET /api/v1/tasks returns ALL tasks. The daemon currently uses a single default project database.
   - What's unclear: Whether the API supports filtering by project_id
   - Recommendation: Fetch all tasks and filter client-side by active project's ID. The task count per project (for sidebar) should also come from this client-side grouping. If the dataset grows, add a query parameter to the daemon API later.

## Sources

### Primary (HIGH confidence)
- Svelte 5 official docs -- $state, $derived, $effect runes (https://svelte.dev/docs/svelte/$state, https://svelte.dev/docs/svelte/$derived, https://svelte.dev/docs/svelte/$effect)
- SvelteKit official docs -- project structure, SPA mode, adapter-static (https://svelte.dev/docs/kit/project-structure, https://svelte.dev/docs/kit/single-page-apps)
- Svelte testing docs (https://svelte.dev/docs/svelte/testing)
- sv create CLI docs (https://svelte.dev/docs/cli/sv-create)
- tower-http CORS module (https://docs.rs/tower-http/latest/tower_http/cors/index.html)
- Existing codebase: agtxd API handlers (tasks.rs, projects.rs, sessions.rs), models (models.rs), config (config/mod.rs)

### Secondary (MEDIUM confidence)
- Mainmatter blog on Svelte 5 global state patterns (https://mainmatter.com/blog/2025/03/11/global-state-in-svelte-5/) -- verified against official docs
- Fuse.js documentation (https://www.fusejs.io/) -- widely used, stable API
- Vite server proxy configuration -- documented in Vite docs, used in SvelteKit projects

### Tertiary (LOW confidence)
- svelte-command-palette library -- exists but may not be maintained for Svelte 5; recommend building custom instead since the scope is limited (10-15 actions)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- SvelteKit 5, adapter-static, Tailwind, fuse.js are all stable, well-documented, and verified against official sources
- Architecture: HIGH -- patterns (runes, .svelte.ts modules, fetch wrapper) come from official Svelte 5 docs and community best practices
- Pitfalls: HIGH -- CORS, reactivity destructuring, search dimming are well-documented issues with clear solutions
- API integration: HIGH -- read actual daemon source code; API surface is fully implemented and tested

**Research date:** 2026-03-04
**Valid until:** 2026-04-04 (stable stack, 30-day validity)
