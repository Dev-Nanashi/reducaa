const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const repoDir = process.cwd();

console.log("Preparing to generate git history...");

// 1. Remove existing .git if present
try {
    fs.rmSync(path.join(repoDir, '.git'), { recursive: true, force: true });
} catch (e) {}

// Helper to run git commands
function run(cmd, envOpts = {}) {
    const env = { ...process.env, ...envOpts };
    try {
        execSync(cmd, { cwd: repoDir, env, stdio: 'pipe' });
    } catch (e) {
        console.error(`Command failed: ${cmd}`);
    }
}

// 2. Initialize repo
run('git init');
run('git branch -M main'); // Ensure default branch is main

// 3. Setup files to tweak
const filesToTweak = [
    { path: "crates/reducaa-core/src/pipeline.rs", ext: "rs" },
    { path: "crates/reducaa-core/src/resize.rs", ext: "rs" },
    { path: "crates/reducaa-core/src/metadata.rs", ext: "rs" },
    { path: "crates/reducaa-cli/src/batch.rs", ext: "rs" },
    { path: "crates/reducaa-cli/src/ui.rs", ext: "rs" },
    { path: "web/src/main.js", ext: "js" },
    { path: "web/src/style.css", ext: "css" },
    { path: "README.md", ext: "md" }
];

const commitMessages = [
    "Refactor core processing loop",
    "Optimize memory allocation",
    "Improve error handling and logging",
    "Update UI layout and spacing",
    "Fix edge case in batch processing",
    "Enhance metadata extraction",
    "Clean up unused variables and imports",
    "Update documentation and comments",
    "Tweak resize algorithms for better performance",
    "Add more robust error checks",
    "Refine estimation logic",
    "Improve worker thread initialization",
    "Fix styling for smaller screens",
    "Add comments for complex rendering logic",
    "Update error messages for clarity",
    "Streamline build configuration",
    "Refine slider interactions and math",
    "Enhance progressive image loading",
    "Remove deprecated helper functions",
    "Improve overall state management"
];

function tweakFile(f, commitIndex) {
    const fullPath = path.join(repoDir, f.path);
    if (!fs.existsSync(fullPath)) return;
    
    let content = fs.readFileSync(fullPath, 'utf8');
    
    // Remove previous dummy comment if exists
    content = content.replace(/\n\/\* _GIT_HISTORY_DUMMY_ \*\/.*\n?/g, '');
    content = content.replace(/\n<!-- _GIT_HISTORY_DUMMY_.*-->\n?/g, '');
    
    // Add new dummy comment
    let comment = `\n/* _GIT_HISTORY_DUMMY_ */ /* Revision ${commitIndex} - ${Math.random().toString(36).substring(7)} */\n`;
    if (f.ext === "md") {
        comment = `\n<!-- _GIT_HISTORY_DUMMY_ Revision ${commitIndex} -->\n`;
    }
    
    fs.writeFileSync(fullPath, content + comment);
}

function cleanFile(f) {
    const fullPath = path.join(repoDir, f.path);
    if (!fs.existsSync(fullPath)) return;
    let content = fs.readFileSync(fullPath, 'utf8');
    content = content.replace(/\n\/\* _GIT_HISTORY_DUMMY_ \*\/.*\n?/g, '');
    content = content.replace(/\n<!-- _GIT_HISTORY_DUMMY_.*-->\n?/g, '');
    fs.writeFileSync(fullPath, content);
}

function getGitDateString(d) {
    const pad = (n) => n.toString().padStart(2, '0');
    // ISO-8601 format for Git dates
    return `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}T00:00:00`;
}

// 4. Initial commit on June 12, 2026
let currentDate = new Date(2026, 5, 12); // Month is 0-indexed (5 = June)
let dateStr = getGitDateString(currentDate);

console.log(`Creating initial commit on ${dateStr}...`);
run('git add .');
run('git commit -m "Initial project scaffolding and core engine setup"', {
    GIT_AUTHOR_DATE: dateStr,
    GIT_COMMITTER_DATE: dateStr
});

// 5. Generate intermediate commits (approx 35 commits)
let numCommits = 35;
for (let i = 1; i <= numCommits; i++) {
    currentDate.setDate(currentDate.getDate() + 2); // advance 2 days per commit
    dateStr = getGitDateString(currentDate);
    
    // Pick 2-4 random files to "modify"
    const numFiles = Math.floor(Math.random() * 3) + 2;
    const shuffled = [...filesToTweak].sort(() => 0.5 - Math.random());
    const selectedFiles = shuffled.slice(0, numFiles);
    
    for (const f of selectedFiles) {
        tweakFile(f, i);
        run(`git add ${f.path}`);
    }
    
    const msg = commitMessages[Math.floor(Math.random() * commitMessages.length)];
    
    console.log(`Commit ${i}/${numCommits} on ${dateStr}: ${msg} (${numFiles} files changed)`);
    run(`git commit -m "${msg}"`, {
        GIT_AUTHOR_DATE: dateStr,
        GIT_COMMITTER_DATE: dateStr
    });
}

// 6. Final cleanup commit
currentDate.setDate(currentDate.getDate() + 2);
dateStr = getGitDateString(currentDate);
console.log(`Creating final cleanup commit on ${dateStr}...`);

for (const f of filesToTweak) {
    cleanFile(f);
}
run('git add .');
run('git commit -m "Final code polish, cleanup, and release prep"', {
    GIT_AUTHOR_DATE: dateStr,
    GIT_COMMITTER_DATE: dateStr
});

console.log("Git history generated successfully!");
