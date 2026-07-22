import { invoke } from '@tauri-apps/api/core'

/** Thin wrapper around Tauri's `invoke`: forwards resolution/rejection as-is. */
export async function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args)
}
