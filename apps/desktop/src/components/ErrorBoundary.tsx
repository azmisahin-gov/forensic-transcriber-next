import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Catches render errors so a crash shows a readable Turkish message instead of
 * a blank window, and offers the raw message for diagnosis.
 */
export class ErrorBoundary extends Component<Props, State> {
  override state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  override componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("Arayüz hatası:", error, info.componentStack);
  }

  override render(): ReactNode {
    const { error } = this.state;
    if (!error) {
      return this.props.children;
    }
    return (
      <div className="crash" role="alert">
        <h1>Arayüzde beklenmeyen bir hata oluştu</h1>
        <p>
          Uygulama verileriniz korunur. Sorunu bildirirken aşağıdaki teknik
          ayrıntıyı paylaşabilirsiniz.
        </p>
        <pre className="crash-detail">{error.message}</pre>
        <button type="button" onClick={() => this.setState({ error: null })}>
          Yeniden dene
        </button>
      </div>
    );
  }
}
