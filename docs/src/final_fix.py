import os
import re

docs_dir = r"c:\Users\Owner\vox\docs\src"
repo_root = r"c:\Users\Owner\vox"

# The issue in validate_links might be os.path.exists vs actual paths.
# Let's write fixes explicitly for the known broken paths.

replacements = {
    "api/vox-cli.md": "../api/vox-cli.md",
    "../../agents/llm-documentation-playbook.md": "../../docs/agents/llm-documentation-playbook.md",
    "how-to/efficient-mode.md": "efficient-mode.md",
    "how-to/how-to-publish-populi-hf.md": "how-to-publish-populi-hf.md",
    "reference/compiler-internals.md": "compiler-internals.md",
}

def fix_content(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
        
    orig = content
    
    # Fix how-to/examples.md etc which have incorrect depths to root scripts
    content = content.replace("../../../scripts/", "../../scripts/")
    content = content.replace("../../agents/", "../../docs/agents/")
    content = content.replace("../../../examples/", "../../examples/")
    content = content.replace("../../../.cursor/", "../../.cursor/")
    
    # Fix crates depth from api/ and architecture/ (which are 2 levels deep: docs/src/api)
    content = content.replace("../../../crates/", "../../crates/")
    
    # Ad-hoc replaces
    for old, new in replacements.items():
        content = re.sub(r'\]\(' + re.escape(old) + r'\)', f']({new})', content)
        
    if orig != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)

for root, _, files in os.walk(docs_dir):
    for file in files:
        if file.endswith('.md'):
            fix_content(os.path.join(root, file))

# Run the validation natively here and print to stdout
link_pattern = re.compile(r'\[([^\]]+)\]\(([^)]+)\)')
broken = 0
for root, _, files in os.walk(docs_dir):
    for file in files:
        if file.endswith('.md'):
            filepath = os.path.join(root, file)
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            for match in link_pattern.finditer(content):
                target = match.group(2)
                if target.startswith('http') or target.startswith('#'):
                    continue
                target_path = target.split('#')[0]
                if not target_path:
                    continue
                resolved = os.path.normpath(os.path.join(root, target_path))
                if not os.path.exists(resolved):
                    broken += 1
                    print(f"Broken: {os.path.relpath(filepath, docs_dir)} -> {target}")

if broken == 0:
    print("All internal links are valid!")
else:
    print(f"{broken} broken links remain.")
