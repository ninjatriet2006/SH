import { describe, it, expect } from 'vitest';
import { parseRemotePath } from './fileOps';

describe('fileOps', () => {
  describe('parseRemotePath', () => {
    it('should parse valid remote path', () => {
      const result = parseRemotePath('GoogleDrive::/Documents/Work');
      expect(result).toEqual({
        remote: 'GoogleDrive',
        realPath: '/Documents/Work'
      });
    });

    it('should handle local path correctly', () => {
      const result = parseRemotePath('/home/user/Documents');
      expect(result).toEqual({
        remote: 'Local',
        realPath: '/home/user/Documents'
      });
    });

    it('should handle edge cases', () => {
      expect(parseRemotePath('')).toEqual({ remote: '', realPath: '' });
      expect(parseRemotePath('::')).toEqual({ remote: '', realPath: '' });
      expect(parseRemotePath('MyRemote::')).toEqual({ remote: 'MyRemote', realPath: '' });
    });
  });
});
