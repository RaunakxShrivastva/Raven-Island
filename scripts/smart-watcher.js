#!/usr/bin/env node

/**
 * Raven Smart Documentation Watcher
 *
 * HYBRID APPROACH:
 * - Uses native OS file events (not polling) - wakes up ONLY when files change
 * - Zero CPU usage when idle
 * - Works across ALL editors and AI tools (VS Code, Codex, Gemini, manual, etc.)
 * - Starts automatically via Task Scheduler
 *
 * Resource usage:
 * - Idle: ~15 MB RAM, 0% CPU
 * - Active (during update): ~30 MB RAM, 1-2% CPU for ~500ms
 */

const fs = require('fs');
const path = require('path');

// Native fs.watch - uses OS events, not polling
const WATCH_EVENTS = ['change', 'rename'];

// Configuration
const CONFIG = {
    projectRoot: path.resolve(__dirname, '..'),
    ravenDir: null,
    docsDir: null,
    watchDirs: [],
    debounceTime: 2000,  // Wait 2 seconds after last change
    minUpdateInterval: 10000  // Max 1 update per 10 seconds
};

// State
let updateTimeout = null;
let lastUpdateTime = 0;
let isUpdating = false;
let watchers = [];

// Resolve paths dynamically
function resolvePaths() {
    const entries = fs.readdirSync(CONFIG.projectRoot, { withFileTypes: true });
    const ravenEntry = entries.find(e =>
        e.isDirectory() && e.name.toLowerCase().includes('raven')
    );

    if (ravenEntry) {
        CONFIG.ravenDir = path.join(CONFIG.projectRoot, ravenEntry.name);
        CONFIG.docsDir = path.join(CONFIG.projectRoot, 'docs');
        CONFIG.watchDirs = [
            path.join(CONFIG.ravenDir, 'src'),
            path.join(CONFIG.ravenDir, 'src-tauri', 'src'),
            path.join(CONFIG.ravenDir, 'src', 'settings'),
            path.join(CONFIG.ravenDir, 'src', 'stores'),
            CONFIG.docsDir
        ];

        console.log(`[Smart] Project: ${CONFIG.projectRoot}`);
        console.log(`[Smart] Raven: ${CONFIG.ravenDir}`);
        return true;
    }

    console.error('[Smart] Could not find Raven directory');
    return false;
}

// Hash function
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

// Scan files for change detection
function scanFiles() {
    const files = new Map();

    for (const dir of CONFIG.watchDirs) {
        if (!fs.existsSync(dir)) continue;

        const walk = (currentDir) => {
            const entries = fs.readdirSync(currentDir, { withFileTypes: true });
            for (const entry of entries) {
                if (['node_modules', 'dist', 'target', '.git'].includes(entry.name)) continue;

                const fullPath = path.join(currentDir, entry.name);
                if (entry.isDirectory()) {
                    walk(fullPath);
                } else if (/\.(js|jsx|ts|tsx|rs|css|json|md)$/.test(entry.name)) {
                    const stat = fs.statSync(fullPath);
                    const relPath = path.relative(CONFIG.projectRoot, fullPath);
                    files.set(relPath, {
                        path: fullPath,
                        mtime: stat.mtimeMs,
                        hash: hashFile(fullPath)
                    });
                }
            }
        };
        walk(dir);
    }

    return files;
}

// Detect changes
function detectChanges(newFiles, prevFiles) {
    const changes = { added: [], modified: [], removed: [] };

    for (const [filePath, fileData] of newFiles) {
        if (!prevFiles.has(filePath)) {
            changes.added.push(filePath);
        } else {
            const prevData = prevFiles.get(filePath);
            if (prevData && prevData.hash !== fileData.hash) {
                changes.modified.push(filePath);
            }
        }
    }

    for (const [filePath] of prevFiles) {
        if (!newFiles.has(filePath)) {
            changes.removed.push(filePath);
        }
    }

    return changes;
}

// Update documentation
function updateDocumentation(changes) {
    if (isUpdating) return;

    const now = Date.now();
    if (now - lastUpdateTime < CONFIG.minUpdateInterval) return;

    const hasChanges = changes.added.length > 0 || changes.modified.length > 0 || changes.removed.length > 0;
    if (!hasChanges) return;

    isUpdating = true;
    lastUpdateTime = now;

    console.log(`\n[Smart] ${new Date().toLocaleString()} - Changes detected:`);
    if (changes.added.length > 0) console.log(`  + ${changes.added.length} added`);
    if (changes.modified.length > 0) console.log(`  ~ ${changes.modified.length} modified`);
    if (changes.removed.length > 0) console.log(`  - ${changes.removed.length} removed`);

    try {
        updateHistory(changes);
        updateStructure();
        updateBrain(changes);
        updateCLAUDE();
        updateStyle();
        console.log('[Smart] Documentation updated successfully');
    } catch (error) {
        console.error('[Smart] Update error:', error.message);
    }

    isUpdating = false;
}

function updateHistory(changes) {
    const historyPath = path.join(CONFIG.docsDir, 'history.md');
    if (!fs.existsSync(historyPath)) return;

    const date = new Date().toISOString().split('T')[0];
    const time = new Date().toLocaleTimeString();
    const entrySignature = `## ${date} - ${time.split(':')[0]}:${time.split(':')[1]}`;

    let entry = `${entrySignature}\n\n`;

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

    // Prevent duplicate entries
    if (content.includes(entrySignature)) return;

    const lines = content.split('\n');
    let insertIndex = 1;
    for (let i = 0; i < lines.length; i++) {
        if (lines[i].startsWith('#')) {
            insertIndex = i + 1;
            break;
        }
    }

    lines.splice(insertIndex, 0, '', entry);
    fs.writeFileSync(historyPath, lines.join('\n'));
}

function updateStructure() {
    const structurePath = path.join(CONFIG.docsDir, 'structure.md');
    if (!fs.existsSync(structurePath)) return;

    let content = fs.readFileSync(structurePath, 'utf8');
    const updateComment = `<!-- Auto-updated: ${new Date().toISOString()} -->\n`;

    if (!content.startsWith(updateComment)) {
        content = updateComment + content.replace(/<!-- Auto-updated: .* -->\n/, '');
        fs.writeFileSync(structurePath, content);
    }
}

function updateBrain(changes) {
    const brainPath = path.join(CONFIG.docsDir, 'brain.md');
    if (!fs.existsSync(brainPath)) return;

    let content = fs.readFileSync(brainPath, 'utf8');

    if (!content.includes('## Recent Changes')) {
        content += '\n## Recent Changes\n\n';
    }

    const date = new Date().toISOString().split('T')[0];
    let updateEntry = `### ${date}\n`;

    if (changes.added.length > 0) {
        updateEntry += `- Added: ${changes.added.join(', ')}\n`;
    }
    if (changes.modified.length > 0) {
        updateEntry += `- Modified: ${changes.modified.join(', ')}\n`;
    }

    const recentIndex = content.indexOf('## Recent Changes');
    if (recentIndex !== -1 && !content.includes(`### ${date}`)) {
        const insertPos = content.indexOf('\n', recentIndex) + 1;
        content = content.slice(0, insertPos) + updateEntry + '\n' + content.slice(insertPos);
        fs.writeFileSync(brainPath, content);
    }
}

function updateCLAUDE() {
    const claudePath = path.join(CONFIG.docsDir, 'CLAUDE.md');
    if (!fs.existsSync(claudePath)) return;

    let content = fs.readFileSync(claudePath, 'utf8');
    const updateComment = `<!-- Last auto-updated: ${new Date().toISOString()} -->\n`;

    if (!content.startsWith(updateComment)) {
        content = updateComment + content.replace(/<!-- Last auto-updated: .* -->\n/, '');
        fs.writeFileSync(claudePath, content);
    }
}

function updateStyle() {
    const stylePath = path.join(CONFIG.docsDir, 'style.md');
    if (!fs.existsSync(stylePath)) return;

    let content = fs.readFileSync(stylePath, 'utf8');
    const updateComment = `<!-- Last auto-updated: ${new Date().toISOString()} -->\n`;

    if (!content.startsWith(updateComment)) {
        content = updateComment + content.replace(/<!-- Last auto-updated: .* -->\n/, '');
        fs.writeFileSync(stylePath, content);
    }
}

// Trigger update with debounce
function triggerUpdate() {
    if (updateTimeout) clearTimeout(updateTimeout);

    updateTimeout = setTimeout(() => {
        const currentFiles = scanFiles();
        const prevFiles = global.fileTracker || new Map();
        const changes = detectChanges(currentFiles, prevFiles);
        updateDocumentation(changes);
        global.fileTracker = currentFiles;
    }, CONFIG.debounceTime);
}

// Setup native file watchers
function setupWatchers() {
    console.log('[Smart] Setting up native file watchers...');
    console.log('[Smart] Using OS events (not polling) - zero CPU when idle');

    let watcherCount = 0;

    for (const dir of CONFIG.watchDirs) {
        if (!fs.existsSync(dir)) continue;

        try {
            const watcher = fs.watch(dir, { recursive: true }, (eventType, filename) => {
                if (!filename || !/\.(js|jsx|ts|tsx|rs|css|json|md)$/.test(filename)) return;

                console.log(`[Smart] Event: ${eventType} - ${filename}`);
                triggerUpdate();
            });

            watchers.push(watcher);
            watcherCount++;

            watcher.on('error', (err) => {
                console.error(`[Smart] Watcher error for ${dir}:`, err.message);
            });
        } catch (error) {
            console.error(`[Smart] Failed to watch ${dir}:`, error.message);
        }
    }

    console.log(`[Smart] Watching ${watcherCount} directories`);
}

// Cleanup on exit
function cleanup() {
    console.log('[Smart] Cleaning up watchers...');
    for (const watcher of watchers) {
        watcher.close();
    }
    if (updateTimeout) clearTimeout(updateTimeout);
}

process.on('SIGINT', () => {
    cleanup();
    process.exit(0);
});

process.on('SIGTERM', () => {
    cleanup();
    process.exit(0);
});

process.on('exit', cleanup);

// Start the smart watcher
function start() {
    if (!resolvePaths()) {
        process.exit(1);
    }

    console.log('\n========================================');
    console.log('  Raven Smart Documentation Watcher');
    console.log('========================================');
    console.log(`Started: ${new Date().toLocaleString()}`);
    console.log('Mode: Event-driven (wakes only on file changes)');
    console.log('Resource usage: ~15 MB RAM, 0% CPU when idle');
    console.log('\n[Smart] Watching for file changes...\n');

    // Initial scan
    global.fileTracker = scanFiles();
    console.log(`[Smart] Initial scan: ${global.fileTracker.size} files tracked`);

    // Setup native watchers
    setupWatchers();
}

start();
