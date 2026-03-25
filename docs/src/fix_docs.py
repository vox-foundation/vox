import os

docs_dir = r"c:\Users\Owner\vox\docs\src"
reference_dir = os.path.join(docs_dir, "reference")
api_dir = os.path.join(docs_dir, "api")

# Ensure directories exist
os.makedirs(os.path.join(docs_dir, "tutorials"), exist_ok=True)
os.makedirs(os.path.join(docs_dir, "how-to"), exist_ok=True)
os.makedirs(os.path.join(docs_dir, "explanation"), exist_ok=True)
os.makedirs(os.path.join(docs_dir, "reference"), exist_ok=True)

# Merge CLI Docs
cli_docs = [
    os.path.join(reference_dir, "ref-cli.md"),
    os.path.join(api_dir, "vox-cli.md"),
    os.path.join(reference_dir, "cli-design-rules.md"),
    os.path.join(reference_dir, "cli-reachability.md")
]

merged_content = ""
for doc in cli_docs:
    if os.path.exists(doc):
        with open(doc, 'r', encoding='utf-8') as f:
            merged_content += f"\n\n<!-- Merged from {os.path.basename(doc)} -->\n\n"
            merged_content += f.read()

cli_md_path = os.path.join(reference_dir, "cli.md")
if merged_content:
    with open(cli_md_path, 'w', encoding='utf-8') as f:
        f.write(merged_content)

for doc in cli_docs:
    if os.path.exists(doc):
        os.remove(doc)

# Link fixing and Frontmatter updating across all markdown files
for root, _, files in os.walk(docs_dir):
    for file in files:
        if file.endswith('.md'):
            filepath = os.path.join(root, file)
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            
            original_content = content
            
            # Fix -ssot.md suffix
            content = content.replace('-ssot.md', '.md')
            
            # Fix pathing
            content = content.replace('architecture/', 'reference/')
            content = content.replace('../architecture/', '../reference/')
            content = content.replace('ci/command-compliance.md', 'reference/command-compliance.md')
            content = content.replace('../ref/', '../reference/')
            content = content.replace('/ref/', '/reference/')
            
            # Since some files moved from root
            content = content.replace('(faq.md)', '(explanation/faq.md)')
            content = content.replace('(glossary.md)', '(explanation/glossary.md)')
            content = content.replace('(examples.md)', '(how-to/examples.md)')
            content = content.replace('(changelog.md)', '(reference/changelog.md)')
            content = content.replace('(mcp_serverless_research.md)', '(explanation/mcp_serverless_research.md)')
            content = content.replace('(zig-inspired-deployment.md)', '(explanation/zig-inspired-deployment.md)')
            
            # Fix tut/expl/how-to missing paths
            content = content.replace('(tut-', '(tutorials/tut-')
            content = content.replace('(expl-', '(explanation/expl-')
            
            # Hardcode specific fix for how-to to avoid transforming already fixed links (e.g. how-to-deploy vs how-to/how-to-deploy)
            import re
            content = re.sub(r'\((how-to-[^/]+\.md)\)', r'(how-to/\1)', content)
            
            content = content.replace('(ref-cli.md)', '(reference/cli.md)')
            content = content.replace('(reference/ref-cli.md)', '(reference/cli.md)')
            
            # Update Frontmatter
            if not content.startswith('---'):
                category = "reference"
                if "tutorials" in root: category = "tutorials"
                elif "how-to" in root: category = "how-to"
                elif "explanation" in root: category = "explanation"
                
                training = "true"
                if "adr" in root: training = "false"
                elif "api" in root: training = "true"
                
                frontmatter = f"---\ntitle: \"{file.replace('.md', '')}\"\ndescription: \"Documentation for {file}\"\ncategory: \"{category}\"\nlast_updated: 2026-03-24\ntraining_eligible: {training}\n---\n\n"
                content = frontmatter + content
                
            if content != original_content:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(content)

print("Link fixing and frontmatter generation complete.")
