import { describe, it, expect } from 'vitest';
import { serializeDrag, parseDrag, baseName, joinPath } from '../../../../frontend/src/features/dragDrop';

describe('dragDrop payload', () => {
  it('serialize produces JSON', () => {
    expect(serializeDrag({ pane: 'left', paths: ['/a', '/b'] })).toBe('{"pane":"left","paths":["/a","/b"]}');
  });

  it('parse valid payload', () => {
    expect(parseDrag('{"pane":"right","paths":["/a","/b"]}')).toEqual({ pane: 'right', paths: ['/a', '/b'] });
  });

  it('parse returns null on invalid JSON', () => {
    expect(parseDrag('not json')).toBeNull();
  });

  it('parse returns null for invalid pane', () => {
    expect(parseDrag('{"pane":"center","paths":["/a"]}')).toBeNull();
  });

  it('parse returns null when paths not array', () => {
    expect(parseDrag('{"pane":"left","paths":"/a"}')).toBeNull();
  });

  it('parse filters non-string paths', () => {
    expect(parseDrag('{"pane":"left","paths":["/a",42]}')).toEqual({ pane: 'left', paths: ['/a'] });
  });
});

describe('path utils', () => {
  it('baseName', () => {
    expect(baseName('/a/b/c.txt')).toBe('c.txt');
    expect(baseName('/dir/')).toBe('dir');
    expect(baseName('file.txt')).toBe('file.txt');
  });
  it('joinPath', () => {
    expect(joinPath('/dir', 'x.txt')).toBe('/dir/x.txt');
    expect(joinPath('/dir/', 'x.txt')).toBe('/dir/x.txt');
  });
});