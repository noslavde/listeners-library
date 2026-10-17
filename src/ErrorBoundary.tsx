import { Component, type ReactNode } from "react";

export class ErrorBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
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
      </main>
    ) : (
      this.props.children
    );
  }
}
