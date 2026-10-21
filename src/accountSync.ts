/**
 * Coordinates account-triggered library refreshes. Coalesces overlapping sync
 * requests and invalidates stale results so an old account cannot repopulate
 * the next account's UI.
 */

// One refresh per connection, including a remembered connection on app launch.
// Repeated account/focus events must not queue duplicate library requests.
/**
 * Creates a single-flight library sync coordinator. Invalidation makes
 * in-flight responses harmless; callers supply publishing/status functions so
 * transport and presentation remain separate.
 */
export function createAccountSync<T>(options: {
  load: () => Promise<T>;
  publish: (value: T) => void;
  status: (message: string) => void;
  error: (message: string) => void;
  busy: (value: boolean) => void;
}) {
  let connected = false,
    disposed = false,
    running = false,
    epoch = 0;
  const sync = async () => {
    if (disposed || !connected || running) return;
    running = true;
    const attempt = epoch;
    options.busy(true);
    options.error("");
    options.status("Syncing your library…");
    try {
      const value = await options.load();
      if (!disposed && connected && attempt === epoch) {
        options.publish(value);
        options.status("Library updated. Choose a book to listen.");
      }
    } catch (error) {
      if (!disposed && connected && attempt === epoch) {
        const message = `Library sync failed. Use Sync library to retry. ${String(error)}`;
        options.error(message);
        options.status(message);
      }
    } finally {
      running = false;
      if (!disposed) {
        options.busy(false);
        if (connected && attempt !== epoch) void sync();
      }
    }
  };
  return {
    sync,
    observe(value: boolean) {
      if (disposed || value === connected) return;
      connected = value;
      epoch++;
      if (connected) void sync();
    },
    dispose() {
      disposed = true;
      epoch++;
    },
  };
}
