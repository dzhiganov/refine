import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../App';
import type { Entry } from '../bindings/Entry';
import { Results } from '../features/write/Results';
import { initApi, setApi } from '../lib/api';
import demo from '../lib/fixtures/demo.json';
import { createMockApi } from '../lib/mock';
import { useApp } from '../stores/app';

async function renderApp() {
  setApi(createMockApi());
  const api = await initApi();
  useApp.getState().init(await api.bootstrap());
  useApp.setState({ view: 'write', draft: '', entry: null, checkError: null });
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <App />
    </QueryClientProvider>,
  );
}

describe('Write flow', () => {
  beforeEach(() => localStorage.clear());

  it('checks a text and shows the three blocks', async () => {
    await renderApp();
    const box = screen.getByLabelText('Your text');
    fireEvent.change(box, { target: { value: 'Ich habe gestern mit der Kunde gesprochen und alles ist gut.' } });
    expect(screen.getByText('11 words')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /check/i }));
    await waitFor(() => expect(screen.getByText('Fixed text')).toBeTruthy(), { timeout: 4000 });

    const mistakes = screen.getByRole('region', { name: 'Mistakes' });
    expect(within(mistakes).getByText('mit dem Kunden')).toBeTruthy();
    expect(within(mistakes).getByText('Case')).toBeTruthy();
    expect(screen.getByRole('region', { name: 'Suggestions' })).toBeTruthy();

    // Editor and result share the screen: the text stays editable, and editing marks the result stale.
    const editor = screen.getByLabelText('Your text') as HTMLTextAreaElement;
    expect(editor.value).toContain('mit der Kunde');
    act(() => {
      fireEvent.change(editor, { target: { value: `${editor.value} Danke!` } });
    });
    expect(screen.getByText('Edited since the last check')).toBeTruthy();
    expect(screen.getByRole('region', { name: 'Mistakes' })).toBeTruthy();
    expect(screen.getByRole('button', { name: /check again/i })).toBeTruthy();
  });

  it('navigates between pages from the rail', async () => {
    await renderApp();
    fireEvent.click(screen.getByLabelText('Progress'));
    await waitFor(() => expect(screen.getByText('Watching')).toBeTruthy());
    fireEvent.click(screen.getByLabelText('History'));
    await waitFor(() => expect(screen.getByPlaceholderText('Search your texts')).toBeTruthy());
    fireEvent.click(screen.getByLabelText('Settings'));
    await waitFor(() => expect(screen.getByText('Provider')).toBeTruthy());
  });
});

describe('Results', () => {
  const entries = demo.entries as unknown as Entry[];

  it('marks deletions and insertions from the word diff', async () => {
    await renderApp();
    const entry = entries.find((e) => e.mistakes.length === 2 && e.language === 'de')!;
    const { container } = render(<Results entry={entry} />);
    // Clean by default: the text to copy, no markup.
    expect(container.querySelectorAll('del').length).toBe(0);
    fireEvent.click(screen.getByRole('button', { name: 'Changes' }));
    const dels = [...container.querySelectorAll('del')].map((d) => d.textContent);
    const ins = [...container.querySelectorAll('ins')].map((d) => d.textContent);
    expect(dels.join(' ')).toContain('der');
    expect(ins.join(' ')).toContain('dem');
  });

  it('says so when there is nothing to fix', async () => {
    await renderApp();
    const entry = entries.find((e) => e.mistakes.length === 0)!;
    render(<Results entry={entry} />);
    expect(screen.getByText('Nothing to fix')).toBeTruthy();
    expect(screen.queryByText('Changes')).toBeNull();
  });
});
