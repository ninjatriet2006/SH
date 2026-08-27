import re

with open("frontend/src/components/TransferDrawer.ts", "r") as f:
    content = f.read()

old_block = """    // Render tree of transferring files
    if (task.status === 'running' && task.transferringFiles && task.transferringFiles.length > 0) {
      const treeContainer = document.createElement('div');
      treeContainer.style.marginTop = '8px';
      treeContainer.style.marginLeft = '8px';
      treeContainer.style.fontFamily = 'monospace';
      treeContainer.style.fontSize = '12px';
      treeContainer.style.color = 'var(--text-secondary)';

      for (const file of task.transferringFiles) {
        const fileRow = document.createElement('div');
        fileRow.style.whiteSpace = 'nowrap';
        fileRow.style.overflow = 'hidden';
        fileRow.style.textOverflow = 'ellipsis';
        const eta = file.eta !== undefined && file.eta >= 0 ? `${file.eta}s` : '-';
        fileRow.textContent = `|_ ${file.name}: ${file.percentage}% (${formatSize(file.speed || 0)}/s, ETA: ${eta})`;
        treeContainer.appendChild(fileRow);
      }
      card.appendChild(treeContainer);
    }"""

new_block = """    // Render tree of transferring files
    if (task.status === 'running' && task.transferringFiles && task.transferringFiles.length > 0) {
      const treeContainer = document.createElement('div');
      treeContainer.style.marginTop = '8px';
      treeContainer.style.paddingLeft = '12px';
      treeContainer.style.borderLeft = '1px dashed var(--border-color)';
      treeContainer.style.display = 'flex';
      treeContainer.style.flexDirection = 'column';
      treeContainer.style.gap = '4px';

      for (const file of task.transferringFiles) {
        const fileRow = document.createElement('div');
        fileRow.style.display = 'flex';
        fileRow.style.alignItems = 'center';
        fileRow.style.justifyContent = 'space-between';
        fileRow.style.fontSize = '12px';
        fileRow.style.color = 'var(--text-secondary)';
        
        const nameSpan = document.createElement('span');
        nameSpan.style.whiteSpace = 'nowrap';
        nameSpan.style.overflow = 'hidden';
        nameSpan.style.textOverflow = 'ellipsis';
        nameSpan.style.maxWidth = '50%';
        nameSpan.textContent = `📄 ${file.name}`;
        
        const statsSpan = document.createElement('span');
        statsSpan.style.whiteSpace = 'nowrap';
        const eta = file.eta !== undefined && file.eta >= 0 ? `${file.eta}s` : '-';
        statsSpan.innerHTML = `<strong>${file.percentage}%</strong> &middot; ${formatSize(file.speed || 0)}/s &middot; ETA: ${eta}`;
        
        fileRow.appendChild(nameSpan);
        fileRow.appendChild(statsSpan);
        treeContainer.appendChild(fileRow);
      }
      card.appendChild(treeContainer);
    }"""

if old_block in content:
    content = content.replace(old_block, new_block)
    with open("frontend/src/components/TransferDrawer.ts", "w") as f:
        f.write(content)
    print("Success")
else:
    print("Failed to find block")
