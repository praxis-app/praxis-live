import { useQuery } from "@tanstack/react-query";
import { Card, CardContent } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";

type HealthResponse = {
  status: string;
};

function App() {
  const { data, error, isPending } = useQuery({
    queryKey: ["health"],
    queryFn: async (): Promise<HealthResponse> => {
      const response = await fetch("/api/health", {
        headers: {
          Accept: "application/json",
        },
      });

      if (!response.ok) {
        throw new Error(`Health check failed with status ${response.status}.`);
      }

      return response.json() as Promise<HealthResponse>;
    },
  });

  return (
    <main className="flex min-h-screen items-center justify-center bg-muted/30 px-6 py-12">
      <Card className="w-full max-w-md border-border/70 shadow-sm">
        <CardContent className="p-6">
          {isPending ? (
            <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
              <Spinner />
              <span>Loading health check...</span>
            </div>
          ) : error ? (
            <p className="text-sm text-destructive">
              {error instanceof Error ? error.message : "Health check failed."}
            </p>
          ) : (
            <pre className="overflow-x-auto rounded-md bg-background px-4 py-3 text-sm leading-6 text-foreground">
              {JSON.stringify(data, null, 2)}
            </pre>
          )}
        </CardContent>
      </Card>
    </main>
  );
}

export default App;
