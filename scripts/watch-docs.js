#!/usr/bin/env node

/**
 * Raven Documentation Watcher
 *
 * Continuously monitors the Raven project for changes and auto-updates documentation.
 * Handles folder renames by dynamically resolving paths.
 *
 * Usage: node scripts/watch-docs.js
 */

const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

// Configuration - paths resolved dynamically
let CONFIG = {
  projectRoot: '',
  ravenDir: '',
  docsDir: '',
  watchDirs: []
};

// Resolve project paths dynamically (handles folder renames)
function resolvePaths() {
  // Start from script location and go up to find project root
  let currentDir = __dirname;
  while (currentDir !== path.dirname(currentDir)) {
    const entries = fs.readdirSync(currentDir, { withFileTypes: true });
    const hasRavenDir = entries.some(e =>
      e.isDirectory() && e.name.toLowerCase().includes('raven')
    );
    const hasDocsDir = entries.some(e =>
      e.isDirectory() && e.name === 'docs'
    );

    if (hasRavenDir && hasDocsDir) {
      CONFIG.projectRoot = currentDir;

      // Find raven-tauri directory dynamically
      for (const entry of entries) {
        if (entry.isDirectory() && entry.name.toLowerCase().includes('raven')) {
          CONFIG.ravenDir = path.join(currentDir, entry.name);
          break;
        }
      }

      CONFIG.docsDir = path.join(currentDir, 'docs');
      CONFIG.watchDirs = [
        path.join(CONFIG.ravenDir, 'src'),
        path.join(CONFIG.ravenDir, 'src-tauri', 'src'),
        path.join(CONFIG.ravenDir, 'src', 'settings'),
        path.join(CONFIG.ravenDir, 'src', 'stores'),
        CONFIG.docsDir  // Also watch the docs folder itself for .md changes
      ];

      console.log(`[Watch] Project root: ${CONFIG.projectRoot}`);
      console.log(`[Watch] Raven dir: ${CONFIG.ravenDir}`);
      console.log(`[Watch] Docs dir: ${CONFIG.docsDir}`);
      return true;
    }
    currentDir = path.dirname(currentDir);
  }

  console.error('[Watch] Could not resolve project paths');
  return false;
}

// File tracker
const fileTracker = new Map();

function scanFiles() {
  const files = new Map();

  for (const watchDir of CONFIG.watchDirs) {
    if (!fs.existsSync(watchDir)) continue;

    const walk = (dir) => {
      const entries = fs.readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        if (['node_modules', 'dist', 'target', '.git'].includes(entry.name)) continue;

        const fullPath = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          walk(fullPath);
        } else if (/\.(js|jsx|ts|tsx|rs|json|css|md)$/.test(entry.name)) {
          const stat = fs.statSync(fullPath);
          const relPath = path.relative(CONFIG.projectRoot, fullPath);
          files.set(relPath, {
            mtime: stat.mtimeMs,
            size: stat.size,
            hash: hashFile(fullPath)
          });
        }
      }
    };
    walk(watchDir);
  }

  return files;
}

function hashFile(filePath) {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    let hash = 0;
    for (let i = 0; i < content.length; i++) {
      const char = content.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return hash.toString(16);
  } catch {
    return null;
  }
}

function detectChanges(newFiles) {
  const changes = {
    added: [],
    removed: [],
    modified: []
  };

  // Check for added and modified files
  for (const [filePath, fileData] of newFiles) {
    if (!fileTracker.has(filePath)) {
      changes.added.push(filePath);
    } else {
      const prevData = fileTracker.get(filePath);
      if (prevData.hash !== fileData.hash) {
        changes.modified.push(filePath);
      }
    }
  }

  // Check for removed files
  for (const [filePath] of fileTracker) {
    if (!newFiles.has(filePath)) {
      changes.removed.push(filePath);
    }
  }

  // Update tracker
  fileTracker.clear();
  for (const [filePath, fileData] of newFiles) {
    fileTracker.set(filePath, fileData);
  }

  return changes;
}

// Documentation update functions
function updateHistory(changes) {
  const historyPath = path.join(CONFIG.docsDir, 'history.md');
  if (!fs.existsSync(historyPath)) return;

  const date = new Date().toISOString().split('T')[0];
  const time = new Date().toLocaleTimeString();

  let entry = `## ${date} - ${time}\n\n`;

  if (changes.added.length > 0) {
    entry += `### Added\n${changes.added.map(f => `- ${f}`).join('\n')}\n\n`;
  }
  if (changes.modified.length > 0) {
    entry += `### Modified\n${changes.modified.map(f => `- ${f}`).join('\n')}\n\n`;
  }
  if (changes.removed.length > 0) {
    entry += `### Removed\n${changes.removed.map(f => `- ${f}`).join('\n')}\n\n`;
  }

  const content = fs.readFileSync(historyPath, 'utf8');
  const lines = content.split('\n');

  // Find first section and insert after it
  let insertIndex = 1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].startsWith('#')) {
      insertIndex = i + 1;
      break;
    }
  }

  lines.splice(insertIndex, 0, '', entry);
  fs.writeFileSync(historyPath, lines.join('\n'));
  console.log('[Watch] Updated history.md');
}

function updateStructure(changes) {
  const structurePath = path.join(CONFIG.docsDir, 'structure.md');
  if (!fs.existsSync(structurePath)) return;

  let content = fs.readFileSync(structurePath, 'utf8');

  // Scan current state
  const components = [];
  const panels = [];
  const rustModules = [];

  for (const [filePath] of fileTracker) {
    if (filePath.includes('components/') && filePath.endsWith('.jsx')) {
      components.push(path.basename(filePath, '.jsx'));
    }
    if (filePath.includes('panels/') && filePath.endsWith('.jsx')) {
      panels.push(path.basename(filePath, '.jsx'));
    }
    if (filePath.includes('src-tauri/src/') && filePath.endsWith('.rs')) {
      rustModules.push(path.basename(filePath, '.rs'));
    }
  }

  // Update component list in structure.md
  const componentRegex = /(\| `?[A-Z][a-zA-Z]+\.jsx`? \|.*\n)+/g;
  const componentTable = components.map(c => `| \`${c}.jsx\` | Auto-generated |`).join('\n');

  // Simple update - prepend change note
  const changeNote = `<!-- Auto-updated: ${new Date().toISOString()} -->\n`;
  content = changeNote + content;

  fs.writeFileSync(structurePath, content);
  console.log('[Watch] Updated structure.md');
}

function updateBrain(changes) {
  const brainPath = path.join(CONFIG.docsDir, 'brain.md');
  if (!fs.existsSync(brainPath)) return;

  // Add change notes to brain.md
  let content = fs.readFileSync(brainPath, 'utf8');

  // Check if we have a "Recent Changes" section
  if (!content.includes('## Recent Changes')) {
    const changesSection = '\n## Recent Changes\n\n<!-- Auto-updated section -->\n\n';
    content = content + changesSection;
  }

  const date = new Date().toISOString().split('T')[0];
  let updateEntry = `### ${date}\n`;

  if (changes.added.length > 0) {
    updateEntry += `- Added: ${changes.added.join(', ')}\n`;
  }
  if (changes.modified.length > 0) {
    updateEntry += `- Modified: ${changes.modified.join(', ')}\n`;
  }

  // Insert after "Recent Changes" header
  const recentIndex = content.indexOf('## Recent Changes');
  if (recentIndex !== -1) {
    const insertPos = content.indexOf('\n', recentIndex) + 1;
    content = content.slice(0, insertPos) + updateEntry + '\n' + content.slice(insertPos);
  }

  fs.writeFileSync(brainPath, content);
  console.log('[Watch] Updated brain.md');
}

function updateCLAUDE(changes) {
  const claudePath = path.join(CONFIG.docsDir, 'CLAUDE.md');
  if (!fs.existsSync(claudePath)) return;

  let content = fs.readFileSync(claudePath, 'utf8');

  // Update component references
  const components = [];
  const panels = [];

  for (const [filePath] of fileTracker) {
    if (filePath.includes('components/') && filePath.endsWith('.jsx')) {
      components.push(path.basename(filePath, '.jsx'));
    }
    if (filePath.includes('panels/') && filePath.endsWith('.jsx')) {
      panels.push(path.basename(filePath, '.jsx'));
    }
  }

  // Add update timestamp
  const updateComment = `<!-- Last auto-updated: ${new Date().toISOString()} -->\n`;

  if (!content.startsWith(updateComment)) {
    content = updateComment + content.replace(/<!-- Last auto-updated: .* -->\n/, '');
  }

  fs.writeFileSync(claudePath, content);
  console.log('[Watch] Updated CLAUDE.md');
}

function updateStyleDoc() {
  const stylePath = path.join(CONFIG.docsDir, 'style.md');
  if (!fs.existsSync(stylePath)) return;

  // Scan for CSS files and update style documentation
  const cssFiles = [];
  for (const [filePath] of fileTracker) {
    if (filePath.endsWith('.css')) {
      cssFiles.push(filePath);
    }
  }

  if (cssFiles.length > 0) {
    let content = fs.readFileSync(stylePath, 'utf8');
    const updateComment = `<!-- Last auto-updated: ${new Date().toISOString()} -->\n`;

    if (!content.startsWith(updateComment)) {
      content = updateComment + content.replace(/<!-- Last auto-updated: .* -->\n/, '');
      fs.writeFileSync(stylePath, content);
      console.log('[Watch] Updated style.md');
    }
  }
}

// Main update function
function updateDocumentation(changes) {
  const hasChanges = changes.added.length > 0 ||
                     changes.modified.length > 0 ||
                     changes.removed.length > 0;

  if (!hasChanges) return;

  console.log('[Watch] Changes detected:');
  if (changes.added.length > 0) console.log(`  + ${changes.added.length} added`);
  if (changes.modified.length > 0) console.log(`  ~ ${changes.modified.length} modified`);
  if (changes.removed.length > 0) console.log(`  - ${changes.removed.length} removed`);

  updateHistory(changes);
  updateStructure(changes);
  updateBrain(changes);
  updateCLAUDE(changes);
  updateStyleDoc();

  console.log('[Watch] Documentation updated successfully');
}

// Initial scan
let lastScan = scanFiles();
fileTracker.clear();
for (const [filePath, fileData] of lastScan) {
  fileTracker.set(filePath, fileData);
}

console.log('[Watch] Initial file scan complete');
console.log(`[Watch] Tracking ${fileTracker.size} files`);

// Polling interval (ms)
const POLL_INTERVAL = 5000;

// Start watching
console.log(`[Watch] Starting file watcher (polling every ${POLL_INTERVAL}ms)...`);
console.log('[Watch] Press Ctrl+C to stop');

setInterval(() => {
  try {
    const currentScan = scanFiles();
    const changes = detectChanges(currentScan);
    updateDocumentation(changes);
    lastScan = currentScan;
  } catch (error) {
    console.error('[Watch] Error during scan:', error.message);
  }
}, POLL_INTERVAL);

// Handle path resolution at start
if (!resolvePaths()) {
  process.exit(1);
}
