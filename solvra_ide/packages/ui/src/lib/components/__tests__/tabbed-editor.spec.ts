import { fireEvent, render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import TabbedEditor from '../TabbedEditor.svelte';
import type { EditorTabViewModel } from '../../types';

const tabs: EditorTabViewModel[] = [
  {
    id: '1',
    title: 'main.svs',
    path: '/workspace/src/main.svs',
    content: 'let a = 1',
    dirty: false
  },
  {
    id: '2',
    title: 'lib.svs',
    path: '/workspace/src/lib.svs',
    content: 'fn add() {}',
    dirty: false
  }
];

describe('TabbedEditor', () => {
  it('emits save for active tab', async () => {
    const onSave = vi.fn();
    const { getByText, component } = render(TabbedEditor, { tabs });
    component.$on('save', (event) => onSave(event.detail));
    await fireEvent.click(getByText('Save'));
    expect(onSave).toHaveBeenCalledWith(tabs[0]);
  });

  it('switches tabs on click', async () => {
    const onActivate = vi.fn();
    const { getByText, component } = render(TabbedEditor, { tabs });
    component.$on('activate', (event) => onActivate(event.detail));
    await fireEvent.click(getByText('lib.svs'));
    expect(onActivate).toHaveBeenCalledWith(tabs[1]);
  });
});
