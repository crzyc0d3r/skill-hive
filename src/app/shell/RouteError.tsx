import { isRouteErrorResponse, useRouteError } from "react-router-dom";
import { AlertTriangle } from "lucide-react";
import { Button, EmptyState } from "@/components/ui";

export function RouteError() {
  const error = useRouteError();
  const message = isRouteErrorResponse(error)
    ? `${error.status} ${error.statusText}`
    : error instanceof Error
      ? error.message
      : "Something went wrong.";
  return (
    <div className="flex h-full items-center justify-center p-10">
      <EmptyState
        icon={AlertTriangle}
        title="This screen hit an error"
        description={<span className="selectable mono">{message}</span>}
        action={
          <Button variant="primary" onClick={() => window.location.reload()}>
            Reload
          </Button>
        }
      />
    </div>
  );
}
