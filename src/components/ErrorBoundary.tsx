import { Component, type ErrorInfo, type ReactNode } from "react";
import { useBridgeStore } from "@/stores/bridge-store";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", error, info.componentStack);
    useBridgeStore.getState().pushUiError(
      error.message,
      info.componentStack?.slice(0, 300) ?? undefined,
    );
  }

  handleRetry = () => {
    this.setState({ hasError: false });
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center gap-3 p-8 text-center">
          <p className="text-sm font-medium text-destructive">
            Something went wrong
          </p>
          <p className="text-xs text-muted-foreground">
            A UI error occurred. Check the error log for details.
          </p>
          <button
            type="button"
            onClick={this.handleRetry}
            className="rounded-lg bg-primary px-4 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
