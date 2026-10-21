/**
 * Regression tests covering automatic account library sync. Uses synthetic
 * fixtures; these checks do not verify real account sign-in, audio output or
 * mobile synchronization.
 */

import { describe, it, expect, vi } from "vitest";
import { createAccountSync } from "./accountSync";
function deferred<T>() {
  let resolve!: (value: T) => void, reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function fixture() {
  const request = deferred<string[]>(),
    load = vi.fn(() => request.promise),
    publish = vi.fn(),
    status = vi.fn(),
    error = vi.fn(),
    busy = vi.fn();
  return {
    request,
    load,
    publish,
    status,
    error,
    busy,
    sync: createAccountSync({ load, publish, status, error, busy }),
  };
}
const settle = async () => {
  await Promise.resolve();
  await Promise.resolve();
};
describe("automatic account library sync", () => {
  it("syncs once on connection and ignores duplicate account/focus events and concurrent clicks", async () => {
    const f = fixture();
    f.sync.observe(false);
    expect(f.load).not.toHaveBeenCalled();
    f.sync.observe(true);
    f.sync.observe(true);
    await f.sync.sync();
    expect(f.load).toHaveBeenCalledTimes(1);
    f.request.resolve(["owned-book"]);
    await settle();
    expect(f.publish).toHaveBeenCalledWith(["owned-book"]);
    expect(f.busy.mock.calls).toEqual([[true], [false]]);
    f.sync.observe(true);
    expect(f.load).toHaveBeenCalledTimes(1);
  });
  it("keeps failure visible without automatic retry loops and allows manual retry", async () => {
    const f = fixture();
    f.sync.observe(true);
    f.request.reject("Network unavailable");
    await settle();
    expect(f.error).toHaveBeenLastCalledWith(
      expect.stringContaining("Use Sync library to retry"),
    );
    f.sync.observe(true);
    expect(f.load).toHaveBeenCalledTimes(1);
    f.load.mockResolvedValueOnce(["retried-book"]);
    await f.sync.sync();
    expect(f.publish).toHaveBeenCalledWith(["retried-book"]);
    expect(f.error).toHaveBeenLastCalledWith("");
  });
  it("ignores a disconnected account result and syncs the next connection", async () => {
    const f = fixture();
    f.sync.observe(true);
    f.sync.observe(false);
    f.sync.observe(true);
    f.load.mockResolvedValueOnce(["new-account-book"]);
    f.request.resolve(["old-account-book"]);
    await settle();
    await settle();
    expect(f.load).toHaveBeenCalledTimes(2);
    expect(f.publish.mock.calls).toEqual([[["new-account-book"]]]);
  });
  it("does not update an unmounted panel", async () => {
    const f = fixture();
    f.sync.observe(true);
    f.sync.dispose();
    f.request.resolve(["book"]);
    await settle();
    expect(f.publish).not.toHaveBeenCalled();
    expect(f.status).toHaveBeenCalledTimes(1);
  });
});
