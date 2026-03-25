import os
import re

docs_dir = r"c:\Users\Owner\vox\docs\src"
repo_root = r"c:\Users\Owner\vox"
link_pattern = re.compile(r'\[([^\]]+)\]\(([^)]+)\)')

updates = 0

for root, _, files in os.walk(docs_dir):
    for file in files:
        if file.endswith('.md'):
            filepath = os.path.join(root, file)
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            
            original_content = content
            
            def replace_link(match):
                text = match.group(1)
                target = match.group(2)
                
                if target.startswith('http') or target.startswith('#'):
                    return match.group(0)
                
                parts = target.split('#', 1)
                target_path = parts[0]
                anchor = '#' + parts[1] if len(parts) > 1 else ''
                
                if not target_path:
                    return match.group(0)
                
                resolved_target_path = os.path.normpath(os.path.join(root, target_path))
                
                # Check if it was trying to go out of docs/src but failed by 1 directory level
                if not os.path.exists(resolved_target_path):
                    # Try going one level higher, representing moving from root to inside a dir like 'reference'
                    test_path_up = os.path.normpath(os.path.join(root, "..", target_path))
                    if os.path.exists(test_path_up):
                        new_rel = os.path.relpath(test_path_up, root).replace('\\', '/')
                        return f"[{text}]({new_rel}{anchor})"
                    
                    # Also try checking if the target path is just the filename of an existing doc
                    target_basename = os.path.basename(target_path)
                    # Let's find it in docs if we can
                    for r, _, fs in os.walk(docs_dir):
                        if target_basename in fs:
                            found_abs = os.path.join(r, target_basename)
                            new_rel = os.path.relpath(found_abs, root).replace('\\', '/')
                            return f"[{text}]({new_rel}{anchor})"
                
                return match.group(0)
                
            new_content = link_pattern.sub(replace_link, content)
            
            if new_content != original_content:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(new_content)
                updates += 1

print(f"Updated links in {updates} files.")
