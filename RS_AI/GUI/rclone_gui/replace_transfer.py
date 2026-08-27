import re

with open("frontend/src/features/transferManager.ts", "r") as f:
    content = f.read()

old_block = """        if (task && task.status === 'running') {
          task.bytesDone = payload.stats.bytes;
          task.totalBytes = payload.stats.totalBytes;
          task.speed = payload.stats.speed;
          if (payload.stats.transferring) {"""

new_block = """        if (task && task.status === 'running') {
          task.bytesDone = payload.stats.bytes;
          task.totalBytes = payload.stats.totalBytes;
          task.speed = payload.stats.speed;
          if (task.totalBytes > 0) {
              task.progress = task.bytesDone / task.totalBytes;
          }
          if (payload.stats.transferring) {"""

if old_block in content:
    content = content.replace(old_block, new_block)
    with open("frontend/src/features/transferManager.ts", "w") as f:
        f.write(content)
    print("Success")
else:
    print("Failed to find block")
