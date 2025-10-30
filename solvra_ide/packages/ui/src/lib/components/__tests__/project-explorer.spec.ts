import { fireEvent, render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ProjectExplorer from '../ProjectExplorer.svelte';
import type { ProjectNode } from '../../types';

const sampleTree: ProjectNode = {
  name: 'workspace',
  path: '/workspace',
  is_dir: true,
  children: [
    {
      name: 'src',
      path: '/workspace/src',
      is_dir: true,
      children: [
        {
          name: 'main.svs',
          path: '/workspace/src/main.svs',
          is_dir: false,
          children: []
        }
      ]
    },
    {
      name: 'README.md',
      path: '/workspace/README.md',
      is_dir: false,
      children: []
    }
  ]
};

describe('ProjectExplorer', () => {
  it('dispatches open on file double click', async () => {
    const handleOpen = vi.fn();
    const { getByTestId, component } = render(ProjectExplorer, { project: sampleTree });
    component.$on('open', (event) => handleOpen(event.detail));
    const fileNode = getByTestId('tree-item-README.md').querySelector('.label');
    await fireEvent.dblClick(fileNode!);
    expect(handleOpen).toHaveBeenCalledWith('/workspace/README.md');
  });

  it('supports drag and drop reordering', async () => {
    const handleMove = vi.fn();
    const { getByTestId, component } = render(ProjectExplorer, { project: sampleTree });
    component.$on('move', (event) => handleMove(event.detail));
    const source = getByTestId('tree-item-README.md');
    const target = getByTestId('tree-item-src');
    const dataTransfer = {
      data: {} as Record<string, string>,
      setData(key: string, value: string) {
        this.data[key] = value;
      },
      getData(key: string) {
        return this.data[key];
      }
    };
    await fireEvent.dragStart(source, { dataTransfer });
    await fireEvent.drop(target, { dataTransfer });
    expect(handleMove).toHaveBeenCalled();
  });
});
