import type { Preferences } from './types';

/** Keep edits made during a save while accepting the engine's allocated ports. */
export function mergeSavedPreferences(current: Preferences | null, submitted: Preferences, saved: Preferences, editedDuringSave: boolean): Preferences | null {
  if (!current) return current;
  if (!editedDuringSave) return saved;
  return {
    ...current,
    api_port: current.api_port === submitted.api_port ? saved.api_port : current.api_port,
    rtmp_port: current.rtmp_port === submitted.rtmp_port ? saved.rtmp_port : current.rtmp_port,
  };
}
