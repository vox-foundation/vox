import os

docs_dir = r"c:\Users\Owner\vox\docs\src"
summary_path = os.path.join(docs_dir, "SUMMARY.md")

sections = {
    "Getting Started": [],
    "Tutorials": [],
    "How-To Guides": [],
    "Explanations": [],
    "Reference": [],
    "Archived / ADRs": []
}

def extract_title(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        for line in f:
            if line.startswith('title:'):
                return line.split('title:')[1].strip().strip('"').strip("'")
            if line.startswith('# '):
                return line[2:].strip()
    return os.path.basename(filepath)

def process_dir(directory, section_name):
    target_dir = os.path.join(docs_dir, directory)
    if not os.path.exists(target_dir): return
    for root, _, files in os.walk(target_dir):
        for file in files:
            if file.endswith('.md'):
                rel_path = os.path.relpath(os.path.join(root, file), docs_dir).replace('\\', '/')
                # Skip index and SUMMARY
                if rel_path in ['index.md', 'SUMMARY.md']: continue
                title = extract_title(os.path.join(root, file))
                sections[section_name].append((title, rel_path))

# Root level
sections["Getting Started"].append(("Vox: The AI-Native Programming Language", "index.md"))

# Process directories
process_dir("tutorials", "Tutorials")
process_dir("how-to", "How-To Guides")
process_dir("explanation", "Explanations")
process_dir("reference", "Reference")
process_dir("api", "Reference") # Put API under reference
process_dir("adr", "Archived / ADRs")

# Write out the SUMMARY.md
with open(summary_path, 'w', encoding='utf-8') as f:
    f.write("# Summary\n\n")
    for section_name, items in sections.items():
        if items:
            f.write(f"\n# {section_name}\n\n")
            # Sort items alphabetically by title
            items.sort(key=lambda x: x[0].lower())
            for title, rel_path in items:
                f.write(f"- [{title}]({rel_path})\n")

print("Generated SUMMARY.md successfully.")
