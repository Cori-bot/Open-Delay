import { describe, expect, it } from 'vitest';
import { defaultPreferences } from './types';
import { mergeSavedPreferences } from './preferences';

describe('preferences saved while editing', () => {
  it('retains unrelated edits and accepts ports allocated by the engine', () => {
    const submitted = { ...defaultPreferences, api_port: 0, rtmp_port: 0 };
    const current = { ...submitted, language: 'en' as const };
    const saved = { ...submitted, api_port: 42001, rtmp_port: 42002 };
    expect(mergeSavedPreferences(current, submitted, saved, true)).toMatchObject({ language: 'en', api_port: 42001, rtmp_port: 42002 });
  });
  it('does not overwrite a port edited after submission', () => {
    const submitted = { ...defaultPreferences, api_port: 0, rtmp_port: 0 };
    const current = { ...submitted, api_port: 43000 };
    const saved = { ...submitted, api_port: 42001, rtmp_port: 42002 };
    expect(mergeSavedPreferences(current, submitted, saved, true)).toMatchObject({ api_port: 43000, rtmp_port: 42002 });
  });
});
