/**
 * Replacement for `@tauri-apps/api/event` in `--mode mock`. `listen` registers
 * the handler on a small in-memory bus and returns the unlisten. Unlike the
 * baseline (which was a no-op), the mock CAN now emit events — used by `core.ts`
 * to simulate the embedded model download progress
 * (`embedded://download-progress`), the key moment of the Models tab. Affects
 * mock mode only; does not end up in the real build.
 * Enabled by an alias in vite.config.ts.
 */

export type UnlistenFn = () => void;

export interface Event<T> {
  event: string;
  id: number;
  payload: T;
}

export type EventCallback<T> = (event: Event<T>) => void;

let nextId = 1;
const listeners = new Map<string, Set<EventCallback<unknown>>>();

export async function listen<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  const set = listeners.get(event) ?? new Set();
  set.add(handler as EventCallback<unknown>);
  listeners.set(event, set);
  return () => {
    set.delete(handler as EventCallback<unknown>);
  };
}

export async function once<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  return listen(event, handler);
}

export async function emit(event: string, payload?: unknown): Promise<void> {
  emitMock(event, payload);
}

/** Delivers a payload to all registered handlers of an event (mock). */
export function emitMock<T>(event: string, payload: T): void {
  const set = listeners.get(event);
  if (!set) return;
  const wrapped: Event<T> = { event, id: nextId++, payload };
  for (const handler of set) (handler as EventCallback<T>)(wrapped);
}
