import os
import re
import shutil

ROOT = r"c:\Users\Owner\vox"

def replace_in_file(path, replacements):
    try:
        with open(path, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception:
        return
    
    new_content = content
    for pattern, repl in replacements:
        new_content = re.sub(pattern, repl, new_content)
        
    if new_content != content:
        with open(path, 'w', encoding='utf-8') as f:
            f.write(new_content)

def main():
    # 1. Delete folders
    for d in ["crates/vox-gamify", "crates/vox-codex"]:
        dp = os.path.join(ROOT, d)
        if os.path.exists(dp):
            shutil.rmtree(dp)
            
    # 2. Delete python scripts
    for f in ["scripts/_patch_qlora_eta.py", "scripts/_patch_qlora_eta2.py"]:
        fp = os.path.join(ROOT, f)
        if os.path.exists(fp):
            os.remove(fp)

    # 3. Mass textual replace
    for root, dirs, files in os.walk(ROOT):
        norm_root = root.replace("\\", "/")
        if "/target" in norm_root or "/.git" in norm_root or "/node_modules" in norm_root:
            continue
        for file in files:
            if file.endswith((".rs", ".toml", ".vox", ".md")):
                path = os.path.join(root, file)
                
                replacements = []
                replacements.append((r'\bvox_gamify\b', 'vox_ludus'))
                replacements.append((r'\bvox-gamify\b', 'vox-ludus'))
                
                replacements.append((r'\bvox_codex::', 'vox_db::'))
                replacements.append((r'use vox_codex;', 'use vox_db;'))
                replacements.append((r'\bvox-codex\b(?!\-api)', 'vox-db'))
                
                if "vox-integration-tests" in norm_root:
                    replacements.append((r'@component fn', 'fn'))
                
                replace_in_file(path, replacements)

if __name__ == "__main__":
    main()
