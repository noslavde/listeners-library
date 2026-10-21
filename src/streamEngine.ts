/**
 * Development-only Shaka protected-streaming experiment. Delegates encrypted
 * media and platform-license requests through a transport; no audio keys are
 * extracted and runtime support is not a playable-license guarantee.
 */

import shaka from "shaka-player";
import { nativeStreamTransport, type StreamTransport } from "./streamTransport";

import type { Book, Snapshot } from "./model";
import type { Action } from "./playback";
import { registerDirectCommands, playFailure } from "./directCommands";
import { safePlaybackMessage } from "./streamLog";
type Start = {
  id: string;
  manifest: string;
  resume: number | null;
  certificate: number[];
  profile: "ios" | "android";
  runtime_version?: string | null;
  has_license_context?: boolean;
  license_auth?: string;
  grant_kind?: string;
  grant_right?: string;
  grant_start_seconds?: number | null;
  grant_end_seconds?: number | null;
};
type MediaResult = { data: number[]; content_type: string; uri: string };
let activeId: string | null = null;
let activeTransport: StreamTransport = nativeStreamTransport;
let report: (message: string) => void = () => {};
let registered = false;
function registerNetworking() {
  if (registered) return;
  registered = true;
  const plugin: shaka.extern.SchemePlugin = (
    uri,
    request,
    type,
    progress,
    headersReceived,
  ) => {
    const id = activeId;
    const reportRequest = report;
    const transport = activeTransport;
    const promise = (async (): Promise<shaka.extern.Response> => {
      if (!id) throw Error("No protected playback session");
      try {
        if (type === shaka.net.NetworkingEngine.RequestType.LICENSE) {
          if (request.drmInfo?.keySystem !== "com.widevine.alpha")
            throw Error("This session only supports Widevine license requests");
          reportRequest("Platform license request sent");
          const body = request.body;
          const challenge = body
            ? Array.from(
                ArrayBuffer.isView(body)
                  ? new Uint8Array(
                      body.buffer,
                      body.byteOffset,
                      body.byteLength,
                    )
                  : new Uint8Array(body),
              )
            : [];
          if (
            challenge.length === 2 &&
            challenge[0] === 8 &&
            challenge[1] === 4
          )
            throw Error(
              "Widevine requested a service certificate instead of a book license; Audible certificate initialization did not complete",
            );
          reportRequest(
            "Platform challenge: " +
              challenge.length +
              " bytes · message: " +
              ([
                "license-request",
                "license-renewal",
                "individualization-request",
              ].includes(request.licenseRequestType ?? "")
                ? request.licenseRequestType
                : "not reported"),
          );
          const bytes = await transport.invoke<number[]>("api_stream_license", {
            id,
            challenge,
          });
          reportRequest("Platform license response received");
          return {
            data: Uint8Array.from(bytes).buffer,
            uri,
            originalUri: uri,
            originalRequest: request,
            headers: { "content-type": "application/octet-stream" },
          };
        }
        const range = request.headers.Range ?? request.headers.range ?? null;
        const started = performance.now();
        const v = await transport.invoke<MediaResult>("api_stream_fetch", {
          id,
          uri,
          range,
        });
        const bytes = Uint8Array.from(v.data);
        reportRequest(
          "Encrypted " +
            (uri === "audible-media:manifest" ? "manifest" : "media chunk") +
            " received · " +
            bytes.byteLength +
            " bytes · " +
            Math.round(performance.now() - started) +
            " ms",
        );
        headersReceived({ "content-type": v.content_type });
        progress(performance.now() - started, bytes.byteLength, 0);
        return {
          data: bytes.buffer,
          uri: v.uri,
          originalUri: uri,
          originalRequest: request,
          headers: { "content-type": v.content_type },
          timeMs: performance.now() - started,
        };
      } catch (e) {
        if (type === shaka.net.NetworkingEngine.RequestType.LICENSE)
          reportRequest("Platform license request failed");
        reportRequest(safePlaybackMessage(e));
        throw Error(safePlaybackMessage(e));
      }
    })();
    return new shaka.util.AbortableOperation(promise, () => Promise.resolve());
  };
  for (const scheme of ["audible-media", "audible-license", "https"])
    shaka.net.NetworkingEngine.registerScheme(scheme, plugin);
}
/**
 * Maps known protected-playback failures to sanitized status text instead of
 * leaking request details.
 */
export function playbackError(error: unknown) {
  if (
    error &&
    typeof error === "object" &&
    "code" in error &&
    typeof error.code === "number"
  )
    return (
      "Protected playback failed · Shaka code " +
      error.code +
      " (see the stage status above)"
    );
  return typeof error === "string"
    ? error
    : error instanceof Error
      ? error.message
      : "Protected playback failed";
}
/**
 * Owns one experimental Shaka/media session and its transport. Teardown must
 * unregister command delivery and end the native session.
 */
export class StreamEngine {
  private player: shaka.Player | null = null;
  private id: string | null = null;
  private timer: ReturnType<typeof setInterval> | undefined;
  private stopCommands: (() => void) | undefined;
  private book: Book;
  private pausedAt = 0;
  private rewind = 5;
  private pauseChapter = false;
  private stopAt = Infinity;
  private recovery = 0;
  private last = 0;
  private intentUntil = 0;
  private disposed = false;
  private stopLocalCommands: (() => void) | undefined;
  private waitingForKey = false;
  private licenseStage = "not requested";
  private initDataSeen = false;
  private encryptedEvents = 0;
  private drmFailure = false;
  private lastFailure = "";
  constructor(
    private audio: HTMLAudioElement,
    book: Book,
    private status: (message: string) => void,
    private diagnostics: (message: string) => void = () => {},
    private log: (message: string) => void = () => {},
    private transport: StreamTransport = nativeStreamTransport,
  ) {
    this.book = book;
  }
  updateBook(book: Book) {
    if (book.asin === this.book.asin) this.book = book;
  }
  async load() {
    registerNetworking();
    shaka.polyfill.installAll();
    if (!shaka.Player.isBrowserSupported())
      throw Error("Shaka does not support this runtime");
    report = (m) => {
      if (this.disposed) return;
      this.log(m);
      if (
        this.licenseStage === "request failed" &&
        m !== "Platform license request failed"
      )
        this.lastFailure = safePlaybackMessage(m);
      if (m === "Platform license request sent")
        this.licenseStage = "request sent";
      if (m === "Platform license request failed") {
        this.licenseStage = "request failed";
        this.drmFailure = true;
        this.audio.pause();
        this.publish();
      }
      if (m === "Platform license response received")
        this.licenseStage = "response received";
      this.status(m);
    };
    this.log(this.transport.label);
    this.log(
      "Starting direct streaming · website session route · consumption: Streaming · DRM: Widevine",
    );
    this.status("Requesting protected streaming grant…");
    const start = await this.transport
      .invoke<Start>("api_stream_start", { asin: this.book.asin })
      .catch((error) => {
        this.log(safePlaybackMessage(error));
        throw error;
      });
    if (this.disposed) {
      await this.transport.invoke("api_stream_end", { id: start.id });
      return;
    }
    this.log(
      "Streaming grant and Audible service certificate received · certificate " +
        start.certificate.length +
        " bytes · API registration: " +
        (start.profile === "android" ? "Android" : "iOS"),
    );
    if (start.profile === "ios")
      this.log(
        "Compatibility warning: iOS API registration is not supported by upstream Widevine integrations; reconnect with Android for the next test",
      );
    this.log(
      "Grant access: " +
        (start.grant_kind ?? "not reported") +
        " · right: " +
        (start.grant_right ?? "not reported") +
        " · allowed range: " +
        (start.grant_start_seconds ?? "not reported") +
        "–" +
        (start.grant_end_seconds ?? "not reported") +
        " seconds",
    );
    this.log(
      "DRM request context: browser Client-ID header set · grant license ID " +
        (start.has_license_context ? "attached" : "absent"),
    );
    this.log(
      "DRM authentication: " +
        (start.license_auth === "website-session-cookie"
          ? "Audible website session · cookies retained in native memory"
          : "not reported"),
    );
    this.log(
      "Desktop host WebView2 version: " +
        (start.runtime_version ?? "not reported"),
    );
    this.id = start.id;
    activeId = start.id;
    activeTransport = this.transport;
    this.player = new shaka.Player();
    await this.player.attach(this.audio);
    this.player.configure({
      drm: {
        servers: { "com.widevine.alpha": `audible-license:${start.id}` },
        advanced: {
          "com.widevine.alpha": {
            serverCertificate: Uint8Array.from(start.certificate),
          },
        },
        preferredKeySystems: ["com.widevine.alpha"],
        delayLicenseRequestUntilPlayed: false,
        parseInbandPsshEnabled: true,
        initDataTransform: (data: Uint8Array) => {
          if (!this.initDataSeen)
            this.log(
              "Protected initialization data submitted to playback runtime",
            );
          this.initDataSeen = true;
          return data;
        },
        logLicenseExchange: false,
      },
      streaming: { bufferingGoal: 30, rebufferingGoal: 2 },
    });
    this.player.addEventListener("error", (event: Event) => {
      const detail = (event as Event & { detail?: { code?: number } }).detail;
      this.drmFailure = true;
      this.audio.pause();
      this.publish();
      this.log(`Protected playback error ${detail?.code ?? "unknown"}`);
      this.status(
        `Protected playback error ${detail?.code ?? "unknown"} · ${this.lastFailure || "underlying request detail is in the event log"}`,
      );
    });
    this.audio.addEventListener("pause", this.onPause);
    for (const name of [
      "encrypted",
      "playing",
      "waiting",
      "waitingforkey",
      "error",
      "play",
      "seeking",
      "seeked",
      "volumechange",
    ])
      this.audio.addEventListener(name, this.onMediaEvent);
    this.stopLocalCommands = registerDirectCommands((action, value) =>
      this.command(action, value),
    );
    this.stopCommands = await this.transport.commands((action, value) => {
      try {
        this.command(action, value);
      } catch (error) {
        this.status(playbackError(error));
      }
    });
    this.status("Loading the encrypted DASH manifest and platform license…");
    await this.player.load(
      start.manifest,
      start.resume ?? undefined,
      "application/dash+xml",
    );
    if (this.disposed) return;
    this.last = this.audio.currentTime;
    this.recovery = this.last;
    this.stopAt =
      this.book.chapters.find((c) => c > this.last) ?? this.audio.duration;
    this.timer = setInterval(() => this.publish(), 500);
    this.publish();
    if (!this.drmFailure)
      this.status("Direct protected stream loaded · press Play");
  }
  private onPause = () => {
    this.pausedAt = Date.now();
  };
  private onMediaEvent = (event: Event) => {
    if (this.disposed) return;
    this.log(`Audio event: ${event.type}`);
    if (
      this.drmFailure &&
      (event.type === "play" || event.type === "playing")
    ) {
      this.audio.pause();
      this.status(
        "Playback stopped · " +
          (this.lastFailure || "Audible license request failed") +
          " · see the event log",
      );
      this.publish();
      return;
    }
    if (event.type === "encrypted") this.encryptedEvents++;
    if (event.type === "waitingforkey") {
      this.waitingForKey = true;
      if (!this.drmFailure)
        this.status("Audio is waiting for a usable protected playback license");
    }
    if (event.type === "playing") {
      this.waitingForKey = false;
      this.status("Playing direct audio");
    }
    if (event.type === "waiting" && !this.audio.paused && !this.drmFailure)
      this.status("Playback is waiting for audio data or license readiness");
    if (event.type === "error")
      this.status(
        `Audio runtime error ${this.audio.error?.code ?? "unknown"} · 2: network, 3: decode, 4: unsupported format`,
      );
    this.publish();
  };
  private seek(position: number) {
    if (!Number.isFinite(position) || !Number.isFinite(this.audio.duration))
      throw Error("The direct timeline is not ready");
    this.intentUntil = Date.now() + 3000;
    this.audio.currentTime = Math.max(
      0,
      Math.min(this.audio.duration, position),
    );
    this.stopAt =
      this.book.chapters.find((c) => c > this.audio.currentTime) ??
      this.audio.duration;
  }
  private describePlayback() {
    const a = this.audio;
    // Never include key IDs, signed URLs, license bodies, or runtime error data.
    const allowed = new Set([
      "usable",
      "expired",
      "released",
      "output-restricted",
      "output-downscaled",
      "status-pending",
      "internal-error",
    ]);
    const statuses = [
      ...new Set(Object.values(this.player?.getKeyStatuses() ?? {})),
    ].map((s) => (allowed.has(s) ? s : "unknown"));
    const ready =
      [
        "no media data",
        "metadata only",
        "current audio data",
        "future audio data",
        "enough audio data",
      ][a.readyState] ?? "unknown";
    this.diagnostics(
      `${a.paused ? "Paused" : "Play requested"} · ${ready} · ${a.seeking ? "seeking · " : ""}${this.waitingForKey ? "waiting for key · " : ""}license: ${this.licenseStage} · init data: ${this.initDataSeen ? "submitted" : "not submitted"} · encrypted events: ${this.encryptedEvents} · CDM attached: ${a.mediaKeys ? "yes" : "no"} · DRM: ${this.player?.keySystem() === "com.widevine.alpha" ? "Widevine" : this.player?.keySystem() ? "other" : "not selected"} · key status: ${statuses.join(", ") || "not reported"} · ${a.muted ? "muted" : `volume ${Math.round(a.volume * 100)}%`}${a.error ? ` · media error ${a.error.code}` : ""}`,
    );
  }
  private command(action: Action, value: number) {
    if (action === "play") {
      if (this.drmFailure) {
        this.log(
          "Play ignored: license failed; load a new session after resolving the reported error",
        );
        this.status(
          "Playback stopped · " +
            (this.lastFailure || "Audible license request failed") +
            " · see the event log",
        );
        return;
      }
      this.log("Play requested");
      if (this.pausedAt && Date.now() - this.pausedAt > 60000)
        this.seek(this.audio.currentTime - this.rewind);
      if (!this.drmFailure) this.status("Starting audio…");
      void this.audio
        .play()
        .then(() => {
          if (!this.disposed) this.status("Playing direct audio");
        })
        .catch((error) => {
          if (!this.disposed) {
            if (!this.drmFailure) {
              this.log(playFailure(error));
              this.status(playFailure(error));
            }
            this.publish();
          }
        });
    }
    if (action === "pause") this.audio.pause();
    if (action === "seek") this.seek(value);
    if (action === "speed") this.audio.playbackRate = value;
    if (action === "rewind") this.rewind = Math.max(0, Math.min(30, value));
    if (action === "pauseChapter") {
      this.pauseChapter = !!value;
      this.stopAt =
        this.book.chapters.find((c) => c > this.audio.currentTime) ??
        this.audio.duration;
    }
    if (action === "recover") this.seek(this.recovery);
  }
  private publish() {
    if (!this.id || this.disposed) return;
    this.describePlayback();
    const a = this.audio;
    if (
      Date.now() > this.intentUntil &&
      Math.abs(a.currentTime - this.last) > 20
    )
      this.recovery = this.last;
    this.last = a.currentTime;
    if (this.pauseChapter && !a.paused && a.currentTime >= this.stopAt)
      a.pause();
    const ranges = (r: TimeRanges) =>
      Array.from({ length: Math.min(64, r.length) }, (_, i) => ({
        start: r.start(i),
        end: r.end(i),
      }));
    const state: Snapshot & { asin: string; sample: boolean } = {
      asin: this.book.asin,
      sample: false,
      position: Number.isFinite(a.currentTime) ? a.currentTime : 0,
      duration: Number.isFinite(a.duration) ? a.duration : 0,
      speed: a.playbackRate,
      paused: a.paused,
      available:
        !this.drmFailure && Number.isFinite(a.duration) && a.duration > 0,
      seekable: ranges(a.seekable),
      buffered: ranges(a.buffered),
    };
    void this.transport
      .invoke("api_stream_state", { id: this.id, state })
      .catch((e) => this.status(String(e)));
  }
  async destroy() {
    this.log("Direct stream stopped");
    this.disposed = true;
    clearInterval(this.timer);
    this.stopLocalCommands?.();
    this.stopCommands?.();
    this.audio.removeEventListener("pause", this.onPause);
    for (const name of [
      "encrypted",
      "playing",
      "waiting",
      "waitingforkey",
      "error",
      "play",
      "seeking",
      "seeked",
      "volumechange",
    ])
      this.audio.removeEventListener(name, this.onMediaEvent);
    this.audio.pause();
    if (this.player) await this.player.destroy();
    if (this.id) {
      await this.transport
        .invoke("api_stream_end", { id: this.id })
        .catch(() => {});
      if (activeId === this.id) activeId = null;
    }
    this.player = null;
  }
}
