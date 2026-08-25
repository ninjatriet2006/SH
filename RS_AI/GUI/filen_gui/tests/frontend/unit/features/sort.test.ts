import { describe, it, expect } from 'vitest';
import { sortFiles, typeLabel } from '../../../../frontend/src/features/sort';
import type { FileItem } from '../../../../frontend/src/store';

function item(name: string, is_dir: boolean, size = 0, mod_time = '2024-01-01T00:00:00Z', file_type?: string): FileItem {
  return { name, is_dir, size, mod_time, file_type };
}

describe('sortFiles', () => {
  const files: FileItem[] = [
    item('b.txt', false, 200, '2024-01-02T00:00:00Z', 'TXT'),
    item('a.txt', false, 100, '2024-01-01T00:00:00Z', 'TXT'),
    item('Zdir', true, 0, '2024-01-03T00:00:00Z'),
    item('Adir', true, 0, '2024-01-04T00:00:00Z'),
  ];

  it('dirs always first regardless of dir', () => {
    const asc = sortFiles(files, 'name', true, 'asc');
    expect(asc[0].is_dir).toBe(true);
    expect(asc[1].is_dir).toBe(true);
    expect(asc[2].is_dir).toBe(false);
    expect(asc[3].is_dir).toBe(false);
    const desc = sortFiles(files, 'name', true, 'desc');
    expect(desc[0].is_dir).toBe(true);
    expect(desc[1].is_dir).toBe(true);
  });

  it('sorts by name asc case-insensitive', () => {
    const r = sortFiles(files, 'name', true, 'asc');
    expect(r.map((f) => f.name)).toEqual(['Adir', 'Zdir', 'a.txt', 'b.txt']);
  });

  it('sorts by name desc', () => {
    const r = sortFiles(files, 'name', true, 'desc');
    expect(r.map((f) => f.name)).toEqual(['Zdir', 'Adir', 'b.txt', 'a.txt']);
  });

  it('sorts by size asc/desc', () => {
    const asc = sortFiles(files, 'size', true, 'asc');
    expect(asc.map((f) => f.name)).toEqual(['Zdir', 'Adir', 'a.txt', 'b.txt']);
    const desc = sortFiles(files, 'size', true, 'desc');
    expect(desc.map((f) => f.name)).toEqual(['Zdir', 'Adir', 'b.txt', 'a.txt']);
  });

  it('sorts by date asc/desc', () => {
    const asc = sortFiles(files, 'date', true, 'asc');
    expect(asc.map((f) => f.name)).toEqual(['Zdir', 'Adir', 'a.txt', 'b.txt']);
    const desc = sortFiles(files, 'date', true, 'desc');
    expect(desc.map((f) => f.name)).toEqual(['Adir', 'Zdir', 'b.txt', 'a.txt']);
  });

  it('sorts by type', () => {
    const r = sortFiles(files, 'type', true, 'asc');
    // dirs first (Folder), then TXT files
    expect(r[0].is_dir).toBe(true);
    expect(r[2].file_type).toBe('TXT');
  });

  it('does not mutate input', () => {
    const copy = [...files];
    sortFiles(files, 'name');
    expect(files).toEqual(copy);
  });

  it('dirsFirst=false mixes dirs and files', () => {
    const r = sortFiles(files, 'name', false, 'asc');
    expect(r.map((f) => f.name)).toEqual(['a.txt', 'Adir', 'b.txt', 'Zdir']);
  });
});

describe('typeLabel', () => {
  it('uses file_type when present', () => {
    expect(typeLabel(item('x.pdf', false, 0, '', 'PDF'))).toBe('PDF');
  });
  it('returns Folder for dirs', () => {
    expect(typeLabel(item('x', true, 0))).toBe('Folder');
  });
  it('extracts extension uppercase', () => {
    expect(typeLabel(item('report.pdf', false, 0))).toBe('PDF');
  });
  it('returns empty for no extension', () => {
    expect(typeLabel(item('README', false, 0))).toBe('');
  });
});