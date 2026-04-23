; vox/  inject into fenced code blocks inside Markdown docs
((fenced_code_block_delimiter) @_lang
 (fenced_code_block_content) @injection.content
 (#match? @_lang "^vox$")
 (#set! injection.language "vox"))

((fenced_code_block_delimiter) @_lang
 (fenced_code_block_content) @injection.content
 (#match? @_lang "^tsx$")
 (#set! injection.language "vox"))
