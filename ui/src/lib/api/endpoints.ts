import { api } from "./client";
import type {
  ApiChain,
  SimulateRequest,
  SimulateResponse,
  ApiResultsResponse,
  ApiResultsCompleted,
  ApiHistoryEntry,
  ApiDeleteResponse,
  SseStageStart,
  SseStageEnd,
  SseProgress,
  SseLog,
  SseComplete,
  SseError,
  SseEvent,
} from "./types";

export function getChains(): Promise<ApiChain[]> {
  return api.get<ApiChain[]>("/api/chains");
}

export function simulate(body: SimulateRequest): Promise<SimulateResponse> {
  return api.post<SimulateResponse>("/api/simulate", body);
}

export async function getResults(runId: string): Promise<ApiResultsResponse> {
  return api.get<ApiResultsResponse>(`/api/simulate/${runId}/results`);
}

export async function getCompletedResults(runId: string): Promise<ApiResultsCompleted | null> {
  const res = await getResults(runId);
  if (res && "status" in res && (res.status === "running" || res.status === "pending")) {
    return null;
  }
  if (res && "status" in res && res.status === "error") {
    throw new Error((res as { error: string }).error ?? "Simulation failed");
  }
  return res as ApiResultsCompleted;
}

export function getHistory(): Promise<ApiHistoryEntry[]> {
  return api.get<ApiHistoryEntry[]>("/api/history");
}

export function getHistoryRun(runId: string): Promise<ApiResultsCompleted> {
  return api.get<ApiResultsCompleted>(`/api/history/${runId}`);
}

export function deleteHistoryRun(runId: string): Promise<ApiDeleteResponse> {
  return api.delete<ApiDeleteResponse>(`/api/history/${runId}`);
}

export function getExportJsonUrl(runId: string, baseUrl?: string): string {
  const base = baseUrl ?? import.meta.env.VITE_API_BASE_URL ?? "http://localhost:3001";
  return `${base}/api/export/${runId}/json`;
}

export function getExportCsvUrl(runId: string, baseUrl?: string): string {
  const base = baseUrl ?? import.meta.env.VITE_API_BASE_URL ?? "http://localhost:3001";
  return `${base}/api/export/${runId}/csv`;
}

export function subscribeToStatus(
  runId: string,
  handlers: {
    onStageStart?: (data: SseStageStart) => void;
    onStageEnd?: (data: SseStageEnd) => void;
    onProgress?: (data: SseProgress) => void;
    onLog?: (data: SseLog) => void;
    onComplete?: (data: SseComplete) => void;
    onError?: (data: SseError) => void;
  },
): () => void {
  const baseUrl = import.meta.env.VITE_API_BASE_URL ?? "http://localhost:3001";
  const es = new EventSource(`${baseUrl}/api/simulate/${runId}/status`);

  if (handlers.onStageStart) {
    es.addEventListener("stage_start", (e: MessageEvent) => {
      handlers.onStageStart!(JSON.parse(e.data));
    });
  }
  if (handlers.onStageEnd) {
    es.addEventListener("stage_end", (e: MessageEvent) => {
      handlers.onStageEnd!(JSON.parse(e.data));
    });
  }
  if (handlers.onProgress) {
    es.addEventListener("progress", (e: MessageEvent) => {
      handlers.onProgress!(JSON.parse(e.data));
    });
  }
  if (handlers.onLog) {
    es.addEventListener("log", (e: MessageEvent) => {
      handlers.onLog!(JSON.parse(e.data));
    });
  }
  if (handlers.onComplete) {
    es.addEventListener("complete", (e: MessageEvent) => {
      handlers.onComplete!(JSON.parse(e.data));
    });
  }
  if (handlers.onError) {
    es.addEventListener("error", (e: MessageEvent) => {
      handlers.onError!(JSON.parse(e.data));
    });
  }

  return () => es.close();
}

export async function* streamSseEvents(runId: string): AsyncGenerator<SseEvent> {
  const baseUrl = import.meta.env.VITE_API_BASE_URL ?? "http://localhost:3001";
  const queue: SseEvent[] = [];
  let resolve: (() => void) | null = null;
  let done = false;

  const es = new EventSource(`${baseUrl}/api/simulate/${runId}/status`);

  const push = (type: SseEvent["type"], data: unknown) => {
    queue.push({ type, data } as SseEvent);
    if (resolve) { resolve(); resolve = null; }
  };

  es.addEventListener("stage_start", (e: MessageEvent) => push("stage_start", JSON.parse(e.data)));
  es.addEventListener("stage_end", (e: MessageEvent) => push("stage_end", JSON.parse(e.data)));
  es.addEventListener("progress", (e: MessageEvent) => push("progress", JSON.parse(e.data)));
  es.addEventListener("log", (e: MessageEvent) => push("log", JSON.parse(e.data)));
  es.addEventListener("complete", (e: MessageEvent) => { push("complete", JSON.parse(e.data)); es.close(); done = true; });
  es.addEventListener("error", (e: MessageEvent) => { push("error", JSON.parse(e.data)); es.close(); done = true; });

  try {
    while (!done || queue.length > 0) {
      if (queue.length > 0) {
        yield queue.shift()!;
      } else {
        await new Promise<void>((r) => { resolve = r; });
      }
    }
  } finally {
    es.close();
  }
}
