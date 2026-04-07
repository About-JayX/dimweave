import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-screen flex-col items-center justify-center gap-4 bg-background p-8 text-foreground">
          <h1 className="text-lg font-semibold text-destructive">UI crashed</h1>
          <pre className="max-w-[80vw] overflow-auto rounded bg-muted p-4 text-xs">
            {this.state.error.message}
            {"\n"}
            {this.state.error.stack}
          </pre>
          <button
            className="rounded bg-primary px-4 py-2 text-sm text-primary-foreground"
            onClick={() => this.setState({ error: null })}
          >
            Try to recover
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
