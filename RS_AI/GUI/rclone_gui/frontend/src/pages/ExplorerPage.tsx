/*
[INTEGRITY NOTES]
- Mục đích: Trang duyệt tệp tin chính (Explorer View) dạng Dual-Pane.
- Trách nhiệm: Tích hợp DualPaneExplorer, điều phối các modal Xung đột, Thuộc tính, và Tìm kiếm sâu.
- Tương tác: Dùng `useExplorerStore`.
*/

import React, { useState } from 'react';
import type { ConflictInfo } from '../../../bridge/types';
import { ConflictModal } from '../components/ConflictModal';
import { DualPaneExplorer } from '../components/DualPaneExplorer';
import { PropertiesModal } from '../components/PropertiesModal';
import { SearchModal } from '../components/SearchModal';

export const ExplorerPage: React.FC = () => {
  const [conflicts, setConflicts] = useState<ConflictInfo[] | null>(null);
  const [onProceedConflict, setOnProceedConflict] = useState<((skips: string[]) => void) | null>(null);
  const [propPath, setPropPath] = useState<string | null>(null);
  const [searchPath, setSearchPath] = useState<string | null>(null);

  const handleShowConflicts = (list: ConflictInfo[], onProceed: (skips: string[]) => void) => {
    setConflicts(list);
    setOnProceedConflict(() => onProceed);
  };

  return (
    <div className="page-container" style={{ padding: '0.75rem' }}>
      <DualPaneExplorer
        onShowConflicts={handleShowConflicts}
        onShowProperties={(path) => setPropPath(path)}
        onShowSearch={(path) => setSearchPath(path)}
      />

      {conflicts && onProceedConflict && (
        <ConflictModal
          conflicts={conflicts}
          onClose={() => {
            setConflicts(null);
            setOnProceedConflict(null);
          }}
          onProceed={(skips) => {
            onProceedConflict(skips);
            setConflicts(null);
            setOnProceedConflict(null);
          }}
        />
      )}

      {propPath && <PropertiesModal path={propPath} onClose={() => setPropPath(null)} />}

      {searchPath && <SearchModal basePath={searchPath} onClose={() => setSearchPath(null)} />}
    </div>
  );
};
