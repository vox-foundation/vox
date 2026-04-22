import fs from 'node:fs';
import path from 'node:path';

export function getSidebar() {
  const summaryPath = path.resolve('../docs/src/SUMMARY.md');
  if (!fs.existsSync(summaryPath)) {
    console.warn(`[Sidebar] SUMMARY.md not found at ${summaryPath}`);
    return [];
  }

  const content = fs.readFileSync(summaryPath, 'utf8');
  const lines = content.split('\n');
  const sidebar = [];
  let currentGroup = null;

  for (const line of lines) {
    let match = line.match(/^#\s+(.*)/);
    if (match) {
      if (match[1] === 'Summary') continue;
      currentGroup = { label: match[1], items: [] };
      sidebar.push(currentGroup);
      continue;
    }

    match = line.match(/^-\s+\[(.*?)\]\((.*?)\)/);
    if (match) {
      let label = match[1];
      let link = match[2];
      
      // Remove trailing .md for Starlight routing
      link = link.replace(/\.md$/, '');
      // Ensure no leading slash or ./ 
      link = link.replace(/^\.?\//, '');

      const item = { label, link };
      if (currentGroup) {
        currentGroup.items.push(item);
      } else {
        sidebar.push(item);
      }
    }
  }

  // Filter out empty groups
  return sidebar.filter(item => !item.items || item.items.length > 0);
}
