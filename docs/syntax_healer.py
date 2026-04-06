import os
import re

doc_dir = r"docs\src"
issues = []

# Regex replacement pairs
patterns = [
    # 1. @component -> @island
    (r'@component\b', '@island'),
    # 2. @actor fn -> actor
    (r'@actor\s+fn\s+(\w+)\s*\([^)]*\)\s*(?:to\s+\w+(?:\[.*?\])?)?\s*:', r'actor \1 {'),
    # 3. @activity fn -> activity
    (r'@activity\s+fn', 'activity'),
    # 4. @workflow fn -> workflow
    (r'@workflow\s+fn', 'workflow'),
    # 5. routes: -> routes {
    (r'routes:', 'routes {'),
    # 6. match ...: -> match ... {
    (r'(match\s+[^:]+):', r'\1 {'),
    # 7. fn func_name(...): -> fn func_name(...) {
    (r'(fn\s+\w+\(.*\)\s+(?:to\s+\w+(?:\[.*?\])?\s*)?):', r'\1 {'),
    # 8. @table type Name: -> @table type Name {
    (r'(@table\s+type\s+\w+):', r'\1 {'),
    # 9. | Ok => -> Ok ->
    (r'\|\s*(\w+(?:\([^)]*\))?)\s*=>', r'\1 ->')
]

for root, _, files in os.walk(doc_dir):
    for filename in files:
        if filename.endswith(".md"):
            filepath = os.path.join(root, filename)
            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()
            
            orig_content = content
            for pattern, replacement in patterns:
                content = re.sub(pattern, replacement, content)
            
            if content != orig_content:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(content)
                issues.append(filepath.replace("\\", "/"))

print("Fixed syntax in files:")
for i in issues:
    print(f" - {i}")
