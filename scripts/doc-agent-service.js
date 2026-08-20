#!/usr/bin/env node

/**
 * Raven Documentation Agent - Persistent Service
 *
 * A true file system watcher that runs 24/7 independent of any session.
 * Uses chokidar for efficient native file system events (not polling).
 * Automatically updates documentation when ANY change occurs.
 *
 * Install as Windows Service: npm install -g windows-service
 * Or run via Task Scheduler on startup
 */

const fs = require('fs');
const path = require('path');

// Try to use chokidar for efficient file watching (fallback to polling if not available)
let chokidar;
try {
    chokidar = require('chokidar');
} catch (e) {
    console.log('[Service] Chokidar not available, will use polling fallback');
}

// Configuration
const CONFIG = {
    projectRoot: path.resolve(__dirname, '..'),
    ravenDir: null,
    docsDir: null,
    watchPatterns: [],
    ignorePatterns: ['**/node_modules/**', '**/dist/**', '**/target/**', '**/.git/**'],
    pollInterval: 3000,  // 3 seconds when using polling
    debounceTime: 1000   // Wait 1 second after last change before updating
};

// State
let isRunning = false;
let updateTimeout = null;
let fileTracker = new Map();
let lastUpdateTime = 0;

// Resolve paths dynamically (handles folder renames)
function resolvePaths() {
    const entries = fs.readdirSync(CONFIG.projectRoot, { withFileTypes: true });
    const ravenEntry = entries.find(e =>
        e.isDirectory() && e.name.toLowerCase().includes('raven')
    );

    if (ravenEntry) {
        CONFIG.ravenDir = path.join(CONFIG.projectRoot, ravenEntry.name);
        CONFIG.docsDir = path.join(CONFIG.projectRoot, 'docs');
        CONFIG.watchPatterns = [
            path.join(CONFIG.ravenDir, 'src', '**', '*.{js,jsx,ts,tsx,css,json,md}'),
            path.join(CONFIG.ravenDir, 'src-tauri', 'src', '**', '*.{js,jsx,ts,tsx,rs,css,json,md}'),
            path.join(CONFIG.ravenDir, 'src', 'settings', '**', '*.{js,jsx,ts,tsx,css,json,md}'),
            path.join(CONFIG.ravenDir, 'src', 'stores', '**', '*.{js,jsx,ts,tsx,css,json,md}'),
            path.join(CONFIG.docsDir, '**', '*.md')
        ];

        console.log(`[Service] Project root: ${CONFIG.projectRoot}`);
        console.log(`[Service] Raven dir: ${CONFIG.ravenDir}`);
        console.log(`[Service] Docs dir: ${CONFIG.docsDir}`);
        return true;
    }

    console.error('[Service] Could not find Raven directory');
    return false;
}

// Hash function for change detection
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

// Scan all watched files
function scanFiles() {
    const files = new Map();
    const dirsToScan = [
        path.join(CONFIG.ravenDir, 'src'),
        path.join(CONFIG.ravenDir, 'src-tauri', 'src'),
        path.join(CONFIG.ravenDir, 'src', 'settings'),
        path.join(CONFIG.ravenDir, 'src', 'stores'),
        CONFIG.docsDir
    ];

    for (const dir of dirsToScan) {
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
                        size: stat.size,
                        hash: hashFile(fullPath)
                    });
                }
            }
        };
        walk(dir);
    }

    return files;
}

// Detect changes between snapshots
function detectChanges(newFiles) {
    const changes = { added: [], modified: [], removed: [] };

    for (const [filePath, fileData] of newFiles) {
        if (!fileTracker.has(filePath)) {
            changes.added.push(filePath);
        } else {
            const prevData = fileTracker.get(filePath);
            if (prevData && prevData.hash !== fileData.hash) {
                changes.modified.push(filePath);
            }
        }
    }

    for (const [filePath] of fileTracker) {
        if (!newFiles.has(filePath)) {
            changes.removed.push(filePath);
        }
    }

    fileTracker = newFiles;
    return changes;
}

// Update documentation files
function updateDocumentation(changes) {
    const hasChanges = changes.added.length > 0 || changes.modified.length > 0 || changes.removed.length > 0;
    if (!hasChanges) return;

    const now = Date.now();
    if (now - lastUpdateTime < 5000) {
        // Rate limit: max once per 5 seconds
        return;
    }
    lastUpdateTime = now;

    console.log(`\n[Service] Changes detected at ${new Date().toLocaleString()}:`);
    if (changes.added.length > 0) console.log(`  + ${changes.added.length} added`);
    if (changes.modified.length > 0) console.log(`  ~ ${changes.modified.length} modified`);
    if (changes.removed.length > 0) console.log(`  - ${changes.removed.length} removed`);

    // Update history.md
    updateHistory(changes);

    // Update other docs
    updateStructure();
    updateBrain(changes);
    updateCLAUDE();
    updateStyle();

    console.log('[Service] All documentation files updated');
}

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
    let insertIndex = 1;
    for (let i = 0; i < lines.length; i++) {
        if (lines[i].startsWith('#')) {
            insertIndex = i + 1;
            break;
        }
    }

    // Check if this exact entry already exists (prevent duplicates)
    const entrySignature = `## ${date} - ${time}`;
    if (content.includes(entrySignature)) return;

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
        content += '\n## Recent Changes\n\n<!-- Auto-updated section -->\n';
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

// Debounced update trigger
function triggerUpdate(changes) {
    if (updateTimeout) clearTimeout(updateTimeout);

    updateTimeout = setTimeout(() => {
        updateDocumentation(changes);
        updateTimeout = null;
    }, CONFIG.debounceTime);
}

// Polling-based watcher (works everywhere, no dependencies)
function startPollingWatcher() {
    console.log('[Service] Starting polling-based file watcher...');
    console.log(`[Service] Checking for changes every ${CONFIG.pollInterval}ms`);

    // Initial scan
    fileTracker = scanFiles();
    console.log(`[Service] Tracking ${fileTracker.size} files`);

    setInterval(() => {
        try {
            const currentFiles = scanFiles();
            const changes = detectChanges(currentFiles);
            triggerUpdate(changes);
        } catch (error) {
            console.error('[Service] Error during scan:', error.message);
        }
    }, CONFIG.pollInterval);
}

// Chokidar-based watcher (more efficient, requires chokidar package)
function startChokidarWatcher() {
    console.log('[Service] Starting chokidar-based file watcher...');

    const watcher = chokidar.watch(CONFIG.watchPatterns, {
        ignored: CONFIG.ignorePatterns,
        persistent: true,
        ignoreInitial: false,
        awaitWriteFinish: {
            stabilityThreshold: 1000,
            pollInterval: 100
        }
    });

    watcher
        .on('add', (path) => {
            console.log(`[Service] File added: ${path}`);
            scheduleUpdate();
        })
        .on('change', (path) => {
            console.log(`[Service] File changed: ${path}`);
            scheduleUpdate();
        })
        .on('unlink', (path) => {
            console.log(`[Service] File removed: ${path}`);
            scheduleUpdate();
        });

    function scheduleUpdate() {
        if (updateTimeout) clearTimeout(updateTimeout);
        updateTimeout = setTimeout(() => {
            const currentFiles = scanFiles();
            const changes = detectChanges(currentFiles);
            updateDocumentation(changes);
        }, CONFIG.debounceTime);
    }
}

// Handle graceful shutdown
process.on('SIGINT', () => {
    console.log('\n[Service] Shutting down...');
    isRunning = false;
    if (updateTimeout) clearTimeout(updateTimeout);
    process.exit(0);
});

process.on('SIGTERM', () => {
    console.log('\n[Service] Terminating...');
    isRunning = false;
    if (updateTimeout) clearTimeout(updateTimeout);
    process.exit(0);
});

// Start the service
function start() {
    if (isRunning) {
        console.log('[Service] Already running');
        return;
    }

    if (!resolvePaths()) {
        console.error('[Service] Failed to start - could not resolve paths');
        process.exit(1);
    }

    isRunning = true;
    console.log('\n===========================================');
    console.log('  Raven Documentation Agent - SERVICE MODE');
    console.log('===========================================');
    console.log(`Started at: ${new Date().toLocaleString()}`);
    console.log('Monitoring: All Raven source files + docs');
    console.log('Status: ALWAYS ON - No session required\n');

    // Use chokidar if available, otherwise fall back to polling
    if (chokidar) {
        startChokidarWatcher();
    } else {
        startPollingWatcher();
    }

    console.log('\n[Service] Agent is now running 24/7 in the background');
    console.log('[Service] Press Ctrl+C to stop (or close this window)');
}

// Auto-start
start();
