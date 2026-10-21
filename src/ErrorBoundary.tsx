/**
 * Last-resort React render failure screen. Reports only a fixed error code and
 * offers recovery without resetting saved data or exposing exception details.
 */

import { Component, type ReactNode } from "react";
import { reportUiError } from "./diagnostics";
import { DiagnosticsPanel } from "./DiagnosticsPanel";

/**
 * Contains render failures without deleting local state; diagnostics receive a
 * category code rather than the exception or React component stack.
 */
export class ErrorBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch() {
    reportUiError("react-error");
  }
  render() {
    return this.state.failed ? (
      <main role="alert">
        <h1>Something went wrong</h1>
        <p>
          Your saved data has not been reset. Close and reopen the app. If audio
          is still playing, use your media pause key before closing it.
        </p>
        <p>
          Please report the app version and what you were doing. Do not include
          account details or signed-in URLs.
        </p>
        <button onClick={() => location.reload()}>Reload this window</button>
        <DiagnosticsPanel />
      </main>
    ) : (
      this.props.children
    );
  }
}
