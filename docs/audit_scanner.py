import os
import re

doc_dir = r"docs\src"
issues = []

# Regex patterns for V0.2 syntax
patterns = [
    (r'@component\b', "Contains removed @component decorator"),
    (r'@actor\s+fn', "Contains old @actor fn decorator"),
    (r'@activity\s+fn', "Contains old @activity fn decorator"),
    (r'@workflow\s+fn', "Contains old @workflow fn decorator"),
    (r'fn\s+\w+\(.*\)\s+(?:to\s+\w+(?:\[.*?\])?\s*)?:', "Uses colon instead of brace for function block"),
    (r'@table\s+type\s+\w+:', "Uses colon instead of brace for table declaration"),
    (r'routes:', "Uses colon instead of brace for routes block"),
    (r'match\s+.*:', "Uses colon for match block"),
]

for root, _, files in os.walk(doc_dir):
    for filename in files:
        if filename.endswith(".md"):
            filepath = os.path.join(root, filename)
            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()
                
            file_issues = []
            
            # Check for systemic syntax errors
            for pattern, msg in patterns:
                matches = re.finditer(pattern, content)
                count = sum(1 for _ in matches)
                if count > 0:
                    file_issues.append(f"- [Syntax Rot] {count} instances of: {msg}")
                    
            # Check for {{#include}} in api/example_*.md
            if "api" in root and filename.startswith("example_"):
                if "{{#include" not in content:
                    file_issues.append("- [Maintenance] Example file does not use {{#include}} directive; prone to drift.")

            # Record issues
            if file_issues:
                issues.append({"file": filepath.replace("\\", "/"), "issues": file_issues})

# Output findings
for item in issues:
    print(f"File: {item['file']}")
    for issue in item['issues']:
        print(f"  {issue}")

