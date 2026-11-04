/**
 * Starlight renders the frontmatter `title` as the page <h1>, so a leading `# Title`
 * in the Markdown body shows the title twice. Drop the body's first H1 when it is the
 * first content node (HTML comments before it are skipped); later H1s are kept.
 */
export function remarkStripTitleH1() {
  return (tree) => {
    const first = tree.children.findIndex((n) => n.type !== 'html' && n.type !== 'yaml');
    if (first !== -1 && tree.children[first].type === 'heading' && tree.children[first].depth === 1) {
      tree.children.splice(first, 1);
    }
  };
}
