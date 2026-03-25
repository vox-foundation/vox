import os
import re

docs_dir = r"c:\Users\Owner\vox\docs\src"
# Map of base filenames to their new absolute paths
file_map = {}

# Build the map
for root, _, files in os.walk(docs_dir):
    for file in files:
        if file.endswith('.md'):
            # Some files might have the same name in different folders (like README.md or examples.md),
            # but for our specific vox docs, filenames are generally unique or we care about the newly moved ones.
            # We'll map exact filename -> relative path from docs_dir
            rel_path = os.path.relpath(os.path.join(root, file), docs_dir).replace('\\', '/')
            file_map[file] = rel_path
            
            # If the file had "-ssot" stripped, we want to know it maps to the new name too
            # Wait, the target strings might have `-ssot.md`. So let's also map `-ssot.md` to the new path
            # if we see a file without it. 
            # E.g. `env-vars.md` exists, so map `env-vars-ssot.md` to `reference/env-vars.md`
            if '-' in file:
                ssot_name = file.replace('.md', '-ssot.md')
                file_map[ssot_name] = rel_path

# Add some hardcoded overrides for structural changes
file_map['ref-cli.md'] = 'reference/cli.md'
file_map['cli-reachability.md'] = 'reference/cli.md'
file_map['cli-design-rules.md'] = 'reference/cli.md'
file_map['deployment.md'] = 'how-to/how-to-deploy.md'
file_map['004-codex-arca-turso.md'] = 'adr/004-codex-arca-turso-ssot.md'
file_map['005-socrates-anti-hallucination.md'] = 'adr/005-socrates-anti-hallucination-ssot.md'

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
                
                target_basename = os.path.basename(target_path)
                
                # Check if it was an ssot reference that needs stripping
                if target_basename.endswith('-ssot.md') and target_basename not in file_map:
                    target_basename = target_basename.replace('-ssot.md', '.md')
                
                if target_basename in file_map:
                    # Resolve new relative path from current file to the target file
                    target_abs = os.path.join(docs_dir, os.path.normpath(file_map[target_basename]))
                    new_rel = os.path.relpath(target_abs, root).replace('\\', '/')
                    return f"[{text}]({new_rel}{anchor})"
                
                return match.group(0)
                
            new_content = link_pattern.sub(replace_link, content)
            
            if new_content != original_content:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(new_content)
                updates += 1

print(f"Updated links in {updates} files.")
